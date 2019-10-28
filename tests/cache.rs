use std::thread::sleep;
use std::time::Duration;
use doh_client::Cache;


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
