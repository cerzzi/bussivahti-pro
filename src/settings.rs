use config::{Config, ConfigError, File};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub api_key: String,
    pub update_interval: u64,
    pub stops: HashMap<String, Vec<String>>,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let s = Config::builder()
            // Etsii tiedostoa nimeltä "Settings" projektin juuresta
            .add_source(File::with_name("Settings"))
            .build()?;

        s.try_deserialize()
    }
}