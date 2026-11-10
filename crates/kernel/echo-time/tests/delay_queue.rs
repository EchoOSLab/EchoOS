use echo_time::{DelayQueue, TimeError, deadline_after, is_due};

#[test]
fn inserts_in_deadline_order() {
    let mut queue = DelayQueue::<4>::new();
    queue.insert(300, 1).unwrap();
    queue.insert(100, 2).unwrap();
    queue.insert(200, 3).unwrap();

    assert_eq!(queue.next_deadline(), Some(100));
    assert_eq!(queue.pop_due(100), Some(2));
    assert_eq!(queue.pop_due(200), Some(3));
    assert_eq!(queue.pop_due(300), Some(1));
    assert_eq!(queue.pop_due(400), None);
}

#[test]
fn same_deadline_is_stable() {
    let mut queue = DelayQueue::<4>::new();
    queue.insert(100, 1).unwrap();
    queue.insert(100, 2).unwrap();
    assert_eq!(queue.pop_due(100), Some(1));
    assert_eq!(queue.pop_due(100), Some(2));
}

#[test]
fn before_deadline_returns_none() {
    let mut queue = DelayQueue::<4>::new();
    queue.insert(100, 1).unwrap();
    assert_eq!(queue.pop_due(99), None);
    assert_eq!(queue.len(), 1);
}

#[test]
fn duplicate_id_is_rejected() {
    let mut queue = DelayQueue::<4>::new();
    queue.insert(100, 7).unwrap();
    assert_eq!(queue.insert(200, 7), Err(TimeError::DuplicateId));
}

#[test]
fn capacity_is_enforced() {
    let mut queue = DelayQueue::<2>::new();
    queue.insert(100, 1).unwrap();
    queue.insert(200, 2).unwrap();
    assert_eq!(queue.insert(300, 3), Err(TimeError::CapacityExceeded));
}

#[test]
fn remove_by_id() {
    let mut queue = DelayQueue::<4>::new();
    queue.insert(100, 1).unwrap();
    queue.insert(200, 2).unwrap();
    assert!(queue.remove(1));
    assert_eq!(queue.len(), 1);
    assert_eq!(queue.next_deadline(), Some(200));
    assert!(!queue.remove(99));
}

#[test]
fn deadline_after_saturates() {
    assert_eq!(deadline_after(10, 5), 15);
    assert_eq!(deadline_after(u64::MAX - 1, 10), u64::MAX);
}

#[test]
fn due_boundary_is_inclusive() {
    assert!(!is_due(99, 100));
    assert!(is_due(100, 100));
    assert!(is_due(101, 100));
}

#[test]
fn clear_empties_queue() {
    let mut queue = DelayQueue::<4>::new();
    queue.insert(100, 1).unwrap();
    queue.clear();
    assert!(queue.is_empty());
    assert_eq!(queue.pop_due(1000), None);
}
