use echo_conformance::mock::MockPlatform;
use echo_conformance::mock_small::SmallMockPlatform;
use echo_pal_api::{
    ChannelApi, MemoryApi, ObservabilityApi, PAL_VERSION, Platform, StaticTask, TaskApi, TaskError,
    TimeApi,
};

fn noop() {}

fn check_common<P: Platform>(platform: &P) {
    let caps = platform.capabilities();
    assert!(caps.static_tasks);
    assert!(!caps.dynamic_tasks);
    assert!(!caps.dynamic_memory);
    assert!(!caps.posix_partition);

    let now = platform.time().now_ns();
    assert_eq!(platform.time().now_ns(), now);

    assert_eq!(platform.task().task_count(), 0);
    platform
        .task()
        .declare(StaticTask::new("worker", 1, noop))
        .expect("first task must fit");
    assert_eq!(platform.task().task_count(), 1);

    platform.channel().try_send(7).expect("channel must accept");
    assert_eq!(platform.channel().try_recv(), Some(7));
    assert_eq!(platform.channel().try_recv(), None);

    let block = platform.memory().try_alloc(1).expect("block must fit");
    assert!(platform.memory().pool_used() > 0);
    platform.memory().free(block);
    assert_eq!(platform.memory().pool_used(), 0);

    platform.trace().trace_event(1, 2);
    assert_eq!(platform.trace().event_count(), 1);
}

#[test]
fn pal_version_is_frozen() {
    assert_eq!(PAL_VERSION, (0, 1, 0));
}

#[test]
fn two_backends_pass_same_conformance() {
    check_common(&MockPlatform::new());
    check_common(&SmallMockPlatform::new());
}

#[test]
fn capacities_differ_but_semantics_match() {
    let big = MockPlatform::new();
    let small = SmallMockPlatform::new();

    assert_eq!(big.channel().capacity(), 7);
    assert_eq!(small.channel().capacity(), 1);
    assert_eq!(big.memory().pool_capacity(), 256);
    assert_eq!(small.memory().pool_capacity(), 32);

    small.channel().try_send(1).expect("one slot");
    assert_eq!(small.channel().try_send(2), Err(2));
    assert_eq!(small.channel().try_recv(), Some(1));
}

#[test]
fn small_backend_enforces_its_limits() {
    let platform = SmallMockPlatform::new();

    platform
        .task()
        .declare(StaticTask::new("a", 1, noop))
        .unwrap();
    platform
        .task()
        .declare(StaticTask::new("b", 1, noop))
        .unwrap();
    assert_eq!(
        platform
            .task()
            .declare(StaticTask::new("c", 1, noop))
            .unwrap_err(),
        TaskError::CapacityExceeded
    );

    for value in 0..8u32 {
        platform.trace().trace_event(1, value);
    }
    platform.trace().trace_event(1, 99);
    assert_eq!(platform.trace().event_count(), 8);
    assert_eq!(platform.trace().dropped_events(), 1);

    for _ in 0..4 {
        platform.memory().try_alloc(8).expect("four blocks");
    }
    assert!(platform.memory().try_alloc(8).is_none());
}

#[test]
fn shared_contract_passes_both_backends() {
    assert_eq!(
        echo_conformance::contract::check_common(&MockPlatform::new()),
        Ok(())
    );
    assert_eq!(
        echo_conformance::contract::check_common(&SmallMockPlatform::new()),
        Ok(())
    );
}
