mod plugin;

use jni::{jni_sig, jni_str};

use plugin::EventBus;

use crate::plugin::fire_event;

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

    fire_event(|env| -> Result<_> {
        let event_class = env.find_class(jni_str!(
            "com/velocitypowered/api/event/proxy/ProxyInitializeEvent"
        ))?;
        let event = env.new_object(event_class, jni_sig!("()V"), &[])?;

        Ok(bus.fire(
            env,
            &event,
            "com.velocitypowered.api.event.proxy.ProxyInitializeEvent",
        ))
    })?;

    Ok(())
}
