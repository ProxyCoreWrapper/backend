use super::proto;
use crate::configs_manager::ConfigsManager;
use crate::consts;
use crate::subprocess_control::SubprocessController;
use log::{error, info, warn};
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast, mpsc};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Result, Status};

pub struct Managment {
    subprocess_controller: Arc<Mutex<SubprocessController<'static>>>,
    configs_manager: Arc<Mutex<ConfigsManager<'static>>>,
}

impl Managment {
    pub fn new(
        subprocess_controller: Arc<Mutex<SubprocessController<'static>>>,
        configs_manager: Arc<Mutex<ConfigsManager<'static>>>,
    ) -> Self {
        Self {
            subprocess_controller,
            configs_manager,
        }
    }
}

#[tonic::async_trait]
impl proto::managment_server::Managment for Managment {
    async fn restart(
        &self,
        request: Request<proto::RestartRequest>,
    ) -> Result<Response<proto::RestartResponse>> {
        let config_id = &request.get_ref().config;

        let mut subprocess = self.subprocess_controller.lock().await;
        let configs_manager = self.configs_manager.lock().await;

        let config_path = configs_manager
            .get_path(config_id)
            .ok_or(Status::not_found(format!(
                "Config {:?} not found",
                config_id
            )))?;

        if subprocess.is_running() {
            match subprocess.stop().await {
                Ok(exit_code) => info!("Subprocess stopped with exit code: {}", exit_code),
                Err(e) => warn!("Subprocess stopping error: {}", e),
            }
        }

        if let Err(e) = subprocess.start(config_path) {
            error!("Subprocess starting error: {}", e);
        }

        Ok(Response::new(proto::RestartResponse::default()))
    }

    async fn stop(&self, _: Request<proto::StopRequest>) -> Result<Response<proto::StopResponse>> {
        let mut subprocess = self.subprocess_controller.lock().await;

        if subprocess.is_running() {
            match subprocess.stop().await {
                Ok(exit_code) => info!("Subprocess stopped with exit code: {}", exit_code),
                Err(e) => warn!("Subprocess stopping error: {}", e),
            }
        }

        Ok(Response::new(proto::StopResponse::default()))
    }

    async fn get_state(
        &self,
        _: Request<proto::GetStateRequest>,
    ) -> Result<Response<proto::GetStateResponse>> {
        use proto::get_state_response::State::{Down, Running};

        let state = if self.subprocess_controller.lock().await.is_running() {
            Running
        } else {
            Down
        };

        Ok(Response::new(proto::GetStateResponse {
            state: state.into(),
        }))
    }

    type ReceiveLogStream = ReceiverStream<Result<proto::LogRecord>>;

    async fn receive_log(
        &self,
        _: Request<proto::ReceiveLogRequest>,
    ) -> Result<Response<Self::ReceiveLogStream>> {
        use proto::log_record::FileDescriptor as FD;

        let (tx, rx) = mpsc::channel(consts::LOGS_BUFFER_SIZE);

        let subprocess_controller = self.subprocess_controller.lock().await;

        for (mut receiver, fd) in vec![
            (subprocess_controller.get_stdout(), FD::Out),
            (subprocess_controller.get_stderr(), FD::Err),
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
