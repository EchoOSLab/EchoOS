use echo_sync::{Event, Mutex, Semaphore};

#[test]
fn mutex_is_exclusive() {
    let mutex = Mutex::new();
    assert!(mutex.try_lock(1));
    assert!(!mutex.try_lock(2));
    assert_eq!(mutex.owner(), Some(1));
    assert!(!mutex.unlock(2));
    assert!(mutex.unlock(1));
    assert!(!mutex.is_locked());
    assert!(mutex.try_lock(2));
}

#[test]
fn semaphore_counts_and_saturates() {
    let semaphore = Semaphore::new(0);
    assert!(!semaphore.try_wait());
    assert_eq!(semaphore.post(), 0);
    assert_eq!(semaphore.post(), 1);
    assert_eq!(semaphore.count(), 2);
    assert!(semaphore.try_wait());
    assert!(semaphore.try_wait());
    assert!(!semaphore.try_wait());
}

#[test]
fn event_waits_for_all_bits() {
    let event = Event::new();
    event.set(0b0101);
    assert!(event.try_wait(0b0001));
    assert!(!event.try_wait(0b0011));
    event.set(0b0010);
    assert!(event.try_wait(0b0011));
    assert!(event.take(0b0011));
    assert_eq!(event.bits(), 0b0100);
}
