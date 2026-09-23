mod config;
mod configs_manager;
mod consts;
mod server;
mod subprocesses_control;

use log::{error, info};
use std::os::unix::fs::PermissionsExt;
use std::process::ExitCode;
use std::sync::Arc;
use tokio::sync::Mutex;

const CONFIGS_DIR: &str = const_format::concatcp!(consts::BACKEND_DIR, "/", consts::CONFIGS_DIR);
const META_DATA_PATH: &str =
    const_format::concatcp!(consts::BACKEND_DIR, "/", consts::META_DATA_PATH);

fn initialize_backend_directory(dir: &str) -> ExitCode {
    if let Err(e) = std::fs::create_dir_all(dir) {
        error!("Failed to initialize backend directory: {}", e);
        return ExitCode::FAILURE;
    }
    if let Err(e) = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700)) {
        error!("Failed to set permissions for backend directory: {}", e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

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

    if initialize_backend_directory(consts::BACKEND_DIR) != ExitCode::SUCCESS {
        return ExitCode::FAILURE;
    }

    let controller = Arc::new(Mutex::new(subprocesses_control::SubprocessesController::new(
        config.cores.clone(),
    )));
    let configs_manager = Arc::new(Mutex::new(
        match configs_manager::ConfigsManager::new(CONFIGS_DIR, META_DATA_PATH) {
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
