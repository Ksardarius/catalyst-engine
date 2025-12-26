use std::{collections::HashMap, marker::PhantomData, sync::{Arc, RwLock}};

use bevy_ecs::resource::Resource;
use uuid::Uuid;

// 1. The ID (Handle)
// It's just a unique number. Efficient to copy.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Handle<T> {
    pub id: Uuid,
    marker: PhantomData<T>,
}

impl<T> Handle<T> {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            marker: PhantomData,
        }
    }

    pub fn from_id(id: Uuid) -> Self {
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
    storage: Arc<RwLock<HashMap<Uuid, Arc<T>>>>,
}

impl<T: Send + Sync + 'static> Default for Assets<T> {
    fn default() -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl<T: Send + Sync + 'static> Assets<T> {
    pub fn add(&mut self, asset: T) -> Handle<T> {
        let handle = Handle::new();
        let mut map = self.storage.write().unwrap();
        map.insert(handle.id, Arc::new(asset));
        handle
    }

    pub fn insert(&self, handle: Handle<T>, asset: T) {
        self.insert_by_id(handle.id, asset);
    }

    pub fn insert_by_id(&self, id: Uuid, asset: T) {
        let mut map = self.storage.write().unwrap();
        map.insert(id, Arc::new(asset));
    }

    pub fn get(&self, handle: &Handle<T>) -> Option<Arc<T>> {
        let map = self.storage.read().unwrap();
        map.get(&handle.id).cloned()
    }
}

#[derive(Debug)]
pub struct Vertex {
    pub position: [f32; 3], // Flat lists are easier for generic loaders
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

#[derive(Debug)]
pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}