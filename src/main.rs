mod plugin;

use jni::vm::JavaVM;
use jni::{JValue, jni_sig, jni_str};

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("JNI error: {0}")]
    Jni(#[from] jni::errors::Error),
    #[error("JVM error: {0}")]
    Jvm(#[from] jni::vm::JvmError),
    #[error("Start JVM error: {0}")]
    StartJvm(#[from] jni::errors::StartJvmError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Plugin init error: {0}")]
    PluginInit(#[from] plugin::PluginInitError),
}

type Result<T> = std::result::Result<T, Error>;

fn main() -> Result<()> {
    plugin::init()?;

    let jvm = JavaVM::singleton()?;

    jvm.attach_current_thread(|env| -> Result<()> {
        let class = env.load_class(jni_str!("com.siesque.testPlugin.TestPlugin"))?;

        let instance = env.new_object(class, jni_sig!("()V"), &[])?;

        let logger_class = env.find_class(jni_str!("org/slf4j/LoggerFactory"))?;
        let name = env.new_string("test-plugin")?;
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

        let event_class = env.find_class(jni_str!(
            "com/velocitypowered/api/event/proxy/ProxyInitializeEvent"
        ))?;
        let event = env.new_object(event_class, jni_sig!("()V"), &[])?;

        env.call_method(
            &instance,
            jni_str!("onProxyInitialization"),
            jni_sig!("(Lcom/velocitypowered/api/event/proxy/ProxyInitializeEvent;)V"),
            &[JValue::Object(&event)],
        )?;

        Ok(())
    })
}
