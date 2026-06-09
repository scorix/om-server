use std::sync::Arc;

use tokio::sync::Notify;

#[derive(Clone, Default)]
pub struct WeatherBakeWake {
    notify: Arc<Notify>,
}

impl WeatherBakeWake {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn subscribe(&self) -> Arc<Notify> {
        self.notify.clone()
    }

    pub fn signal(&self) {
        self.notify.notify_waiters();
    }
}
