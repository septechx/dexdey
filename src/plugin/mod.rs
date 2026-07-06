use std::fs;
use std::path::Path;

use jni::vm::{InitArgsBuilder, JavaVM};

#[derive(Debug, thiserror::Error)]
pub(crate) enum PluginInitError {
    #[error("JVM error: {0}")]
    Jvm(#[from] jni::vm::JvmError),
    #[error("Start JVM error: {0}")]
    StartJvm(#[from] jni::errors::StartJvmError),
}

type Result<T> = std::result::Result<T, PluginInitError>;

pub(crate) fn init() -> Result<()> {
    let jvm_args = InitArgsBuilder::new()
        .option(format!(
            "-Djava.class.path={}:{}",
            build_classpath("java_libs"),
            build_classpath("plugins"),
        ))
        .build()?;
    JavaVM::new(jvm_args)?;

    Ok(())
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
