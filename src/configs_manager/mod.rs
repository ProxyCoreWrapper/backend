mod metadata;

use crate::regexes::CONFIG_ID_REGEX;
use log::{error, warn};
use serde_json;
use std::fs::{File, OpenOptions};
use std::io::ErrorKind;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use tonic::{Result, Status};

pub struct ConfigsManager {
    configs_directory: PathBuf,
    metadata_path: PathBuf,
    metadata: metadata::MetadataDB,
}

pub struct ConfigEntry {
    pub config_id: String,
    pub core_tag: String,
}

impl ConfigsManager {
    pub fn new<P1: AsRef<Path>, P2: AsRef<Path>>(
        directory: P1,
        metadata_path: P2,
    ) -> std::io::Result<Self> {
        std::fs::create_dir_all(directory.as_ref())?;
        metadata_path
            .as_ref()
            .parent()
            .map(|parent| std::fs::create_dir_all(parent))
            .ok_or(ErrorKind::InvalidFilename)??;

        Ok(Self {
            configs_directory: directory.as_ref().to_path_buf(),
            metadata_path: metadata_path.as_ref().to_path_buf(),
            metadata: serde_json::from_reader(File::open(metadata_path)?).unwrap_or_else(|_| {
                error!("Metadata is corrupted. Creating an empty metadata...");
                metadata::MetadataDB::default()
            }),
        })
    }

    pub fn list(&self) -> Result<Vec<ConfigEntry>> {
        let mut configs_entries = vec![];

        for entry in self.configs_directory.read_dir().map_err(|err| {
            error!("Backend storage directory is inaccessible: {}", err);
            Status::internal("Backend storage directory is inaccessible")
        })? {
            if let Ok(entry) = entry {
                match entry.file_type() {
                    Ok(file_type) => {
                        if !file_type.is_file() {
                            continue;
                        }
                    }
                    Err(err) => {
                        warn!("{}", err);
                        continue;
                    }
                };

                let file_name = match entry.file_name().to_str() {
                    Some(name) => name.to_string(),
                    None => continue,
                };

                configs_entries.push(ConfigEntry {
                    core_tag: self
                        .metadata
                        .configs_meta
                        .get(&file_name)
                        .map(|meta| meta.core_tag.clone())
                        .unwrap_or_else(String::new),
                    config_id: file_name,
                });
            }
        }

        Ok(configs_entries)
    }

    pub fn get_path(&self, config_id: &str) -> Option<PathBuf> {
        let path = self.configs_directory.join(config_id);

        if path.is_file() { Some(path) } else { None }
    }

    pub fn read(&self, config_id: &str) -> Result<String> {
        if !CONFIG_ID_REGEX.is_match(config_id) {
            return Err(Status::invalid_argument(
                "Config id doesn't match \"[\\w-]+\"",
            ));
        }

        let mut file =
            File::open(self.configs_directory.join(config_id)).map_err(|err| match err.kind() {
                ErrorKind::NotFound => return Status::not_found("Config not found"),
                _ => {
                    error!("File opening error: {}", err);
                    return Status::internal("Config reading error");
                }
            })?;

        let mut config = String::new();
        if let Err(err) = file.read_to_string(&mut config) {
            match err.kind() {
                ErrorKind::InvalidData => {
                    error!("Config {:?} contains non UTF-8 symbols", config_id);
                    return Err(Status::internal("Config reading error"));
                }
                _ => {
                    error!("Config {:?} reading error {}", config_id, err);
                    return Err(Status::internal("Config reading error"));
                }
            }
        }

        Ok(config)
    }

    pub fn edit(
        &mut self,
        config_id: &str,
        new_config: Option<&String>,
        core_tag: Option<String>,
    ) -> Result<()> {
        if !CONFIG_ID_REGEX.is_match(config_id) {
            return Err(Status::invalid_argument(
                "Config id doesn't match \"[\\w-]+\"",
            ));
        }

        if !self.metadata.configs_meta.contains_key(config_id)
            && new_config.is_none()
            && core_tag.is_none()
        {
            return Err(Status::invalid_argument(
                "To create a config both config and core tag must be provided",
            ));
        }

        if let Some(new_config) = new_config {
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(self.configs_directory.join(config_id))?;

            file.write(new_config.as_bytes()).map_err(|err| {
                error!("Config {:?} editing error: {}", config_id, err);
                Status::internal("Config editing error")
            })?;
        }

        if let Some(core_tag) = core_tag {
            self.metadata.configs_meta.insert(
                config_id.to_string(),
                metadata::ConfigMetadata { core_tag: core_tag },
            );

            self.save_metadata()?;
        }

        Ok(())
    }

    pub fn delete(&mut self, config_id: &str) -> Result<()> {
        if !CONFIG_ID_REGEX.is_match(config_id) {
            return Err(Status::invalid_argument(
                "Config id doesn't match \"[\\w-]+\"",
            ));
        }

        self.metadata.configs_meta.remove(config_id);
        self.save_metadata()?;

        std::fs::remove_file(self.configs_directory.join(config_id)).map_err(|err| {
            match err.kind() {
                ErrorKind::NotFound => Status::not_found("Config not found"),
                _ => {
                    error!("Config {:?} deletion {} error", config_id, err);
                    Status::internal("Config deletion error")
                }
            }
        })
    }

    pub fn get_metadata(&self, config_id: &str) -> Option<&metadata::ConfigMetadata> {
        self.metadata.configs_meta.get(config_id)
    }

    fn save_metadata(&self) -> Result<()> {
        serde_json::to_writer(
            OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&self.metadata_path)
                .map_err(|err| {
                    error!("Metadata file opening error: {}", err);
                    Status::internal("Metadata file opening error")
                })?,
            &self.metadata,
        )
        .map_err(|err| {
            error!("Metadata saving error: {}", err);
            Status::internal("Metadata saving error")
        })?;

        Ok(())
    }
}
