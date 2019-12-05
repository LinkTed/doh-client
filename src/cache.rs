use std::time::{Instant, Duration};
use std::hash::Hash;

use lru::LruCache;


pub struct Cache<K: Eq + Hash, V> {
    lru_cache: LruCache<K, (V, Instant)>
}

impl<K: Eq + Hash + Clone, V> Cache<K, V> {
    pub fn new(max_size: usize) -> Cache<K, V> {
        Cache { lru_cache: LruCache::new(max_size) }
    }

    pub fn get(&mut self, k: &K) -> Option<&V> {
        if let Some(v) = self.lru_cache.pop(k) {
            if v.1 > Instant::now() {
                self.lru_cache.put(k.clone(), v);
                return Some(&self.lru_cache.peek(k).unwrap().0);
            }
        }
        return None;
    }

    pub fn get_expired(&mut self, k: &K) -> Option<&V> {
        if let Some(v) = self.lru_cache.get(k) {
            if v.1 > Instant::now() {
                return Some(&v.0);
            }
        }

        return None;
    }

    pub fn get_expired_fallback(&mut self, k: &K) -> Option<&V> {
        if let Some(v) = self.lru_cache.get(k) {
            return Some(&v.0);
        }

        return None;
    }

    pub fn put(&mut self, k: K, v: V, d: Duration) {
        self.lru_cache.put(k, (v, Instant::now() + d));
    }
}

