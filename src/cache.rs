use std::time::{Instant, Duration};
use std::hash::Hash;

use lru::LruCache;

#[cfg(test)]
use std::thread::sleep;

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


#[test]
fn test_1() {
    let mut cache: Cache<i32, i32> = Cache::new(2);
    let d = Duration::from_secs(10);

    cache.put(1, 4, d);

    assert_eq!(cache.get(&1), Some(&4));

    cache.put(2, 5, d);
    cache.put(3, 6, d);

    assert_eq!(cache.get(&1), None);
}

#[test]
fn test_2() {
    let mut cache: Cache<i32, i32> = Cache::new(2);
    let d = Duration::from_secs(10);

    cache.put(1, 4, d);
    cache.put(2, 5, d);

    assert_eq!(cache.get(&1), Some(&4));

    cache.put(3, 6, d);

    assert_eq!(cache.get(&1), Some(&4));
}

#[test]
fn test_3() {
    use std::thread::sleep;

    let mut cache: Cache<i32, i32> = Cache::new(1);
    let key = 10;
    let value = 20;

    cache.put(key, value, Duration::from_secs(6));

    sleep(Duration::from_secs(3));

    assert_eq!(cache.get(&key), Some(&value));

    sleep(Duration::from_secs(4));

    assert_eq!(cache.get(&key), None);
}

#[test]
fn test_4() {
    let mut cache: Cache<i32, i32> = Cache::new(1);
    let key = 10;
    let value = 20;

    cache.put(key, value, Duration::from_secs(2));

    sleep(Duration::from_secs(1));

    assert_eq!(cache.get(&key), Some(&value));

    sleep(Duration::from_secs(2));

    assert_eq!(cache.get_expired(&key), None);
    assert_eq!(cache.get_expired_fallback(&key), Some(&value));
}
