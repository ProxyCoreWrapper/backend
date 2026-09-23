use crate::consts;
use log::{error, warn};
use nix::sys::signal::{SIGTERM, kill};
use nix::unistd::Pid;
use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::io;
use std::path::Path;
use std::process::ExitStatus;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::broadcast;
use tonic::{Result, Status};

fn spawn_reader<R>(reader: R, tx: broadcast::Sender<String>)
where
    R: AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = tx.send(line);
        }
    });
}

fn parse_command(command: &[String], config_path: &OsStr) -> io::Result<Command> {
    let mut is_config_path_replaced = false;

    let mut parts = command.iter();
    let cmd = parts.next().unwrap_or(&"".to_string()).clone();
    let args: Vec<_> = parts
        .map(|s| {
            if s.contains("%c") {
                is_config_path_replaced = true;

                s.replace("%c", config_path.to_str().unwrap_or(""))
            } else {
                s.clone()
            }
        })
        .collect();

    let mut cmd = Command::new(cmd);
    cmd.args(args);
    if !is_config_path_replaced {
        let config_file = std::fs::File::open(config_path).map_err(|e| {
            error!("Failed to open config file {:?}: {}", config_path, e);
            e
        })?;

        cmd.stdin(config_file);
    }

    Ok(cmd)
}

async fn kill_process(process: &mut Child) -> io::Result<ExitStatus> {
    kill(
        Pid::from_raw(process.id().ok_or(io::Error::new(
            io::ErrorKind::NotFound,
            "Subprocess isn't running",
        ))? as i32),
        SIGTERM,
    )?;

    process.wait().await
}

pub struct Subprocess {
    pub(self) process: Child,
    pub(self) stdout: broadcast::Sender<String>,
    pub(self) stderr: broadcast::Sender<String>,
}

impl Subprocess {
    pub fn new(mut process: Child) -> Self {
        let (stdout, _) = broadcast::channel(consts::LOGS_BUFFER_SIZE);
        let (stderr, _) = broadcast::channel(consts::LOGS_BUFFER_SIZE);

        if let Some(out) = process.stdout.take() {
            spawn_reader(out, stdout.clone());
        } else {
            warn!("Started subprocess without stdout pipe");
        }

        if let Some(err) = process.stderr.take() {
            spawn_reader(err, stderr.clone());
        } else {
            warn!("Started subprocess without stderr pipe");
        }

        Self {
            process,
            stdout,
            stderr,
        }
    }
}

pub struct SubprocessesController {
    cores: BTreeMap<String, Vec<String>>, // Core name, execute command
    running_configs: BTreeMap<OsString, Subprocess>, // Config id, process
}

impl SubprocessesController {
    pub fn new(cores: BTreeMap<String, Vec<String>>) -> Self {
        Self {
            cores,
            running_configs: BTreeMap::new(),
        }
    }

    pub fn is_running(&self, config: &OsStr) -> bool {
        if let Some(process) = self.running_configs.get(config).map(|s| &s.process) {
            process.id().is_some()
        } else {
            false
        }
    }

    pub fn start<P: AsRef<Path>>(&mut self, config: P, core_tag: &String) -> Result<()> {
        let config_name = match config.as_ref().file_name() {
            Some(name) => name,
            None => return Err(Status::not_found("Invalid config path")),
        };

        if self.is_running(config_name) {
            return Err(Status::failed_precondition("The config is already running"));
        }

        let core_start_command = self
            .cores
            .get(core_tag)
            .ok_or_else(|| Status::not_found("Core not found"))?;

        let command = parse_command(core_start_command, config.as_ref().as_os_str())
            .map_err(|e| Status::internal(e.to_string()))?
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                error!("Error spawning subprocess: {}", e);
                Status::internal(e.to_string())
            })?;

        self.running_configs
            .insert(config_name.to_os_string(), Subprocess::new(command));

        Ok(())
    }

    pub async fn stop(&mut self, config: &OsStr) -> Result<ExitStatus> {
        match self.running_configs.get_mut(config).map(|s| &mut s.process) {
            Some(process) => kill_process(process).await.map_err(|e| {
                error!("Error stopping subprocess: {}", e);
                Status::internal(e.to_string())
            }),
            None => Err(Status::failed_precondition("The config isn't running")),
        }
    }

    pub fn get_stdout(&self, config: &OsStr) -> Option<broadcast::Receiver<String>> {
        self.running_configs
            .get(config)
            .map(|s| s.stdout.subscribe())
    }

    pub fn get_stderr(&self, config: &OsStr) -> Option<broadcast::Receiver<String>> {
        self.running_configs
            .get(config)
            .map(|s| s.stderr.subscribe())
    }

    pub async fn stop_all(&mut self) -> String {
        let mut errors = String::new();

        for (config, subprocess) in self.running_configs.iter_mut() {
            match kill_process(&mut subprocess.process).await {
                Ok(exit_status) => {
                    if !exit_status.success() {
                        errors.push_str(&format!(
                            "Subprocess for config {:?} exited with status: {}\n",
                            config.to_string_lossy(),
                            exit_status
                        ));
                    }
                }
                Err(e) => {
                    errors.push_str(&format!(
                        "Error stopping subprocess for config {:?}: {}\n",
                        config.to_string_lossy(),
                        e
                    ));
                }
            }
        }

        errors
    }
}
