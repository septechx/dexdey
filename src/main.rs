mod plugin;

use jni::vm::JavaVM;
use jni::{JValue, jni_sig, jni_str};

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
    let plugins = plugin::load_plugins()?;

    let jvm = JavaVM::singleton()?;
    jvm.attach_current_thread(|env| -> Result<()> {
        for plugin in plugins {
            let event_class = env.find_class(jni_str!(
                "com/velocitypowered/api/event/proxy/ProxyInitializeEvent"
            ))?;
            let event = env.new_object(event_class, jni_sig!("()V"), &[])?;

            env.call_method(
                plugin.instance,
                jni_str!("onProxyInitialization"),
                jni_sig!("(Lcom/velocitypowered/api/event/proxy/ProxyInitializeEvent;)V"),
                &[JValue::Object(&event)],
            )?;
        }

        Ok(())
    })?;

    Ok(())
}
