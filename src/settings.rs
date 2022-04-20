use std::path::PathBuf;

use config::Config;
use serde::Deserialize;

#[derive(Deserialize,Debug)]
pub struct Settings {
    mtx_files: Vec<PathBuf>,
}

impl Settings {
    pub fn new(config: PathBuf) -> Self {
        let settings = Config::builder()
            .add_source(config::File::with_name(config.to_str().unwrap()))
            .build()
            .unwrap();
        settings.try_deserialize().unwrap()
    }
    pub fn mtx_files(&self) -> &Vec<PathBuf> {
        &self.mtx_files
    }
}
