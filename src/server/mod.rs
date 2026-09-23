mod configs;
mod management;
mod proto;

use crate::configs_manager::ConfigsManager;
use crate::subprocesses_control::SubprocessesController;
use anyhow::Context;
use log::info;
use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::transport::Server;

pub async fn start(
    subprocess_controller: Arc<Mutex<SubprocessesController>>,
    configs_manager: Arc<Mutex<ConfigsManager>>,
) -> anyhow::Result<()> {
    let config = crate::config::get_or_panic();

    std::fs::create_dir_all(config.server.uds_path.with_file_name(""))
        .context("socket's directory creating error")?;
    let uds =
        tokio::net::UnixListener::bind(&config.server.uds_path).context("socket binding error")?;
    std::fs::set_permissions(&config.server.uds_path, Permissions::from_mode(0o660))
        .context("socket permissions setting error")?;
    let incoming = tokio_stream::wrappers::UnixListenerStream::new(uds);

    let management_serivce = proto::management_server::ManagementServer::new(
        management::Management::new(subprocess_controller.clone(), configs_manager.clone()),
    );
    let configs_serivce =
        proto::configs_server::ConfigsServer::new(configs::Configs::new(subprocess_controller, configs_manager));

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
        .add_service(management_serivce)
        .add_service(configs_serivce)
        .serve_with_incoming_shutdown(incoming, shutdown_signal)
        .await?;

    info!("Stopping...");

    std::fs::remove_file(&config.server.uds_path).context("socket removing error")?;

    Ok(())
}
