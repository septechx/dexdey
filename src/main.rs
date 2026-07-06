mod plugin;

use crate::plugin::{Event, EventBus, fire_event};

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("JNI error: {0}")]
    Jni(#[from] jni::errors::Error),
    #[error("Plugin init error: {0}")]
    PluginInit(#[from] plugin::PluginInitError),
    #[error("Plugin load error: {0}")]
    PluginLoad(#[from] plugin::LoadPluginError),
}

type Result<T> = std::result::Result<T, Error>;

fn main() -> Result<()> {
    plugin::init()?;
    let mut bus = EventBus::new();
    plugin::load_plugins(&mut bus)?;

    fire_event(|env| -> Result<_> { Ok(bus.fire_event(env, Event::proxy_initialize()?)) })?;

    Ok(())
}
