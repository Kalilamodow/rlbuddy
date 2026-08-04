use std::rc::Rc;

use crate::common::channel::{Receiver, Sender};

pub type EventReceiver<T> = Receiver<Rc<T>>;

pub struct EventSource<T> {
    senders: Vec<Sender<Rc<T>>>,
}

impl<T> EventSource<T> {
    pub fn new() -> Self {
        Self {
            senders: Vec::new(),
        }
    }

    pub fn publish(&mut self, item: T) {
        let rc = Rc::new(item);

        self.senders.retain(|sender| {
            sender
                .is_alive()
                .then(|| sender.send(Rc::clone(&rc)))
                .is_some()
        });
    }

    pub fn subscribe(&mut self) -> EventReceiver<T> {
        let receiver = Receiver::new();
        self.senders.push(receiver.send());
        receiver
    }
}
