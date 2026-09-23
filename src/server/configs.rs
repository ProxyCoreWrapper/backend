use super::proto;
use crate::{configs_manager::ConfigsManager, subprocesses_control::SubprocessesController};
use std::{ffi::OsStr, pin::Pin, sync::Arc};
use tokio::sync::Mutex;
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Result, Status};

pub struct Configs {
    subprocess_controller: Arc<Mutex<SubprocessesController>>,
    configs_manager: Arc<Mutex<ConfigsManager>>,
}

impl Configs {
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
impl proto::configs_server::Configs for Configs {
    type ListConfigsStream = Pin<Box<dyn Stream<Item = Result<proto::Config, Status>> + Send>>;

    async fn list_configs(
        &self,
        _: Request<proto::ListConfigsRequest>,
    ) -> Result<Response<Self::ListConfigsStream>> {
        let configs = self.configs_manager.lock().await.list()?;

        let subprocess_controller = self.subprocess_controller.clone();
        let output_stream = tokio_stream::iter(configs).then(move |config| {
            let subprocess_controller = subprocess_controller.clone();

            async move {
                let state = match subprocess_controller
                    .lock()
                    .await
                    .is_running(OsStr::new(&config.config_id))
                {
                    true => proto::config::State::Running,
                    false => proto::config::State::Down,
                };

                Ok(proto::Config {
                    config_id: config.config_id,
                    core_tag: config.core_tag,
                    state: state.into(),
                })
            }
        });

        Ok(Response::new(
            Box::pin(output_stream) as Self::ListConfigsStream
        ))
    }

    async fn get_config(
        &self,
        request: Request<proto::GetConfigRequest>,
    ) -> Result<Response<proto::GetConfigResponse>> {
        Ok(Response::new(proto::GetConfigResponse {
            config: self
                .configs_manager
                .lock()
                .await
                .read(&request.get_ref().config_id)?,
        }))
    }

    async fn edit_config(
        &self,
        request: Request<proto::EditConfigRequest>,
    ) -> Result<Response<proto::EditConfigResponse>> {
        let req = request.get_ref();

        self.configs_manager
            .lock()
            .await
            .edit(&req.config_id, &req.config, &req.core_tag)?;

        Ok(Response::new(proto::EditConfigResponse::default()))
    }

    async fn delete_config(
        &self,
        request: Request<proto::DeleteConfigRequest>,
    ) -> Result<Response<proto::DeleteConfigResponse>> {
        self.configs_manager
            .lock()
            .await
            .delete(&request.get_ref().config_id)?;

        Ok(Response::new(proto::DeleteConfigResponse::default()))
    }
}
