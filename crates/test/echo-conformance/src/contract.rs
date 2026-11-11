//! 可复用的 PAL 契约检查（host Mock 与 QEMU 后端共用）。
//!
//! 返回 `Err(&'static str)` 而不是 panic，便于裸机端输出失败原因。

use echo_pal_api::{
    ChannelApi, MemoryApi, ObservabilityApi, Platform, StaticTask, TaskApi, TimeApi,
};

fn noop() {}

pub fn check_common<P: Platform>(platform: &P) -> Result<(), &'static str> {
    let caps = platform.capabilities();
    if !caps.static_tasks
        || caps.dynamic_tasks
        || caps.dynamic_memory
        || !caps.channels
        || !caps.trace
        || caps.posix_partition
    {
        return Err("capabilities");
    }

    if platform.task().task_count() != 0 {
        return Err("initial task count");
    }
    platform
        .task()
        .declare(StaticTask::new("conformance", 1, noop))
        .map_err(|_| "declare")?;
    if platform.task().task_count() != 1 {
        return Err("task count after declare");
    }

    let first = platform.time().now_ns();
    let second = platform.time().now_ns();
    if second < first {
        return Err("time went backwards");
    }

    if platform.channel().capacity() == 0 {
        return Err("channel capacity");
    }
    platform.channel().try_send(0xA5A5).map_err(|_| "send")?;
    if platform.channel().try_recv() != Some(0xA5A5) {
        return Err("recv");
    }
    if platform.channel().try_recv().is_some() {
        return Err("recv empty");
    }

    let block = platform.memory().try_alloc(1).ok_or("alloc")?;
    if platform.memory().pool_used() == 0 {
        return Err("pool used");
    }
    platform.memory().free(block);
    platform.memory().free(block); // 幂等

    let before = platform.trace().event_count();
    platform.trace().trace_event(1, 2);
    if platform.trace().event_count() != before + 1 {
        return Err("trace event");
    }

    Ok(())
}
