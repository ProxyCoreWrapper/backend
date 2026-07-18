use crate::consts;
use nix::sys::signal::{SIGTERM, kill};
use nix::unistd::Pid;
use std::io;
use std::option::Option;
use std::path::Path;
use std::process::ExitStatus;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::broadcast;

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

pub struct SubprocessController<'a> {
    command: &'a str,
    process: Option<Child>,
    stdout: broadcast::Sender<String>,
    stderr: broadcast::Sender<String>,
}

impl<'a> SubprocessController<'a> {
    pub fn new(command: &'a str) -> Self {
        let (stdout, _) = broadcast::channel(consts::LOGS_BUFFER_SIZE);
        let (stderr, _) = broadcast::channel(consts::LOGS_BUFFER_SIZE);

        Self {
            command,
            process: None,
            stdout,
            stderr,
        }
    }

    pub fn is_running(&self) -> bool {
        if let Some(process) = self.process.as_ref() {
            process.id().is_some()
        } else {
            false
        }
    }

    pub fn start<P: AsRef<Path>>(&mut self, config: P) -> io::Result<()> {
        let mut command = Command::new(self.command)
            .arg("-c")
            .arg(config.as_ref().as_os_str())
            .arg("run")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        spawn_reader(command.stdout.take().unwrap(), self.stdout.clone());
        spawn_reader(command.stderr.take().unwrap(), self.stderr.clone());

        self.process = Some(command);

        Ok(())
    }

    pub async fn stop(&mut self) -> io::Result<ExitStatus> {
        match self.process.take() {
            Some(mut process) => {
                kill(
                    Pid::from_raw(process.id().expect("Subprocess isn't running") as i32),
                    SIGTERM,
                )?;

                process.wait().await
            }
            None => panic!("Subprocess isn't running"),
        }
    }

    pub fn get_stdout(&self) -> broadcast::Receiver<String> {
        self.stdout.subscribe()
    }

    pub fn get_stderr(&self) -> broadcast::Receiver<String> {
        self.stderr.subscribe()
    }
}
