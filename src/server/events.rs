use super::proto;
use tonic::{Request, Response, Result};

#[derive(Default)]
pub struct Events {}

#[tonic::async_trait]
impl proto::network_events_server::NetworkEvents for Events {
    async fn get_config(
        &self,
        request: Request<proto::GetNetworkEventsConfigRequest>,
    ) -> Result<Response<proto::GetNetworkEventsConfigResponse>> {
        Ok(Response::new(proto::GetNetworkEventsConfigResponse::default()))
    }

    async fn edit_config(
        &self,
        request: Request<proto::EditNetworkEventsConfigRequest>,
    ) -> Result<Response<proto::EditNetworkEventsConfigResponse>> {
        Ok(Response::new(proto::EditNetworkEventsConfigResponse::default()))
    }
}
