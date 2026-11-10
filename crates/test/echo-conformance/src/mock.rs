use core::cell::Cell;

use echo_arch_api::Arch;
use echo_arch_mock::MockArch;
use echo_ipc::Mailbox;
use echo_pal_api::{
    Capabilities, ChannelApi, MemoryApi, ObservabilityApi, Platform, PoolBlock, StaticTask,
    TaskApi, TaskError, TimeApi,
};

/// 静态任务表容量。
pub const MAX_TASKS: usize = 8;
/// trace 环形缓冲容量。
pub const TRACE_CAPACITY: usize = 64;
/// 静态池单块大小（字节）。
pub const POOL_BLOCK_SIZE: usize = 16;
/// 静态池块数。
pub const POOL_BLOCKS: usize = 16;
/// 通道槽位数（可用容量 = 7）。
pub const CHANNEL_CAPACITY: usize = 8;

/// PAL 的 host mock 后端。
///
/// 五个能力域都由本结构自身实现（`type Task = Self` 等），
/// 因此 conformance 测试可以通过 `P: Platform` 泛型只依赖契约。
pub struct MockPlatform {
    arch: MockArch,
    tasks: Cell<usize>,
    pool_bitmap: Cell<u16>,
    trace: [Cell<u32>; TRACE_CAPACITY],
    trace_len: Cell<usize>,
    trace_dropped: Cell<usize>,
    channel: Mailbox<CHANNEL_CAPACITY>,
}

impl MockPlatform {
    pub const fn new() -> Self {
        Self {
            arch: MockArch::new(),
            tasks: Cell::new(0),
            pool_bitmap: Cell::new(0),
            trace: [const { Cell::new(0u32) }; TRACE_CAPACITY],
            trace_len: Cell::new(0),
            trace_dropped: Cell::new(0),
            channel: Mailbox::new(),
        }
    }

    /// 仅 mock：显式推进时间。
    pub fn advance_time(&self, ns: u64) {
        self.arch.advance(ns);
    }

    /// 读取第 `index` 个 trace 事件（编码为 `id << 16 | value`）。
    pub fn event(&self, index: usize) -> Option<u32> {
        if index < self.trace_len.get() {
            Some(self.trace[index].get())
        } else {
            None
        }
    }
}

impl Default for MockPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl Platform for MockPlatform {
    type Task = Self;
    type Time = Self;
    type Memory = Self;
    type Channel = Self;
    type Trace = Self;

    fn capabilities(&self) -> Capabilities {
        Capabilities::micro()
    }

    fn task(&self) -> &Self::Task {
        self
    }

    fn time(&self) -> &Self::Time {
        self
    }

    fn memory(&self) -> &Self::Memory {
        self
    }

    fn channel(&self) -> &Self::Channel {
        self
    }

    fn trace(&self) -> &Self::Trace {
        self
    }
}

impl TaskApi for MockPlatform {
    type TaskId = u32;

    fn current(&self) -> Self::TaskId {
        0
    }

    fn task_count(&self) -> usize {
        self.tasks.get()
    }

    fn yield_now(&self) {
        // mock：无调度副作用
    }

    fn declare(&self, _task: StaticTask) -> Result<Self::TaskId, TaskError> {
        let count = self.tasks.get();
        if count >= MAX_TASKS {
            return Err(TaskError::CapacityExceeded);
        }
        self.tasks.set(count + 1);
        Ok(count as u32)
    }
}

impl TimeApi for MockPlatform {
    fn now_ns(&self) -> u64 {
        self.arch.now_ns()
    }
}

impl MemoryApi for MockPlatform {
    fn pool_capacity(&self) -> usize {
        POOL_BLOCKS * POOL_BLOCK_SIZE
    }

    fn pool_used(&self) -> usize {
        self.pool_bitmap.get().count_ones() as usize * POOL_BLOCK_SIZE
    }

    fn try_alloc(&self, len: usize) -> Option<PoolBlock> {
        if len == 0 || len > POOL_BLOCK_SIZE {
            return None;
        }

        let bitmap = self.pool_bitmap.get();
        for index in 0..POOL_BLOCKS {
            let bit = 1u16 << index;
            if bitmap & bit == 0 {
                self.pool_bitmap.set(bitmap | bit);
                return Some(PoolBlock {
                    index: index as u16,
                    len: POOL_BLOCK_SIZE as u16,
                });
            }
        }
        None
    }

    fn free(&self, block: PoolBlock) {
        if block.index < POOL_BLOCKS as u16 {
            let bit = 1u16 << usize::from(block.index);
            self.pool_bitmap.set(self.pool_bitmap.get() & !bit);
        }
    }
}

impl ChannelApi for MockPlatform {
    fn capacity(&self) -> usize {
        self.channel.capacity()
    }

    fn try_send(&self, value: u32) -> Result<(), u32> {
        self.channel.try_send(value)
    }

    fn try_recv(&self) -> Option<u32> {
        self.channel.try_recv()
    }
}

impl ObservabilityApi for MockPlatform {
    fn trace_event(&self, id: u16, value: u32) {
        let len = self.trace_len.get();
        if len < TRACE_CAPACITY {
            self.trace[len].set((u32::from(id) << 16) | (value & 0xFFFF));
            self.trace_len.set(len + 1);
        } else {
            self.trace_dropped.set(self.trace_dropped.get() + 1);
        }
    }

    fn event_count(&self) -> usize {
        self.trace_len.get()
    }

    fn dropped_events(&self) -> usize {
        self.trace_dropped.get()
    }
}
