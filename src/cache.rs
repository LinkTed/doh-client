use std::time::{Instant, Duration};
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex, MutexGuard};

#[cfg(test)]
use std::thread::sleep;


struct Entry<K: Eq + Hash, V> {
    next: Option<Arc<Mutex<Entry<K, V>>>>,
    prev: Option<Arc<Mutex<Entry<K, V>>>>,
    key: Arc<K>,
    value: Arc<V>,
    instant: Instant,
}

impl<K: Eq + Hash, V> Entry<K, V> {
    pub fn new(k: K, v: V, d: Duration) -> Entry<K, V> {
        Entry { next: None, prev: None, key: Arc::new(k), value: Arc::new(v), instant: Instant::now() + d }
    }
}

pub struct Cache<K: Eq + Hash, V> {
    hash_map: HashMap<Arc<K>, Arc<Mutex<Entry<K, V>>>>,
    head: Option<Arc<Mutex<Entry<K, V>>>>,
    tail: Option<Arc<Mutex<Entry<K, V>>>>,
    max_size: usize,
}

impl<K: Eq + Hash, V> Cache<K, V> {
    pub fn new(max_size: usize) -> Cache<K, V> {
        Cache { hash_map: HashMap::with_capacity(max_size), head: None, tail: None, max_size }
    }

    pub fn get(&mut self, k: &K) -> Option<Arc<V>> {
        let key = if let Some(entry) = self.hash_map.get(k) {
            let mut entry_mutex = entry.lock().unwrap();
            if entry_mutex.instant > Instant::now() {
                Cache::push_head(&mut self.head, &mut self.tail, &entry, &mut entry_mutex);
                return Some(entry_mutex.value.clone());
            } else {
                match entry_mutex.prev.take() {
                    Some(entry_prev) => {
                        let mut entry_prev_mutex = entry_prev.lock().unwrap();
                        match entry_mutex.next.take() {
                            Some(entry_next) => {
                                entry_prev_mutex.next.replace(entry_next.clone());
                                entry_next.lock().unwrap().prev.replace(entry_prev.clone());
                            }
                            None => {
                                entry_prev_mutex.next.take();
                                self.tail.replace(entry_prev.clone());
                            }
                        }
                    }
                    None => {
                        match entry_mutex.next.take() {
                            Some(entry_next) => {
                                entry_next.lock().unwrap().prev.take();
                                self.head.replace(entry_next);
                            }
                            None => {
                                self.head.take();
                                self.tail.take();
                            }
                        }
                    }
                }

                entry_mutex.key.clone()
            }
        } else {
            return None;
        };

        self.hash_map.remove(&key);
        None
    }

    pub fn get_expired(&mut self, k: &K) -> Option<Arc<V>> {
        if let Some(entry) = self.hash_map.get(k) {
            let mut entry_mutex = entry.lock().unwrap();
            if entry_mutex.instant > Instant::now() {
                Cache::push_head(&mut self.head, &mut self.tail, &entry, &mut entry_mutex);
                return Some(entry_mutex.value.clone());
            }
        }

        None
    }

    pub fn get_expired_fallback(&mut self, k: &K) -> Option<Arc<V>> {
        if let Some(entry) = self.hash_map.get(k) {
            let mut entry_mutex = entry.lock().unwrap();
            Cache::push_head(&mut self.head, &mut self.tail, &entry, &mut entry_mutex);
            return Some(entry_mutex.value.clone());
        }

        None
    }

    pub fn put(&mut self, k: K, v: V, d: Duration) {
        if self.max_size <= self.hash_map.len() {
            if let Some(tail_old) = self.tail.take() {
                let mut tail_old_mutex = tail_old.lock().unwrap();
                let tail_new = tail_old_mutex.prev.take();

                if let Some(tail_new) = tail_new {
                    tail_new.lock().unwrap().next.take();
                    self.tail.replace(tail_new);
                } else {
                    self.head.take();
                }

                self.hash_map.remove(&tail_old_mutex.key);
            } else {
                panic!("THAT SHOULD NOT HAPPEN")
            }
        }

        let entry_new = Arc::new(Mutex::new(Entry::new(k, v, d)));
        let mut entry_new_mutex = entry_new.lock().unwrap();

        match self.head.replace(entry_new.clone()) {
            Some(head_old) => {
                if head_old.lock().unwrap().prev.replace(entry_new.clone()).is_some() {
                    panic!("THAT SHOULD NOT HAPPEN")
                }

                entry_new_mutex.next.replace(head_old);
            }
            None => {
                if self.tail.replace(entry_new.clone()).is_some() {
                    panic!("THAT SHOULD NOT HAPPEN")
                }
            }
        }

        self.hash_map.insert(entry_new_mutex.key.clone(), entry_new.clone());
    }

    fn push_head(head: &mut Option<Arc<Mutex<Entry<K, V>>>>, tail: &mut Option<Arc<Mutex<Entry<K, V>>>>, entry: &Arc<Mutex<Entry<K, V>>>, entry_mutex: &mut MutexGuard<Entry<K, V>>) {
        match entry_mutex.prev.take() {
            Some(entry_prev) => {
                let mut entry_prev_mutex = entry_prev.lock().unwrap();
                match entry_mutex.next.take() {
                    Some(entry_next) => {
                        entry_prev_mutex.next.replace(entry_next.clone());

                        let mut entry_next_mutex = entry_next.lock().unwrap();
                        entry_next_mutex.prev.replace(entry_prev.clone());
                    }
                    None => {
                        entry_prev_mutex.next.take();
                        tail.replace(entry_prev.clone());
                    }
                }
            }
            None => return
        }

        if let Some(entry_old_head) = head.replace(entry.clone()) {
            {
                let mut entry_old_head_mutex = entry_old_head.lock().unwrap();
                entry_old_head_mutex.prev.replace(entry.clone());
            }

            entry_mutex.next.replace(entry_old_head);
        }
    }
}

#[test]
fn test_1() {
    let mut cache: Cache<i32, i32> = Cache::new(2);
    let d = Duration::from_secs(10);

    cache.put(1, 4, d);

    assert_eq!(cache.get(&1), Some(Arc::new(4)));

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

    assert_eq!(cache.get(&1), Some(Arc::new(4)));

    cache.put(3, 6, d);

    assert_eq!(cache.get(&1), Some(Arc::new(4)));
}

#[test]
fn test_3() {
    use std::thread::sleep;

    let mut cache: Cache<i32, i32> = Cache::new(1);
    let key = 10;
    let value = 20;

    cache.put(key, value, Duration::from_secs(6));

    sleep(Duration::from_secs(3));

    assert_eq!(cache.get(&key), Some(Arc::new(value)));

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

    assert_eq!(cache.get(&key), Some(Arc::new(value)));

    sleep(Duration::from_secs(2));

    assert_eq!(cache.get_expired(&key), None);
    assert_eq!(cache.get_expired_fallback(&key), Some(Arc::new(value)));
}

#[test]
fn get_test_1() {
    let mut cache: Cache<i32, i32> = Cache::new(3);
    cache.put(1, 4, Duration::from_secs(100));
    cache.put(2, 5, Duration::from_secs(100));
    cache.put(3, 6, Duration::from_secs(100));
    assert_eq!(cache.hash_map.len(), 3);

    let head_old = cache.head.clone();
    let tail_old = cache.tail.clone();

    assert_eq!(cache.get(&1), Some(Arc::new(4)));

    assert_eq!(cache.hash_map.len(), 3);
    let head = cache.head.unwrap();
    assert_eq!(Arc::ptr_eq(&head, &head_old.unwrap()), false);
    assert_eq!(Arc::ptr_eq(&head, &tail_old.unwrap()), true);
}

#[test]
fn get_test_2() {
    let mut cache: Cache<i32, i32> = Cache::new(3);
    cache.put(1, 4, Duration::from_secs(100));
    cache.put(2, 5, Duration::from_secs(100));
    cache.put(3, 6, Duration::from_secs(100));
    assert_eq!(cache.hash_map.len(), 3);

    let head_old = cache.head.clone();
    let tail_old = cache.tail.clone();

    assert_eq!(cache.get(&3), Some(Arc::new(6)));

    assert_eq!(cache.hash_map.len(), 3);
    assert_eq!(Arc::ptr_eq(&cache.head.unwrap(), &head_old.unwrap()), true);
    assert_eq!(Arc::ptr_eq(&cache.tail.unwrap(), &tail_old.unwrap()), true);
}

#[test]
fn get_test_3() {
    let mut cache: Cache<i32, i32> = Cache::new(3);
    cache.put(1, 4, Duration::from_secs(100));
    cache.put(2, 5, Duration::from_secs(0));
    cache.put(3, 6, Duration::from_secs(100));
    assert_eq!(cache.hash_map.len(), 3);

    let head_old = cache.head.clone();
    let tail_old = cache.tail.clone();

    sleep(Duration::from_secs(1));

    assert_eq!(cache.get(&2), None);

    assert_eq!(cache.hash_map.len(), 2);
    assert_eq!(Arc::ptr_eq(&cache.head.unwrap(), &head_old.unwrap()), true);
    assert_eq!(Arc::ptr_eq(&cache.tail.unwrap(), &tail_old.unwrap()), true);
}

#[test]
fn get_test_4() {
    let mut cache: Cache<i32, i32> = Cache::new(3);
    cache.put(1, 4, Duration::from_secs(0));
    cache.put(2, 5, Duration::from_secs(100));
    cache.put(3, 6, Duration::from_secs(100));
    assert_eq!(cache.hash_map.len(), 3);

    let head_old = cache.head.clone();
    let tail_old = cache.tail.clone();

    sleep(Duration::from_secs(1));

    assert_eq!(cache.get(&1), None);

    assert_eq!(cache.hash_map.len(), 2);
    assert_eq!(Arc::ptr_eq(&cache.head.unwrap(), &head_old.unwrap()), true);
    assert_eq!(Arc::ptr_eq(&cache.tail.unwrap(), &tail_old.unwrap()), false);
}

#[test]
fn get_test_5() {
    let mut cache: Cache<i32, i32> = Cache::new(3);
    cache.put(1, 4, Duration::from_secs(100));
    cache.put(2, 5, Duration::from_secs(100));
    cache.put(3, 6, Duration::from_secs(0));
    assert_eq!(cache.hash_map.len(), 3);

    let head_old = cache.head.clone();
    let tail_old = cache.tail.clone();

    sleep(Duration::from_secs(1));

    assert_eq!(cache.get(&3), None);

    assert_eq!(cache.hash_map.len(), 2);
    assert_eq!(Arc::ptr_eq(&cache.head.unwrap(), &head_old.unwrap()), false);
    assert_eq!(Arc::ptr_eq(&cache.tail.unwrap(), &tail_old.unwrap()), true);
}

#[test]
fn get_test_6() {
    let mut cache: Cache<i32, i32> = Cache::new(1);
    cache.put(1, 4, Duration::from_secs(0));
    assert_eq!(cache.hash_map.len(), 1);

    sleep(Duration::from_secs(1));

    assert_eq!(cache.get(&1), None);

    assert_eq!(cache.hash_map.len(), 0);
    assert_eq!(cache.head.is_none(), true);
    assert_eq!(cache.tail.is_none(), true);
}

#[test]
fn get_expired_1() {
    let mut cache: Cache<i32, i32> = Cache::new(2);
    cache.put(1, 3, Duration::from_secs(0));
    cache.put(2, 4, Duration::from_secs(100));
    assert_eq!(cache.hash_map.len(), 2);

    let head_old = cache.head.clone();
    let tail_old = cache.tail.clone();

    sleep(Duration::from_secs(1));

    assert_eq!(cache.get_expired(&1), None);

    assert_eq!(cache.hash_map.len(), 2);
    assert_eq!(Arc::ptr_eq(&cache.head.unwrap(), &head_old.unwrap()), true);
    assert_eq!(Arc::ptr_eq(&cache.tail.unwrap(), &tail_old.unwrap()), true);
}

#[test]
fn get_expired_2() {
    let mut cache: Cache<i32, i32> = Cache::new(2);
    cache.put(1, 3, Duration::from_secs(100));
    cache.put(2, 4, Duration::from_secs(100));
    assert_eq!(cache.hash_map.len(), 2);

    let head_old = cache.head.clone();
    let tail_old = cache.tail.clone();

    assert_eq!(cache.get_expired(&1), Some(Arc::new(3)));

    assert_eq!(cache.hash_map.len(), 2);
    assert_eq!(Arc::ptr_eq(&cache.head.unwrap(), &head_old.unwrap()), false);
    assert_eq!(Arc::ptr_eq(&cache.tail.unwrap(), &tail_old.unwrap()), false);
}

#[test]
fn get_expired_fallback_1() {
    let mut cache: Cache<i32, i32> = Cache::new(2);
    cache.put(1, 3, Duration::from_secs(0));
    cache.put(2, 4, Duration::from_secs(100));
    assert_eq!(cache.hash_map.len(), 2);

    let head_old = cache.head.clone();
    let tail_old = cache.tail.clone();

    sleep(Duration::from_secs(1));

    assert_eq!(cache.get_expired_fallback(&1), Some(Arc::new(3)));

    assert_eq!(cache.hash_map.len(), 2);
    assert_eq!(Arc::ptr_eq(&cache.head.unwrap(), &head_old.unwrap()), false);
    assert_eq!(Arc::ptr_eq(&cache.tail.unwrap(), &tail_old.unwrap()), false);
}

#[test]
fn get_expired_fallback_2() {
    let mut cache: Cache<i32, i32> = Cache::new(2);
    cache.put(1, 3, Duration::from_secs(100));
    cache.put(2, 4, Duration::from_secs(100));
    assert_eq!(cache.hash_map.len(), 2);

    let head_old = cache.head.clone();
    let tail_old = cache.tail.clone();

    assert_eq!(cache.get_expired_fallback(&1), Some(Arc::new(3)));

    assert_eq!(cache.hash_map.len(), 2);
    assert_eq!(Arc::ptr_eq(&cache.head.unwrap(), &head_old.unwrap()), false);
    assert_eq!(Arc::ptr_eq(&cache.tail.unwrap(), &tail_old.unwrap()), false);
}