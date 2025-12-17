use std::{
    collections::HashMap,
    marker::PhantomData,
    sync::{Arc, RwLock},
};

use bevy_ecs::resource::Resource;

// 1. The ID (Handle)
// It's just a unique number. Efficient to copy.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Handle<T> {
    pub id: u64,
    marker: PhantomData<T>,
}

impl<T> Handle<T> {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            marker: PhantomData,
        }
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            marker: PhantomData,
        }
    }
}

// 2. The Storage (Bank)
// We use RwLock so we can read from multiple threads (Renderer) safely.
#[derive(Resource)]
pub struct Assets<T: Send + Sync + 'static> {
    storage: Arc<RwLock<HashMap<u64, T>>>,
    next_id: u64,
}

impl<T: Send + Sync + 'static> Default for Assets<T> {
    fn default() -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
            next_id: 0,
        }
    }
}

impl<T: Send + Sync + 'static> Assets<T> {
    pub fn add(&mut self, asset: T) -> Handle<T> {
        let id = self.next_id;
        self.next_id += 1;

        let mut map = self.storage.write().unwrap();
        map.insert(id, asset);

        Handle::new(id)
    }

    pub fn get(&self, handle: &Handle<T>) -> Option<T>
    where
        T: Clone,
    {
        // Simple get for now
        let map = self.storage.read().unwrap();
        map.get(&handle.id).cloned()
    }

    // For Renderer: Lock and read directly without cloning
    pub fn with_storage<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&HashMap<u64, T>) -> R,
    {
        let map = self.storage.read().unwrap();
        f(&map)
    }
}
