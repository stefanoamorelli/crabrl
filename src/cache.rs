use dashmap::DashMap;
use std::sync::Arc;
use std::hash::Hash;

pub struct LockFreeCache<K, V> {
    map: Arc<DashMap<K, V>>,
    capacity: usize,
}

impl<K, V> LockFreeCache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    pub fn new(capacity: usize) -> Self {
        Self {
            map: Arc::new(DashMap::with_capacity(capacity)),
            capacity,
        }
    }

    #[inline(always)]
    pub fn get(&self, key: &K) -> Option<V> {
        self.map.get(key).map(|v| v.clone())
    }

    #[inline(always)]
    pub fn insert(&self, key: K, value: V) {
        if self.map.len() >= self.capacity {
            if let Some(entry) = self.map.iter().next() {
                let k = entry.key().clone();
                drop(entry);
                self.map.remove(&k);
            }
        }
        self.map.insert(key, value);
    }

    #[inline(always)]
    pub fn contains(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    pub fn clear(&self) {
        self.map.clear();
    }
}
