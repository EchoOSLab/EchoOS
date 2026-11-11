#![no_std]
#![forbid(unsafe_code)]

//! 同步原语（Q2 host 版）。
//!
//! 当前实现是单核、非阻塞的 `try_*` 原语；阻塞等待队列在 QEMU runtime 集成时接入。
//! 契约见 `docs/29-scheduler-sync.md`。

use core::cell::Cell;

/// 单核互斥锁：记录 owner，不提供阻塞等待。
pub struct Mutex {
    owner: Cell<Option<u8>>,
}

impl Mutex {
    pub const fn new() -> Self {
        Self {
            owner: Cell::new(None),
        }
    }

    pub fn try_lock(&self, task: u8) -> bool {
        if self.owner.get().is_some() {
            return false;
        }
        self.owner.set(Some(task));
        true
    }

    pub fn unlock(&self, task: u8) -> bool {
        if self.owner.get() != Some(task) {
            return false;
        }
        self.owner.set(None);
        true
    }

    pub fn owner(&self) -> Option<u8> {
        self.owner.get()
    }

    pub fn is_locked(&self) -> bool {
        self.owner.get().is_some()
    }
}

impl Default for Mutex {
    fn default() -> Self {
        Self::new()
    }
}

/// 计数信号量：单核原子计数，不阻塞。
pub struct Semaphore {
    count: Cell<u32>,
}

impl Semaphore {
    pub const fn new(initial: u32) -> Self {
        Self {
            count: Cell::new(initial),
        }
    }

    pub fn try_wait(&self) -> bool {
        let count = self.count.get();
        if count == 0 {
            return false;
        }
        self.count.set(count - 1);
        true
    }

    /// 返回 post 之前的计数；溢出时饱和到 `u32::MAX`。
    pub fn post(&self) -> u32 {
        let previous = self.count.get();
        self.count.set(previous.saturating_add(1));
        previous
    }

    pub fn count(&self) -> u32 {
        self.count.get()
    }
}

impl Default for Semaphore {
    fn default() -> Self {
        Self::new(0)
    }
}

/// 事件位：等待/清除指定 bit 组合。
pub struct Event {
    bits: Cell<u32>,
}

impl Event {
    pub const fn new() -> Self {
        Self { bits: Cell::new(0) }
    }

    pub fn set(&self, mask: u32) {
        self.bits.set(self.bits.get() | mask);
    }

    pub fn clear(&self, mask: u32) {
        self.bits.set(self.bits.get() & !mask);
    }

    pub fn try_wait(&self, mask: u32) -> bool {
        self.bits.get() & mask == mask
    }

    pub fn take(&self, mask: u32) -> bool {
        if !self.try_wait(mask) {
            return false;
        }
        self.clear(mask);
        true
    }

    pub fn bits(&self) -> u32 {
        self.bits.get()
    }
}

impl Default for Event {
    fn default() -> Self {
        Self::new()
    }
}
