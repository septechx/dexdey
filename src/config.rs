use std::path::Path;
use tracing::info;

#[derive(Clone)]
pub(crate) struct Config {
    pub forwarding_secret: Vec<u8>,
}

impl Config {
    pub(crate) fn load() -> Self {
        let secret = load_forwarding_secret();
        Self {
            forwarding_secret: secret,
        }
    }
}

fn load_forwarding_secret() -> Vec<u8> {
    if let Ok(val) = std::env::var("VELOCITY_FORWARDING_SECRET")
        && !val.is_empty()
    {
        info!("Loaded forwarding secret from VELOCITY_FORWARDING_SECRET env var");
        return val.into_bytes();
    }

    let secret_path =
        std::env::var("FORWARDING_SECRET_FILE").unwrap_or_else(|_| "forwarding.secret".to_string());
    let path = Path::new(&secret_path);

    if path.exists() {
        match std::fs::read_to_string(path) {
            Ok(contents) => {
                let trimmed = contents.trim().to_string();
                if !trimmed.is_empty() {
                    info!("Loaded forwarding secret from {}", path.display());
                    return trimmed.into_bytes();
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to read forwarding secret file {}: {e}",
                    path.display()
                );
            }
        }
    }

    tracing::warn!(
        "No forwarding secret found. Set VELOCITY_FORWARDING_SECRET or create {}",
        path.display()
    );
    Vec::new()
}
