use std::fs;
use std::path::Path;

use jni::objects::JObject;
use jni::vm::{InitArgsBuilder, JavaVM};
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
}

type Result<T> = std::result::Result<T, Error>;

fn main() -> Result<()> {
    let jvm_args = InitArgsBuilder::new()
        .option(format!(
            "-Djava.class.path={}:{}",
            build_classpath("java_libs"),
            build_classpath("plugins"),
        ))
        .build()?;
    let jvm = JavaVM::new(jvm_args)?;

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

        env.call_method(
            &instance,
            jni_str!("onProxyInitialization"),
            jni_sig!("(Lcom/velocitypowered/api/event/proxy/ProxyInitializeEvent;)V"),
            &[JValue::Object(&JObject::null())],
        )?;

        Ok(())
    })
}

fn build_classpath<P: AsRef<Path>>(dir: P) -> String {
    let mut entries = vec![];
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();

        if entry.file_type().unwrap().is_dir() {
            let path = entry.path();
            entries.push(build_classpath(path));
        }

        let path = entry.path();
        if path.extension().is_some_and(|e| e == "jar") {
            entries.push(path.to_string_lossy().to_string());
        }
    }
    entries.join(":")
}
