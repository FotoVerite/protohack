
use std::net::SocketAddr;

use anyhow::{self, bail};
use futures::{StreamExt, stream::SplitStream};
use tokio::{net::TcpStream, sync::mpsc::Sender};
use tokio_util::{codec::Framed, sync::CancellationToken};

use crate::road::{
    Plates, RoadDispatchers,
    camera::Camera,
    codec::{Codec, ReqValue, RespValue},
    heartbeat::spawn_heartbeat_task,
    road_dispatcher::RoadDispatcher,
};

enum IAm {
    Camera(Camera),
    Dispatcher(Vec<u16>),
    None,
}

pub async fn handle_request(
    mut reader: SplitStream<Framed<TcpStream, Codec>>,
    peer_address: SocketAddr,
    dispatchers: &mut RoadDispatchers,
    plate_storage: Plates,
    tx: Sender<RespValue>,
) -> anyhow::Result<()> {
    let mut i_am = IAm::None;
    let mut heartbeat: Option<u32> = None;
    let cancel_token = CancellationToken::new();
    while let Some(result) = reader.next().await {
        match result {
            Ok(data) => {
                println!("received request: {:?}", data);
                let resp = match data {
                    ReqValue::WantHeartbeat(interval) => set_heartbeat(
                        &mut heartbeat,
                        interval,
                        tx.clone(),
                        cancel_token.child_token(),
                    ),
                    ReqValue::IAmCamera(road, location, limit) => {
                        set_camera(&mut i_am, road, location, limit)
                    }
                    ReqValue::IAmDispatcher(roads) => {
                        set_dispatcher(&mut i_am, peer_address,
                            roads, tx.clone(), dispatchers).await
                    }
                    ReqValue::Plate(plate, time) => {
                        update_plate(&i_am, &plate_storage, &dispatchers, plate, time).await
                    }
                    _ => Ok(()),
                };
                if resp.is_err() {
                    let msg = RespValue::Error(resp.unwrap_err().to_string());
                    tx.send(msg).await?;
                }
            }
            Err(e) => {
                let msg = RespValue::Error(e.to_string());
                tx.send(msg).await?;
            }
        }
    }
    cancel_token.cancel();
    if let IAm::Dispatcher(roads) = &i_am {
        let mut guard = dispatchers.lock().await;
        for road in roads {
            guard.remove(&road);
        }
    }
    Ok(())
}

fn set_camera(i_am: &mut IAm, road: u16, location: u16, limit: u16) -> anyhow::Result<()> {
    if !matches!(i_am, IAm::None) {
        anyhow::bail!("Client already Defined");
    }
    *i_am = IAm::Camera(Camera::new(road, location, limit));
    Ok(())
}
async fn set_dispatcher(
    i_am: &mut IAm,
    peer_address: SocketAddr,
    roads: Vec<u16>,
    tx: Sender<RespValue>,
    dispatchers: &mut RoadDispatchers,
) -> anyhow::Result<()> {
    if !matches!(i_am, IAm::None) {
        anyhow::bail!("Road is already being watched");
    }
    let mut guard = dispatchers.lock().await;
    *i_am = IAm::Dispatcher(roads.clone());
    // Check for conflicts first
    for road in roads {
        let entry = guard.entry(road).or_insert_with(RoadDispatcher::new);
        entry.add_sender(peer_address, tx.clone()).await;
    }
    Ok(())
}

fn set_heartbeat(
    heartbeat: &mut Option<u32>,
    interval: u32,
    tx: Sender<RespValue>,
    child_token: CancellationToken,
) -> anyhow::Result<()> {
    if heartbeat.is_some() {
        bail!("Heartbeat cannot be set again")
    }
    *heartbeat = Some(interval);
    spawn_heartbeat_task(interval, tx, child_token);
    Ok(())
}

async fn update_plate(
    i_am: &IAm,
    plate_storage: &Plates,
    dispatchers: &RoadDispatchers,
    plate: String,
    timestamp: u32,
) -> anyhow::Result<()> {
    match i_am {
        IAm::Camera(camera) => {
            let mut guard = plate_storage.lock().await;
            let tickets = guard.update_plate(&plate, timestamp, camera)?;
            for ticket in tickets {
                let mut dispatchers_guard = dispatchers.lock().await;
                let entry = dispatchers_guard
                    .entry(camera.road)
                    .or_insert(RoadDispatcher::new());
                let resp = RespValue::Ticket(ticket);
                entry.add_ticket(resp).await;
            }
            Ok(())
        }
        _ => bail!("Client is not a Camera"),
    }
}
