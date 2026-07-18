use log::{error, warn};
use std::fs::{File, OpenOptions};
use std::io::ErrorKind;
use std::io::prelude::*;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tonic::{Result, Status};

pub struct ConfigsManager<'a> {
    directory: &'a Path,
}

impl<'a> ConfigsManager<'a> {
    pub fn new(directory: &'a Path) -> std::io::Result<Self> {
        std::fs::create_dir_all(directory)?;

        Ok(Self { directory })
    }

    pub fn list(&self) -> Result<Vec<String>> {
        let mut configs_ids = vec![];

        for entry in self.directory.read_dir().map_err(|err| {
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

                configs_ids.push(file_name);
            }
        }

        Ok(configs_ids)
    }

    pub fn get_path(&self, config_id: &String) -> Option<PathBuf> {
        let path = self.directory.join(config_id);

        if path.is_file() { Some(path) } else { None }
    }

    pub fn read(&self, config_id: &String) -> Result<String> {
        let mut file =
            File::open(self.directory.join(config_id)).map_err(|err| match err.kind() {
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

    pub fn edit(&self, config_id: &String, new_config: &String) -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(self.directory.join(config_id))?;

        std::fs::set_permissions(
            &self.directory.join(config_id),
            std::fs::Permissions::from_mode(0o660),
        )?;

        if let Err(err) = file.write(new_config.as_bytes()) {
            error!("Config {:?} editing error {}", config_id, err);
            return Err(Status::internal("Config editing error"));
        }

        return Ok(());
    }

    pub fn delete(&self, config_id: &String) -> Result<()> {
        std::fs::remove_file(self.directory.join(config_id)).map_err(|err| match err.kind() {
            ErrorKind::NotFound => Status::not_found("Config not found"),
            _ => {
                error!("Config {:?} deletion {} error", config_id, err);
                Status::internal("Config deletion error")
            }
        })
    }
}
