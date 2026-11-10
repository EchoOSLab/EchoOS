use echo_conformance::mock::{
    CHANNEL_CAPACITY, MAX_TASKS, MockPlatform, POOL_BLOCK_SIZE, POOL_BLOCKS, TRACE_CAPACITY,
};
use echo_pal_api::{
    ChannelApi, MemoryApi, ObservabilityApi, Platform, PoolBlock, StaticTask, TaskApi, TaskError,
    TimeApi,
};

fn noop() {}

/// 只依赖 PAL 契约的通用验收：任何后端实现都应通过。
fn exercise_contract<P: Platform>(platform: &P) {
    let caps = platform.capabilities();
    assert!(caps.static_tasks);
    assert!(!caps.dynamic_tasks);
    assert!(!caps.dynamic_memory);
    assert!(!caps.posix_partition);

    // Time
    let t0 = platform.time().now_ns();
    assert_eq!(platform.time().now_ns(), t0);

    // Task（静态声明）
    assert_eq!(platform.task().task_count(), 0);
    platform
        .task()
        .declare(StaticTask::new("worker", 1, noop))
        .expect("first declaration must fit");
    assert_eq!(platform.task().task_count(), 1);

    // Channel
    platform.channel().try_send(7).expect("channel must accept");
    assert_eq!(platform.channel().try_recv(), Some(7));
    assert_eq!(platform.channel().try_recv(), None);

    // Memory（静态池）
    let block = platform.memory().try_alloc(16).expect("block must fit");
    assert_eq!(platform.memory().pool_used(), 16);
    platform.memory().free(block);
    assert_eq!(platform.memory().pool_used(), 0);

    // Observability
    platform.trace().trace_event(1, 2);
    assert_eq!(platform.trace().event_count(), 1);
}

#[test]
fn mock_platform_reports_micro_capabilities() {
    let platform = MockPlatform::new();
    let caps = platform.capabilities();
    assert!(caps.static_tasks);
    assert!(!caps.dynamic_tasks);
    assert!(!caps.blocking_sleep);
    assert!(!caps.dynamic_memory);
    assert!(caps.channels);
    assert!(caps.trace);
    assert!(!caps.posix_partition);
}

#[test]
fn generic_contract_is_satisfied() {
    let platform = MockPlatform::new();
    exercise_contract(&platform);
}

#[test]
fn task_table_is_bounded() {
    let platform = MockPlatform::new();
    for index in 0..MAX_TASKS {
        platform
            .task()
            .declare(StaticTask::new("task", 1, noop))
            .unwrap_or_else(|_| panic!("task {index} must fit"));
    }
    assert_eq!(
        platform
            .task()
            .declare(StaticTask::new("overflow", 1, noop))
            .unwrap_err(),
        TaskError::CapacityExceeded
    );
}

#[test]
fn channel_is_bounded_and_fifo() {
    let platform = MockPlatform::new();
    let capacity = platform.channel().capacity();
    assert_eq!(capacity, CHANNEL_CAPACITY - 1);

    for value in 0..capacity as u32 {
        platform.channel().try_send(value).expect("must fit");
    }
    assert_eq!(platform.channel().try_send(99), Err(99));

    for value in 0..capacity as u32 {
        assert_eq!(platform.channel().try_recv(), Some(value));
    }
    assert_eq!(platform.channel().try_recv(), None);
}

#[test]
fn trace_records_and_reports_drops() {
    let platform = MockPlatform::new();
    for value in 0..TRACE_CAPACITY as u32 {
        platform.trace().trace_event(1, value);
    }
    assert_eq!(platform.trace().event_count(), TRACE_CAPACITY);
    assert_eq!(platform.trace().dropped_events(), 0);

    platform.trace().trace_event(1, 0);
    assert_eq!(platform.trace().event_count(), TRACE_CAPACITY);
    assert_eq!(platform.trace().dropped_events(), 1);

    assert_eq!(platform.event(0), Some(1 << 16));
}

#[test]
fn memory_pool_exhausts_and_reuses() {
    let platform = MockPlatform::new();
    assert_eq!(
        platform.memory().pool_capacity(),
        POOL_BLOCKS * POOL_BLOCK_SIZE
    );

    let mut blocks = [PoolBlock { index: 0, len: 0 }; POOL_BLOCKS];
    for slot in blocks.iter_mut() {
        *slot = platform
            .memory()
            .try_alloc(POOL_BLOCK_SIZE)
            .expect("must fit");
    }
    assert_eq!(platform.memory().pool_used(), POOL_BLOCKS * POOL_BLOCK_SIZE);
    assert!(platform.memory().try_alloc(POOL_BLOCK_SIZE).is_none());

    platform.memory().free(blocks[0]);
    assert_eq!(
        platform.memory().pool_used(),
        (POOL_BLOCKS - 1) * POOL_BLOCK_SIZE
    );

    let reused = platform.memory().try_alloc(POOL_BLOCK_SIZE).expect("reuse");
    platform.memory().free(reused);
    for block in blocks.iter().skip(1) {
        platform.memory().free(*block);
    }
    assert_eq!(platform.memory().pool_used(), 0);

    assert!(platform.memory().try_alloc(0).is_none());
    assert!(platform.memory().try_alloc(POOL_BLOCK_SIZE + 1).is_none());
}

#[test]
fn time_only_advances_explicitly() {
    let platform = MockPlatform::new();
    assert_eq!(platform.time().now_ns(), 0);
    platform.advance_time(100);
    assert_eq!(platform.time().now_ns(), 100);
    platform.advance_time(0);
    assert_eq!(platform.time().now_ns(), 100);
}
