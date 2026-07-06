mod config;
mod plugin;
mod protocol;
mod proxy;

use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::plugin::EventBus;
use crate::proxy::Proxy;

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Plugin init error: {0}")]
    PluginInit(#[from] plugin::PluginInitError),
    #[error("Plugin load error: {0}")]
    PluginLoad(#[from] plugin::LoadPluginError),
    #[error("Proxy error: {0}")]
    Proxy(#[from] proxy::ProxyError),
}

type Result<T> = std::result::Result<T, Error>;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let config = Config::load();

    plugin::init()?;
    let mut bus = EventBus::new();
    plugin::load_plugins(&mut bus)?;

    let proxy = Proxy::new(bus, config, 25565);
    proxy.start().await?;

    Ok(())
}
