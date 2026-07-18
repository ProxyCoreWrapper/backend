mod config;
mod configs_manager;
mod consts;
mod server;
mod subprocess_control;

use log::{error, info};
use std::path::Path;
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

    let controller = Arc::new(Mutex::new(subprocess_control::SubprocessController::new(
        &config.subprocess.binary,
    )));
    let configs_manager = Arc::new(Mutex::new(
        match configs_manager::ConfigsManager::new(Path::new(consts::STORAGE_DIR)) {
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
    if subprocess.is_running() {
        match subprocess.stop().await {
            Ok(exit_code) => info!("Subprocess stopped with exit code: {}", exit_code),
            Err(e) => error!("Subprocess stopping error: {}", e),
        }
    }

    info!("Gracefully stopped!");

    ExitCode::SUCCESS
}
