use std::path::Path;

use anyhow::Ok;
use tokio::sync::oneshot::Sender;
use tracing::{info, warn};

use crate::version_control::{
    codec::codec::{RespValue, VersionControlCommand},
    file_actor::manager::FileManager,
};

impl FileManager {
    pub async fn file_actor(&mut self) -> anyhow::Result<()> {
        while let Some((command, tx)) = self.rx.recv().await {
            info!("WORKING ON {:?}", command);
            let resp = match command {
                VersionControlCommand::Help => {
                    RespValue::Text("OK usage: HELP|GET|PUT|LIST".to_string())
                }

                VersionControlCommand::List(dir) => {
                    match dir {
                        Some(dir) => { 
                            let absolute_path = Path::new(&dir);
                            let dir = absolute_path.parent().unwrap();
                            let count = self.dirs.get(dir.to_path_buf());
                            RespValue::Text(format!("OK {}", count).to_string())
                        },
                        None => {
                            RespValue::Text("ERR usage: LIST dir".to_string())
                        }
                    }
                }

                VersionControlCommand::Get(filename) => {
                    match filename {
                        Some(filename) => {RespValue::Text("Fix".to_string())}
                        None => {
                            RespValue::Text("ERR usage: GET file [revision]".to_string())
                        }
                    }
                }

                 VersionControlCommand::Put(args) => {
                    match args {
                        Some((path, data)) => {
                            let absolute_path = Path::new(&path);
                            let dir = absolute_path.parent().unwrap();
                            let file_name = absolute_path.file_name().unwrap().to_str().unwrap();
                            let file = self.dirs.find_or_create(dir.to_path_buf(), file_name.to_string());
                            let revision = file.put(&data);
                            RespValue::Text(format!("OK r{}", revision).to_string())
                        }
                        None => {
                            RespValue::Text("ERR usage: PUT file [revision]".to_string())
                        }
                    }
                }
                
            };
            respond(tx, resp);
        }
      Ok(())

    }
}

fn respond<T>(tx: Sender<T>, val: T)
where
    T: std::fmt::Debug,
{
    if let Err(e) = tx.send(val) {
        tracing::warn!("Failed to send response: {:?}", e);
    }
}
