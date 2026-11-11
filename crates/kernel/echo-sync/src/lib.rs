#![no_std]
#![forbid(unsafe_code)]

//! 同步原语（Q2 host 版）。
//!
//! 当前实现是单核、非阻塞的 `try_*` 原语；阻塞等待队列在 QEMU runtime 集成时接入。
//! 契约见 `docs/29-scheduler-sync.md`。

use core::cell::Cell;

/// 固定容量等待队列（单核）。
#[derive(Clone, Copy)]
pub struct WaitQueue<const N: usize> {
    items: [Option<u8>; N],
    len: usize,
}

impl<const N: usize> WaitQueue<N> {
    pub const fn new() -> Self {
        Self {
            items: [None; N],
            len: 0,
        }
    }

    pub fn push(&mut self, task: u8) -> bool {
        if self.len >= N || self.contains(task) {
            return false;
        }
        self.items[self.len] = Some(task);
        self.len += 1;
        true
    }

    pub fn pop(&mut self) -> Option<u8> {
        if self.len == 0 {
            return None;
        }
        let task = self.items[0].take();
        for index in 0..self.len - 1 {
            self.items[index] = self.items[index + 1];
        }
        self.items[self.len - 1] = None;
        self.len -= 1;
        task
    }

    pub fn remove(&mut self, task: u8) -> bool {
        let Some(position) = self.items.iter().position(|entry| *entry == Some(task)) else {
            return false;
        };
        for index in position..self.len - 1 {
            self.items[index] = self.items[index + 1];
        }
        self.items[self.len - 1] = None;
        self.len -= 1;
        true
    }

    pub fn contains(&self, task: u8) -> bool {
        self.items.contains(&Some(task))
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<const N: usize> Default for WaitQueue<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LockResult {
    Acquired,
    Waiting,
    QueueFull,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnlockResult {
    Released,
    Woken(u8),
    InvalidOwner,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaitResult {
    Acquired,
    Waiting,
    QueueFull,
}

/// 单核互斥锁：记录 owner，带固定等待队列。
pub struct Mutex {
    owner: Cell<Option<u8>>,
    waiters: Cell<WaitQueue<8>>,
}

impl Mutex {
    pub const fn new() -> Self {
        Self {
            owner: Cell::new(None),
            waiters: Cell::new(WaitQueue::new()),
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

    pub fn lock_or_wait(&self, task: u8) -> LockResult {
        if self.try_lock(task) {
            return LockResult::Acquired;
        }
        let mut waiters = self.waiters.get();
        if waiters.push(task) {
            self.waiters.set(waiters);
            LockResult::Waiting
        } else {
            LockResult::QueueFull
        }
    }

    pub fn unlock_and_wake(&self, task: u8) -> UnlockResult {
        if self.owner.get() != Some(task) {
            return UnlockResult::InvalidOwner;
        }
        let mut waiters = self.waiters.get();
        let next = waiters.pop();
        self.waiters.set(waiters);
        if let Some(next) = next {
            self.owner.set(Some(next));
            UnlockResult::Woken(next)
        } else {
            self.owner.set(None);
            UnlockResult::Released
        }
    }

    pub fn waiting(&self) -> usize {
        self.waiters.get().len()
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
    waiters: Cell<WaitQueue<8>>,
}

impl Semaphore {
    pub const fn new(initial: u32) -> Self {
        Self {
            count: Cell::new(initial),
            waiters: Cell::new(WaitQueue::new()),
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

    pub fn wait_or_queue(&self, task: u8) -> WaitResult {
        if self.try_wait() {
            return WaitResult::Acquired;
        }
        let mut waiters = self.waiters.get();
        if waiters.push(task) {
            self.waiters.set(waiters);
            WaitResult::Waiting
        } else {
            WaitResult::QueueFull
        }
    }

    /// 优先唤醒等待者；没有等待者时增加计数。
    pub fn post_and_wake(&self) -> Option<u8> {
        let mut waiters = self.waiters.get();
        let next = waiters.pop();
        self.waiters.set(waiters);
        if next.is_none() {
            self.post();
        }
        next
    }

    pub fn waiting(&self) -> usize {
        self.waiters.get().len()
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
