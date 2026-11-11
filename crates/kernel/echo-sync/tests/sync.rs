use echo_sync::{Event, LockResult, Mutex, Semaphore, UnlockResult, WaitQueue, WaitResult};

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

#[test]
fn mutex_wait_queue_handoff() {
    let mutex = Mutex::new();
    assert!(mutex.try_lock(1));
    assert_eq!(mutex.lock_or_wait(2), LockResult::Waiting);
    assert_eq!(mutex.waiting(), 1);
    assert_eq!(mutex.unlock_and_wake(1), UnlockResult::Woken(2));
    assert_eq!(mutex.owner(), Some(2));
    assert_eq!(mutex.waiting(), 0);
    assert_eq!(mutex.unlock_and_wake(2), UnlockResult::Released);
}

#[test]
fn wait_queue_capacity_and_remove() {
    let mut queue = WaitQueue::<2>::new();
    assert!(queue.push(1));
    assert!(!queue.push(1));
    assert!(queue.push(2));
    assert!(!queue.push(3));
    assert!(queue.remove(1));
    assert_eq!(queue.len(), 1);
    assert_eq!(queue.pop(), Some(2));
    assert!(queue.is_empty());
}

#[test]
fn semaphore_queues_and_wakes() {
    let semaphore = Semaphore::new(0);
    assert_eq!(semaphore.wait_or_queue(7), WaitResult::Waiting);
    assert_eq!(semaphore.waiting(), 1);
    assert_eq!(semaphore.post_and_wake(), Some(7));
    assert_eq!(semaphore.waiting(), 0);
    assert_eq!(semaphore.post_and_wake(), None);
    assert_eq!(semaphore.count(), 1);
}
