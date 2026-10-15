use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use visper::Library;

#[derive(Clone)]
pub struct Store {
    path: PathBuf,
    can_save: bool,
}

impl Store {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            can_save: false,
        }
    }

    pub fn load(&mut self) -> Library {
        match fs::read_to_string(&self.path) {
            Ok(json) => match Library::from_json(&json) {
                Ok(library) => {
                    self.can_save = true;
                    library
                }
                Err(_) => {
                    self.can_save = false;
                    Library::new()
                }
            },
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                self.can_save = true;
                Library::new()
            }
            Err(_) => {
                self.can_save = false;
                Library::new()
            }
        }
    }

    pub fn save(&self, library: &Library) {
        if !self.can_save {
            return;
        }
        let Some(parent) = self.path.parent() else {
            return;
        };
        if fs::create_dir_all(parent).is_err() {
            return;
        }
        let Ok(json) = library.to_json() else {
            return;
        };
        let temporary = self.path.with_extension("json.tmp");
        if fs::write(&temporary, json).is_ok() {
            let _ = fs::rename(&temporary, &self.path);
        }
    }
}

pub fn library_path(support: &Path) -> PathBuf {
    support.join("library.json")
}
