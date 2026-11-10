use core::cell::Cell;

use echo_arch_api::Arch;
use echo_arch_mock::MockArch;
use echo_ipc::Mailbox;
use echo_pal_api::{
    Capabilities, ChannelApi, MemoryApi, ObservabilityApi, Platform, PoolBlock, StaticTask,
    TaskApi, TaskError, TimeApi,
};

/// 小容量后端的静态任务上限。
pub const MAX_TASKS: usize = 2;
/// 小容量后端的 trace 容量。
pub const TRACE_CAPACITY: usize = 8;
/// 小容量后端的单块大小（字节）。
pub const POOL_BLOCK_SIZE: usize = 8;
/// 小容量后端的池块数。
pub const POOL_BLOCKS: usize = 4;
/// 小容量后端的通道槽位数（可用容量 = 1）。
pub const CHANNEL_CAPACITY: usize = 2;

/// PAL 的第二个 mock 后端：容量更小，语义相同。
///
/// 用于验证 conformance 只依赖契约，不依赖某个具体后端的容量。
pub struct SmallMockPlatform {
    arch: MockArch,
    tasks: Cell<usize>,
    pool_bitmap: Cell<u8>,
    trace: [Cell<u32>; TRACE_CAPACITY],
    trace_len: Cell<usize>,
    trace_dropped: Cell<usize>,
    channel: Mailbox<CHANNEL_CAPACITY>,
}

impl SmallMockPlatform {
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

    pub fn advance_time(&self, ns: u64) {
        self.arch.advance(ns);
    }

    pub fn event(&self, index: usize) -> Option<u32> {
        if index < self.trace_len.get() {
            Some(self.trace[index].get())
        } else {
            None
        }
    }
}

impl Default for SmallMockPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl Platform for SmallMockPlatform {
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

impl TaskApi for SmallMockPlatform {
    type TaskId = u32;

    fn current(&self) -> Self::TaskId {
        0
    }

    fn task_count(&self) -> usize {
        self.tasks.get()
    }

    fn yield_now(&self) {}

    fn declare(&self, _task: StaticTask) -> Result<Self::TaskId, TaskError> {
        let count = self.tasks.get();
        if count >= MAX_TASKS {
            return Err(TaskError::CapacityExceeded);
        }
        self.tasks.set(count + 1);
        Ok(count as u32)
    }
}

impl TimeApi for SmallMockPlatform {
    fn now_ns(&self) -> u64 {
        self.arch.now_ns()
    }
}

impl MemoryApi for SmallMockPlatform {
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
            let bit = 1u8 << index;
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
            let bit = 1u8 << usize::from(block.index);
            self.pool_bitmap.set(self.pool_bitmap.get() & !bit);
        }
    }
}

impl ChannelApi for SmallMockPlatform {
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

impl ObservabilityApi for SmallMockPlatform {
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
