use super::proto;
use crate::configs_manager::ConfigsManager;
use crate::consts;
use crate::subprocesses_control::SubprocessesController;
use log::{error, info, warn};
use std::{ffi::OsStr, sync::Arc};
use tokio::sync::{Mutex, broadcast, mpsc};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Result, Status};

fn start(
    subprocess_controller: &mut SubprocessesController,
    configs_manager: &ConfigsManager,
    config_id: &str,
) -> Result<()> {
    subprocess_controller.start(
        configs_manager
            .get_path(config_id)
            .ok_or(Status::not_found(format!(
                "Config {:?} not found",
                config_id
            )))?,
        match configs_manager.get_metadata(config_id) {
            Some(metadata) => &metadata.core_tag,
            None => {
                error!("Metadata not found for config: {:?}", config_id);
                return Err(Status::internal(format!("Metadata not found")));
            }
        },
    )
}

async fn stop(
    subprocess_controller: &mut SubprocessesController,
    configs_manager: &ConfigsManager,
    config_id: &str,
) -> Result<()> {
    if let None = configs_manager.get_path(config_id) {
        return Err(Status::not_found(format!(
            "Config {:?} not found",
            config_id
        )));
    }

    if !subprocess_controller.is_running(OsStr::new(config_id)) {
        return Err(Status::failed_precondition("Config is already down"));
    }

    match subprocess_controller.stop(OsStr::new(config_id)).await {
        Ok(exit_code) => info!("Subprocess stopped with exit code: {}", exit_code),
        Err(e) => warn!("Subprocess stopping error: {}", e),
    }

    Ok(())
}

pub struct Management {
    subprocess_controller: Arc<Mutex<SubprocessesController>>,
    configs_manager: Arc<Mutex<ConfigsManager>>,
}

impl Management {
    pub fn new(
        subprocess_controller: Arc<Mutex<SubprocessesController>>,
        configs_manager: Arc<Mutex<ConfigsManager>>,
    ) -> Self {
        Self {
            subprocess_controller,
            configs_manager,
        }
    }
}

#[tonic::async_trait]
impl proto::management_server::Management for Management {
    async fn start(
        &self,
        request: Request<proto::StartRequest>,
    ) -> Result<Response<proto::StartResponse>> {
        let config_id = &request.get_ref().config_id;

        let mut subprocess_controller = self.subprocess_controller.lock().await;
        let configs_manager = self.configs_manager.lock().await;

        start(&mut subprocess_controller, &configs_manager, config_id)?;

        Ok(Response::new(proto::StartResponse::default()))
    }

    async fn restart(
        &self,
        request: Request<proto::RestartRequest>,
    ) -> Result<Response<proto::RestartResponse>> {
        let config_id = &request.get_ref().config_id;

        let mut subprocess_controller = self.subprocess_controller.lock().await;
        let configs_manager = self.configs_manager.lock().await;

        stop(&mut subprocess_controller, &configs_manager, config_id).await?;
        start(&mut subprocess_controller, &configs_manager, config_id)?;

        Ok(Response::new(proto::RestartResponse::default()))
    }

    async fn stop(
        &self,
        request: Request<proto::StopRequest>,
    ) -> Result<Response<proto::StopResponse>> {
        let config_id = &request.get_ref().config_id;

        let mut subprocess_controller = self.subprocess_controller.lock().await;
        let configs_manager = self.configs_manager.lock().await;

        stop(&mut subprocess_controller, &configs_manager, config_id).await?;

        Ok(Response::new(proto::StopResponse::default()))
    }

    type ReceiveLogStream = ReceiverStream<Result<proto::LogRecord>>;

    async fn receive_log(
        &self,
        request: Request<proto::ReceiveLogRequest>,
    ) -> Result<Response<Self::ReceiveLogStream>> {
        use proto::log_record::FileDescriptor as FD;

        let (tx, rx) = mpsc::channel(consts::LOGS_BUFFER_SIZE);

        let config_id = OsStr::new(&request.get_ref().config_id);

        let subprocess_controller = self.subprocess_controller.lock().await;

        for (mut receiver, fd) in vec![
            (
                match subprocess_controller.get_stdout(config_id) {
                    Some(r) => r,
                    None => return Err(Status::not_found("Config stdout not found")),
                },
                FD::Out,
            ),
            (
                match subprocess_controller.get_stderr(config_id) {
                    Some(r) => r,
                    None => return Err(Status::not_found("Config stderr not found")),
                },
                FD::Err,
            ),
        ] {
            let tx_clone = tx.clone();

            tokio::spawn(async move {
                loop {
                    match receiver.recv().await {
                        Ok(record) => tx_clone
                            .send(Ok(proto::LogRecord {
                                fd: fd.into(),
                                log: record,
                            }))
                            .await
                            .unwrap(),
                        Err(e) => {
                            use broadcast::error::RecvError;

                            match e {
                                RecvError::Closed => break,
                                RecvError::Lagged(_) => continue,
                            }
                        }
                    };
                }
            });
        }

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
