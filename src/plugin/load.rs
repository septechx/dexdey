use std::fs;

use jni::objects::JObject;
use jni::refs::Global;
use jni::strings::JNIString;
use jni::vm::JavaVM;
use jni::{JValue, jni_sig, jni_str};

use crate::plugin::EventBus;
use crate::plugin::parse::parse_velocity_plugin;

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct Plugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub main: String,
    pub instance: Global<JObject<'static>>,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum LoadPluginError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(#[from] crate::plugin::parse::ParsePluginError),
    #[error("JNI error: {0}")]
    Jvm(#[from] jni::errors::Error),
    #[error("EventBus error: {0}")]
    EventBus(#[from] crate::plugin::event::EventBusError),
}

type Result<T> = std::result::Result<T, LoadPluginError>;

pub(crate) fn load_plugins(bus: &mut EventBus) -> Result<Vec<Plugin>> {
    let mut plugins = vec![];
    for entry in fs::read_dir("plugins")? {
        let entry = entry?;

        if entry.file_type()?.is_dir() {
            load_plugins(bus)?;
            continue;
        }

        let path = entry.path();
        let Some(extension) = path.extension() else {
            continue;
        };
        if extension != "jar" {
            continue;
        }

        let plugin = parse_velocity_plugin(&path)?;
        plugins.push(plugin);
    }

    let mut loaded_plugins = vec![];
    let jvm = JavaVM::singleton()?;
    jvm.attach_current_thread(|env| -> Result<()> {
        let logger_class = env.find_class(jni_str!("org/slf4j/LoggerFactory"))?;

        for plugin in plugins {
            let class = env.load_class(JNIString::from(&plugin.main))?;
            let instance = env.new_object(class, jni_sig!("()V"), &[])?;

            let name = env.new_string(&plugin.id)?;
            let logger = env
                .call_static_method(
                    &logger_class,
                    jni_str!("getLogger"),
                    jni_sig!("(Ljava/lang/String;)Lorg/slf4j/Logger;"),
                    &[JValue::Object(&name)],
                )?
                .l()?;

            env.set_field(
                &instance,
                jni_str!("logger"),
                jni_sig!("Lorg/slf4j/Logger;"),
                JValue::Object(&logger),
            )?;

            let instance = env.new_global_ref(instance)?;
            bus.register_plugin(env, &plugin.id, &instance)?;

            loaded_plugins.push(Plugin {
                id: plugin.id,
                name: plugin.name,
                version: plugin.version,
                main: plugin.main,
                instance,
            });
        }

        Ok(())
    })?;

    Ok(loaded_plugins)
}
