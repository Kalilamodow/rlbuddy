// okok so basically this hides borrow_mut from Rc<RefCell>

use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

#[derive(Clone)]
pub struct ReadonlyStateHandle<T> {
    state: Rc<RefCell<T>>,
}

impl<T> ReadonlyStateHandle<T> {
    pub fn over(state: &Rc<RefCell<T>>) -> Self {
        Self {
            state: Rc::clone(state),
        }
    }

    pub fn read(&self) -> Ref<'_, T> {
        self.state.borrow()
    }
}

#[derive(Clone)]
pub struct ReadWriteStateHandle<T> {
    state: Rc<RefCell<T>>,
}

impl<T> ReadWriteStateHandle<T> {
    pub fn over(state: &Rc<RefCell<T>>) -> Self {
        Self {
            state: Rc::clone(state),
        }
    }

    pub fn read(&self) -> Ref<'_, T> {
        self.state.borrow()
    }

    pub fn write(&self) -> RefMut<'_, T> {
        self.state.borrow_mut()
    }
}
