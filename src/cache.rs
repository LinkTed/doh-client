use std::hash::Hash;
use std::time::{Duration, Instant};

use lru::LruCache;

pub(crate) struct Cache<K: Eq + Hash, V> {
    lru_cache: LruCache<K, (V, Instant)>,
}

impl<K: Eq + Hash + Clone, V> Cache<K, V> {
    pub(crate) fn new(max_size: usize) -> Cache<K, V> {
        Cache {
            lru_cache: LruCache::new(max_size),
        }
    }

    pub(crate) fn get(&mut self, k: &K) -> Option<&mut V> {
        if let Some(v) = self.lru_cache.pop(k) {
            if v.1 > Instant::now() {
                self.lru_cache.put(k.clone(), v);
                return Some(&mut self.lru_cache.peek_mut(k).unwrap().0);
            }
        }
        None
    }

    pub(crate) fn get_expired(&mut self, k: &K) -> Option<&mut V> {
        if let Some(v) = self.lru_cache.get_mut(k) {
            if v.1 > Instant::now() {
                return Some(&mut v.0);
            }
        }
        None
    }

    pub(crate) fn get_expired_fallback(&mut self, k: &K) -> Option<&mut V> {
        if let Some(v) = self.lru_cache.get_mut(k) {
            return Some(&mut v.0);
        }
        None
    }

    pub(crate) fn put(&mut self, k: K, v: V, d: Duration) {
        self.lru_cache.put(k, (v, Instant::now() + d));
    }
}

#[cfg(test)]
mod tests {
    use std::thread::sleep;
    use std::time::Duration;

    use super::Cache;

    #[test]
    fn test_1() {
        let mut cache: Cache<i32, i32> = Cache::new(2);
        let d = Duration::from_secs(10);

        cache.put(1, 4, d);

        assert_eq!(cache.get(&1), Some(&mut 4));

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

        assert_eq!(cache.get(&1), Some(&mut 4));

        cache.put(3, 6, d);

        assert_eq!(cache.get(&1), Some(&mut 4));
    }

    #[test]
    fn test_3() {
        let mut cache: Cache<i32, i32> = Cache::new(1);
        let key = 10;
        let mut value = 20;

        cache.put(key, value, Duration::from_secs(6));

        sleep(Duration::from_secs(3));

        assert_eq!(cache.get(&key), Some(&mut value));

        sleep(Duration::from_secs(4));

        assert_eq!(cache.get(&key), None);
    }

    #[test]
    fn test_4() {
        let mut cache: Cache<i32, i32> = Cache::new(1);
        let key = 10;
        let mut value = 20;

        cache.put(key, value, Duration::from_secs(2));

        sleep(Duration::from_secs(1));

        assert_eq!(cache.get(&key), Some(&mut value));

        sleep(Duration::from_secs(2));

        assert_eq!(cache.get_expired(&key), None);
        assert_eq!(cache.get_expired_fallback(&key), Some(&mut value));
    }
}
