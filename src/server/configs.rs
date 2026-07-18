use super::proto;
use crate::configs_manager::ConfigsManager;
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{Request, Response, Result};

pub struct Configs {
    manager: Arc<Mutex<ConfigsManager<'static>>>,
}

impl Configs {
    pub fn new(manager: Arc<Mutex<ConfigsManager<'static>>>) -> Self {
        Self { manager }
    }
}

#[tonic::async_trait]
impl proto::configs_server::Configs for Configs {
    async fn list_configs(
        &self,
        _: Request<proto::ListConfigsRequest>,
    ) -> Result<Response<proto::ListConfigsResponse>> {
        Ok(Response::new(proto::ListConfigsResponse {
            configs_ids: self.manager.lock().await.list()?,
        }))
    }

    async fn get_config(
        &self,
        request: Request<proto::GetConfigRequest>,
    ) -> Result<Response<proto::GetConfigResponse>> {
        Ok(Response::new(proto::GetConfigResponse {
            config: self
                .manager
                .lock()
                .await
                .read(&request.get_ref().config_id)?,
        }))
    }

    async fn edit_config(
        &self,
        request: Request<proto::EditConfigRequest>,
    ) -> Result<Response<proto::EditConfigResponse>> {
        self.manager
            .lock()
            .await
            .edit(&request.get_ref().config_id, &request.get_ref().config)?;

        Ok(Response::new(proto::EditConfigResponse::default()))
    }

    async fn delete_config(
        &self,
        request: Request<proto::DeleteConfigRequest>,
    ) -> Result<Response<proto::DeleteConfigResponse>> {
        self.manager
            .lock()
            .await
            .delete(&request.get_ref().config_id)?;

        Ok(Response::new(proto::DeleteConfigResponse::default()))
    }
}
