use std::{collections::{HashMap}, sync::Arc};

use tokio::sync::{mpsc::Sender, Mutex};

use crate::road::{codec::RespValue, plate::PlateStorage};

pub mod handle_road;
pub mod camera;
pub mod plate;
pub mod request_handler;
pub mod response_handler;
pub mod codec;
pub mod ticket;
pub mod heartbeat;

//type FramedType = Framed<TcpListener>;
type RoadDispatchers = Arc<Mutex<HashMap<u16, Sender<RespValue>>>>;
type Plates = Arc<Mutex<PlateStorage>>;