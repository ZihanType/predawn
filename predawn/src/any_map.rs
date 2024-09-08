use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

#[derive(Default)]
pub struct AnyMap(HashMap<TypeId, Box<dyn Any>>);

impl AnyMap {
    pub fn insert<T: Any>(&mut self, value: T) -> Option<T> {
        self.0
            .insert(TypeId::of::<T>(), Box::new(value))
            .map(|boxed| *boxed.downcast().unwrap())
    }
}
