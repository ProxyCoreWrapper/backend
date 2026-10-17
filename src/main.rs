mod config;
mod configs_manager;
mod vars;
mod regexes;
mod server;
mod subprocesses_control;

use log::{error, info};
use std::process::ExitCode;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> ExitCode {
    simple_logger::SimpleLogger::new()
        .init()
        .expect("logger initialization error");

    info!("Initializing...");

    let config = match config::read() {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Configuration reading error: {}", e);
            return ExitCode::FAILURE;
        }
    };

    let configs_directory = vars::configs_directory();
    let metadata_path = vars::metadata_path();

    let controller = Arc::new(Mutex::new(subprocesses_control::SubprocessesController::new(
        config.cores.clone(),
    )));
    let configs_manager = Arc::new(Mutex::new(
        match configs_manager::ConfigsManager::new(configs_directory, metadata_path) {
            Ok(manager) => manager,
            Err(e) => {
                error!("Configs manager initialization error: {}", e);
                return ExitCode::FAILURE;
            }
        },
    ));

    match server::start(controller.clone(), configs_manager).await {
        Ok(_) => info!("Server was stopped"),
        Err(e) => error!("Server error: {}", e),
    }

    let mut subprocess = controller.lock().await;
    let stop_results = subprocess.stop_all().await;
    match stop_results.len() {
        0 => info!("All subprocesses stopped"),
        _ => error!("Error stopping subprocesses:\n{}", stop_results),
    }

    info!("Gracefully stopped!");

    ExitCode::SUCCESS
}
