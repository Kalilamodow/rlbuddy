use std::{cell::RefCell, collections::VecDeque, rc::Rc};

#[derive(Clone)]
pub struct Sender<T> {
    items: Rc<RefCell<VecDeque<T>>>,
}

impl<T> Sender<T> {
    pub fn send(&self, item: T) {
        self.items.borrow_mut().push_back(item);
    }
}

pub struct Receiver<T> {
    items: Rc<RefCell<VecDeque<T>>>,
}

impl<T> Receiver<T> {
    pub fn new() -> Self {
        Self {
            items: Rc::default(),
        }
    }

    pub fn try_recv(&self) -> Option<T> {
        self.items.borrow_mut().pop_front()
    }

    pub fn send(&self) -> Sender<T> {
        Sender {
            items: Rc::clone(&self.items),
        }
    }
}
