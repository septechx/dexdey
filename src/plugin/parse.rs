use std::fs::File;
use std::io::Read;
use std::path::Path;

use zip::ZipArchive;

#[derive(Debug, serde::Deserialize)]
pub(super) struct VelocityPlugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub main: String,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ParsePluginError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),
}

type Result<T> = std::result::Result<T, ParsePluginError>;

pub(super) fn parse_velocity_plugin(path: &Path) -> Result<VelocityPlugin> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut entry = archive.by_name("velocity-plugin.json")?;
    let mut contents = String::new();
    entry.read_to_string(&mut contents)?;
    let plugin: VelocityPlugin = serde_json::from_str(&contents)?;
    Ok(plugin)
}
