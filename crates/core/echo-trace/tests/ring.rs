use echo_trace::{Event, EventRing};

fn event(id: u16) -> Event {
    Event {
        id,
        value: u32::from(id),
        timestamp_ns: u64::from(id) * 10,
    }
}

#[test]
fn fifo_order() {
    let mut ring = EventRing::<4>::new();
    assert!(ring.push(event(1)));
    assert!(ring.push(event(2)));
    assert_eq!(ring.pop(), Some(event(1)));
    assert_eq!(ring.pop(), Some(event(2)));
    assert_eq!(ring.pop(), None);
}

#[test]
fn full_ring_drops() {
    let mut ring = EventRing::<2>::new();
    assert!(ring.push(event(1)));
    assert!(ring.push(event(2)));
    assert!(!ring.push(event(3)));
    assert_eq!(ring.dropped(), 1);
    assert_eq!(ring.len(), 2);
}

#[test]
fn wraps_around() {
    let mut ring = EventRing::<2>::new();
    ring.push(event(1));
    ring.push(event(2));
    ring.pop();
    assert!(ring.push(event(3)));
    assert_eq!(ring.pop(), Some(event(2)));
    assert_eq!(ring.pop(), Some(event(3)));
}
