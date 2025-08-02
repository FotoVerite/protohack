use std::{collections::{HashMap}, sync::Arc};

use tokio::sync::{Mutex};

use crate::road::{plate::PlateStorage, road_dispatcher::RoadDispatcher};

pub mod handle_road;
pub mod camera;
pub mod plate;
pub mod request_handler;
pub mod response_handler;
pub mod codec;
pub mod ticket;
pub mod heartbeat;
pub mod road_dispatcher;

//type FramedType = Framed<TcpListener>;
type RoadDispatchers = Arc<Mutex<HashMap<u16, RoadDispatcher>>>;
type Plates = Arc<Mutex<PlateStorage>>;