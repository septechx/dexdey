mod plugin;
mod proxy;

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
    tracing_subscriber::fmt().init();

    plugin::init()?;
    let mut bus = EventBus::new();
    plugin::load_plugins(&mut bus)?;

    let proxy = Proxy::new(bus, 25565);
    proxy.start().await?;

    Ok(())
}
