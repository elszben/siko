use crate::error::Error;
use crate::location_info::filepath::FilePath;
use std::collections::BTreeMap;
use std::fs;

pub struct FileManager {
    pub files: BTreeMap<FilePath, String>,
}

impl FileManager {
    pub fn new() -> FileManager {
        FileManager {
            files: BTreeMap::new(),
        }
    }

    pub fn content(&self, file_path: &FilePath) -> &str {
        self.files.get(file_path).expect("file content not found")
    }

    pub fn read(&mut self, file_path: FilePath) -> Result<(), Error> {
        let content = fs::read(&file_path.path)?;
        let content = String::from_utf8_lossy(&content).to_string();
        self.files.insert(file_path, content);
        Ok(())
    }
}
