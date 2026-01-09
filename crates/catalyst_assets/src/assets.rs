use std::{cmp::Ordering, marker::PhantomData};
use flecs_ecs::prelude::*;
use std::hash::{Hash, Hasher};
use uuid::Uuid;

use crate::asset_events::AssetLookup;

// 1. The ID (Handle)
// It's just a unique number. Efficient to copy.
#[derive(Debug)]
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

    pub fn try_get_entity<'a>(&self, world: &'a World) -> Option<EntityView<'a>> {
        world.try_get::<&AssetLookup>(|lookup| {
            if let Some(&entity_id) = lookup.map.get(&self.id) {
                let entity = world.entity_from_id(entity_id);
                Some(entity)
            } else {
                None
            }
        }).flatten()
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

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> Eq for Handle<T> {}

// 4. Implement Hash manually
// Crucial for using Handle in HashMaps
impl<T> Hash for Handle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

// 1. Implement Ord (Total ordering)
// This is used by .sort(), .order_by(), and BTreeMap
impl<T> Ord for Handle<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        // We only compare the ID, completely ignoring the generic marker
        self.id.cmp(&other.id)
    }
}

// 2. Implement PartialOrd
// This is used for <, >, <=, >= operators
impl<T> PartialOrd for Handle<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Just delegate to the Ord implementation for consistency
        Some(self.cmp(other))
    }
}

#[derive(Debug)]
pub struct Vertex {
    pub position: [f32; 3], // Flat lists are easier for generic loaders
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

#[derive(Component, Debug)]
pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}