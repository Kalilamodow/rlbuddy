use std::{
    sync::{Arc, Mutex, mpsc},
    thread,
};

use super::{RLEvent, connect_to_stats_api};

pub struct StatsApi {
    subscribers: Arc<Mutex<Vec<mpsc::Sender<Arc<RLEvent>>>>>,
}

impl StatsApi {
    pub fn new() -> Self {
        StatsApi {
            subscribers: Arc::default(),
        }
    }

    pub fn start(&self) {
        let subscribers = Arc::clone(&self.subscribers);
        thread::spawn(move || {
            connect_to_stats_api(|event| {
                let mut subscribers = subscribers.lock().unwrap();
                let event = Arc::new(event);
                subscribers.retain(|sub| sub.send(Arc::clone(&event)).is_ok());
            });
        });
    }

    pub fn subscribe(&self) -> mpsc::Receiver<Arc<RLEvent>> {
        let (tx, rx) = mpsc::channel();
        let mut subscribers = self.subscribers.lock().unwrap();
        subscribers.push(tx);
        rx
    }
}
