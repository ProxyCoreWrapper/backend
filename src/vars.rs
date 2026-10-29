pub const LOGS_BUFFER_SIZE: usize = 64;

pub fn configuration_file_path() -> std::path::PathBuf {
    std::env::var_os("CONFIGURATION_DIRECTORY")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "/etc/pcw".into())
        .join("backend-config.toml")
}

pub fn runtime_socket_path() -> std::path::PathBuf {
    std::env::var_os("RUNTIME_DIRECTORY")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "/run/pcw".into())
        .join("socket")
}

pub fn configs_directory() -> std::path::PathBuf {
    std::env::var_os("STATE_DIRECTORY")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "/var/lib/pcw-backend".into())
        .join("configs")
}

pub fn metadata_path() -> std::path::PathBuf {
    std::env::var_os("STATE_DIRECTORY")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "/var/lib/pcw-backend".into())
        .join("meta_data.json")
}
