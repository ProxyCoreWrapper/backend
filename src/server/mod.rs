mod configs;
mod events;
mod managment;
mod proto;

use crate::configs_manager::ConfigsManager;
use crate::subprocess_control::SubprocessController;
use anyhow::Context;
use log::info;
use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::transport::Server;

pub async fn start<'a>(
    subprocess_controller: Arc<Mutex<SubprocessController<'static>>>,
    configs_manager: Arc<Mutex<ConfigsManager<'static>>>,
) -> anyhow::Result<()> {
    let config = crate::config::get_or_panic();

    std::fs::create_dir_all(config.server.uds_path.with_file_name(""))
        .context("socket's directory creating error")?;
    let uds =
        tokio::net::UnixListener::bind(&config.server.uds_path).context("socket binding error")?;
    std::fs::set_permissions(&config.server.uds_path, Permissions::from_mode(0o660))
        .context("socket permissions setting error")?;
    let incoming = tokio_stream::wrappers::UnixListenerStream::new(uds);

    let managment_serivce = proto::managment_server::ManagmentServer::new(
        managment::Managment::new(subprocess_controller, configs_manager.clone()),
    );
    let configs_serivce =
        proto::configs_server::ConfigsServer::new(configs::Configs::new(configs_manager));
    let network_event_service =
        proto::network_events_server::NetworkEventsServer::new(events::Events::default());

    info!("Running...");

    let shutdown_signal = async {
        use tokio::signal::unix::{SignalKind, signal};

        let mut sigint = signal(SignalKind::interrupt()).expect("cannot create a SIGINT listener");
        let mut sigterm =
            signal(SignalKind::terminate()).expect("cannot create a SIGTERM listener");
        tokio::select! {
            _ = sigint.recv() => {},
            _ = sigterm.recv() => {},
        }
    };

    Server::builder()
        .add_service(managment_serivce)
        .add_service(configs_serivce)
        .add_service(network_event_service)
        .serve_with_incoming_shutdown(incoming, shutdown_signal)
        .await?;

    info!("Stopping...");

    std::fs::remove_file(&config.server.uds_path).context("socket removing error")?;

    Ok(())
}
