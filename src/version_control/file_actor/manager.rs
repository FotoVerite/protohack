use std::{collections::HashMap, fs::File, hash::Hash};

use tokio::sync::{mpsc::{Receiver, Sender}, oneshot};

use crate::version_control::{codec::codec::{RespValue, VersionControlCommand}, file_actor::dir::Dir};

pub struct FileManager {
    pub dirs: Dir,
    pub rx: Receiver<(VersionControlCommand, oneshot::Sender<RespValue>)>
}

impl FileManager {
    pub fn new(rx: Receiver<(VersionControlCommand, oneshot::Sender<RespValue>)>) -> Self {
        Self {dirs: Dir::new(), rx}
    }
}