use tracing::info;

use crate::plugin::{Event, EventBus, fire_event};

pub(crate) struct Proxy {
    event_bus: EventBus,
    port: u16,
}

impl Proxy {
    pub(crate) fn new(event_bus: EventBus, port: u16) -> Self {
        Self { event_bus, port }
    }

    pub(crate) async fn start(&self) -> Result<()> {
        fire_event(|env| -> Result<_> {
            Ok(self.event_bus.fire_event(env, Event::proxy_initialize()?))
        })?;

        info!("Starting proxy on port {}", self.port);

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ProxyError {
    #[error("JNI error: {0}")]
    Jni(#[from] jni::errors::Error),
}

type Result<T> = std::result::Result<T, ProxyError>;
