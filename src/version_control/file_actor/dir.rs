use std::{collections::HashMap, path::PathBuf};

use crate::version_control::file_actor::file::File;

#[derive(Default)]
pub struct Dir {
    folders: HashMap<PathBuf, HashMap<String, File>>,
}

impl Dir {
    pub fn new() -> Self {
        Self {
            folders: HashMap::new(),
        }
    }
    pub fn find_or_create(&mut self, path: PathBuf, file_name: String) -> &mut File {
        let dir = self.folders.entry(path).or_insert_with(|| HashMap::new());

        let file = dir.entry(file_name).or_insert_with(|| 
            File::new());
        file
    }

    pub fn get(&mut self, dir: PathBuf) -> usize {
        self.folders.get(&dir).iter().count()
    }
}
