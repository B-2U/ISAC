use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LruVector<T> {
    capacity: usize,
    data: VecDeque<T>,
}

// impl<T> Deref for LruVector<T> {
//     type Target = VecDeque<T>;

//     fn deref(&self) -> &Self::Target {
//         &self.data
//     }
// }

// impl<T> DerefMut for LruVector<T> {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.data
//     }
// }

// impl<T> AsRef<VecDeque<T>> for LruVector<T> {
//     fn as_ref(&self) -> &VecDeque<T> {
//         &self.data
//     }
// }

// impl<T> AsMut<VecDeque<T>> for LruVector<T> {
//     fn as_mut(&mut self) -> &mut VecDeque<T> {
//         &mut self.data
//     }
// }

impl<T> LruVector<T>
where
    T: PartialEq,
{
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            data: VecDeque::with_capacity(capacity),
        }
    }

    pub fn get(&mut self, value: &T) -> Option<&T> {
        if let Some(pos) = self.data.iter().position(|x| x == value) {
            // Move the accessed item to the front (most recent)
            let item = self.data.remove(pos).unwrap();
            self.data.push_front(item);
            return self.data.front();
        }
        None
    }
    /// return the evicted element if its over capacity
    pub fn put(&mut self, value: T) -> Option<T> {
        // If the item is already in the vector, remove it first
        if let Some(pos) = self.data.iter().position(|x| x == &value) {
            self.data.remove(pos);
        }
        // Add the new item to the front (most recent)
        self.data.push_front(value);
        // If over capacity, remove the least recently used (last) item
        if self.data.len() > self.capacity {
            self.data.pop_back()
        } else {
            None
        }
    }

    pub fn contains(&self, value: &T) -> bool {
        self.data.contains(value)
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.data.iter()
    }
}
