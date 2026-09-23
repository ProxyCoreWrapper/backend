use crate::consts;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

#[derive(Deserialize, Serialize)]
pub struct Server {
    pub uds_path: PathBuf,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            uds_path: PathBuf::from("/run/PCW/backend/socket"),
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct Config {
    // Core tag to startup command
    pub cores: BTreeMap<String, Vec<String>>,
    pub server: Server,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cores: BTreeMap::from([
                (
                    "sing-box".to_string(),
                    vec!["sing-box", "run", "-c", "%c"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                ), // Config path
                (
                    "xray-core".to_string(),
                    vec!["xray", "run"].iter().map(|s| s.to_string()).collect(),
                ), // Stdin
            ]),
            server: Server::default(),
        }
    }
}

impl Config {
    pub fn read() -> Result<Self> {
        let config_path = PathBuf::from(consts::CONFIG_PATH);

        let mut config_file = match File::open(config_path) {
            Ok(file) => file,
            Err(_) => return Ok(Default::default()),
        };

        let mut config = String::new();
        config_file
            .read_to_string(&mut config)
            .context("reading error")?;

        Ok(toml::from_str(config.as_str())?)
    }
}
