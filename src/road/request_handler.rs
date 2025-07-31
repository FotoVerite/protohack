use anyhow::{self, bail};
use futures::{StreamExt, stream::SplitStream};
use tokio::{net::TcpStream, sync::mpsc::Sender};
use tokio_util::codec::Framed;

use crate::road::{
    Plates, RoadDispatchers,
    camera::Camera,
    codec::{Codec, ReqValue, RespValue},
    heartbeat::spawn_heartbeat_task,
};

enum IAm {
    Camera(Camera),
    Dispatcher(Vec<u16>),
    None,
}

pub async fn handle_request(
    mut reader: SplitStream<Framed<TcpStream, Codec>>,
    dispatchers: &mut RoadDispatchers,
    plate_storage: Plates,
    tx: Sender<RespValue>,
) -> anyhow::Result<()> {
    let mut i_am = IAm::None;
    let mut heartbeat: Option<u32> = None;
    while let Some(result) = reader.next().await {
        match result {
            Ok(data) => {
                let resp = match data {
                    ReqValue::WantHeartbeat(interval) => {
                        set_heartbeat(&mut heartbeat, interval, tx.clone())
                    }
                    ReqValue::IAmCamera(road, location, limit) => {
                        set_camera(&mut i_am, road, location, limit)
                    }
                    ReqValue::IAmDispatcher(roads) => {
                        set_dispatcher(&mut i_am, roads, tx.clone(), dispatchers).await
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
                if let IAm::Dispatcher(roads) = &i_am {
                    let mut guard = dispatchers.lock().await;
                    for road in roads {
                        guard.remove(&road);
                    }
                }
            }
            Err(e) => {
                let msg = RespValue::Error(e.to_string());
                tx.send(msg).await?;
            }
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
    roads: Vec<u16>,
    tx: Sender<RespValue>,
    dispatchers: &mut RoadDispatchers,
) -> anyhow::Result<()> {
    if !matches!(i_am, IAm::None) {
        anyhow::bail!("Road is already being watched");
    }
    let mut guard = dispatchers.lock().await;
    *i_am = IAm::Dispatcher(roads.clone());
    for road in roads {
        if guard.contains_key(&road) {
            anyhow::bail!("Road is already being watched")
        }
        guard.insert(road, tx.clone());
    }

    Ok(())
}
fn set_heartbeat(
    heartbeat: &mut Option<u32>,
    interval: u32,
    tx: Sender<RespValue>,
) -> anyhow::Result<()> {
    if heartbeat.is_some() {
        bail!("Heartbeat cannot be set again")
    }
    *heartbeat = Some(interval);
    spawn_heartbeat_task(interval, tx);
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
            let ticket = guard.update_plate(&plate, timestamp, camera)?;
            if let Some(ticket) = ticket {
                let dispatchers_guard = dispatchers.lock().await;
                if let Some(rx) = dispatchers_guard.get(&camera.road) {
                    let resp = RespValue::Ticket(ticket);
                    _ = rx.send(resp).await?;
                }
            }
            Ok(())
        }
        _ => bail!("Client is not a Camera"),
    }
}
