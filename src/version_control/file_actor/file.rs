use std::collections::HashMap;

#[derive(Default)]
pub struct File {
    versions: HashMap<Vec<u8>, usize>,
    version_lookup: HashMap<usize, Vec<u8>>,
    version: usize,
}

impl File {
    pub fn new() -> Self {
        Self {
            version: 0,
            versions: HashMap::new(),
            version_lookup: HashMap::new(),
        }
    }
    pub fn put(&mut self, data: &str) -> usize {
        let hash: String = sha256::digest(data.as_bytes());
        let entry = self
            .versions
            .entry(hash.as_bytes().to_vec())
            .or_insert_with(|| {
                self.version += 1;
                self.version_lookup
                    .insert(self.version, data.as_bytes().to_vec());
                self.version
            });

        *entry
    }

    pub fn get(&mut self, revision: Option<usize>) -> Vec<u8> {
        match revision {
            None => {
                self.version_lookup.get(&self.version).unwrap().clone()
            }
            Some(number) => {
               self.version_lookup.get(&number).unwrap().clone()
            }
        }
    }
}
