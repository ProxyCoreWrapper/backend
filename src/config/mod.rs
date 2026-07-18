mod model;

use model::Config;
use once_cell::sync::OnceCell;

static CONFIG: OnceCell<Config> = OnceCell::new();

pub fn read() -> anyhow::Result<&'static Config> {
    let config = Config::read()?;

    CONFIG
        .set(config)
        .map_err(|_| anyhow::anyhow!("config already has been read"))?;

    Ok(get_or_panic())
}

pub fn get_or_panic() -> &'static Config {
    CONFIG.get().expect("config hadn't been read")
}
