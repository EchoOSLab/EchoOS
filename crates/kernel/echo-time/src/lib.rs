#![no_std]
#![forbid(unsafe_code)]

//! 时间域：单调时间辅助 + 延时队列（W8）。
//!
//! 设计契约见 `docs/19-time-and-bench.md`。
//!
//! - `deadline_after`：饱和加法，避免回绕/溢出；
//! - `DelayQueue`：固定容量、按 deadline 有序、静态存储；
//! - 时基本身由 `Arch`（MockArch / DWT）提供，本 crate 不做时钟。

/// 延时队列错误。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeError {
    CapacityExceeded,
    DuplicateId,
}

#[derive(Clone, Copy, Debug)]
struct TimerEntry {
    deadline_ns: u64,
    id: u16,
}

/// 固定容量的有序延时队列。
pub struct DelayQueue<const N: usize> {
    entries: [Option<TimerEntry>; N],
    count: usize,
}

impl<const N: usize> DelayQueue<N> {
    pub const fn new() -> Self {
        assert!(N > 0, "DelayQueue capacity must be > 0");
        Self {
            entries: [None; N],
            count: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// 插入一个定时器；相同 deadline 按插入顺序稳定排列。
    pub fn insert(&mut self, deadline_ns: u64, id: u16) -> Result<(), TimeError> {
        if self.count >= N {
            return Err(TimeError::CapacityExceeded);
        }
        if self.entries.iter().flatten().any(|entry| entry.id == id) {
            return Err(TimeError::DuplicateId);
        }

        let mut insert_at = self.count;
        for index in 0..self.count {
            if let Some(entry) = self.entries[index]
                && entry.deadline_ns > deadline_ns
            {
                insert_at = index;
                break;
            }
        }

        let mut index = self.count;
        while index > insert_at {
            self.entries[index] = self.entries[index - 1];
            index -= 1;
        }
        self.entries[insert_at] = Some(TimerEntry { deadline_ns, id });
        self.count += 1;
        Ok(())
    }

    pub fn next_deadline(&self) -> Option<u64> {
        self.entries[0].map(|entry| entry.deadline_ns)
    }

    /// 取出最早一个已到期定时器。
    pub fn pop_due(&mut self, now_ns: u64) -> Option<u16> {
        let first = self.entries[0]?;
        if first.deadline_ns > now_ns {
            return None;
        }

        for index in 0..self.count - 1 {
            self.entries[index] = self.entries[index + 1];
        }
        self.entries[self.count - 1] = None;
        self.count -= 1;
        Some(first.id)
    }

    /// 按 id 移除；返回是否找到。
    pub fn remove(&mut self, id: u16) -> bool {
        let Some(position) = self
            .entries
            .iter()
            .position(|entry| entry.is_some_and(|timer| timer.id == id))
        else {
            return false;
        };

        for index in position..self.count - 1 {
            self.entries[index] = self.entries[index + 1];
        }
        self.entries[self.count - 1] = None;
        self.count -= 1;
        true
    }

    pub fn clear(&mut self) {
        self.entries = [None; N];
        self.count = 0;
    }
}

impl<const N: usize> Default for DelayQueue<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// `now + delta`，饱和到 `u64::MAX`。
pub fn deadline_after(now_ns: u64, delta_ns: u64) -> u64 {
    now_ns.saturating_add(delta_ns)
}

/// 边界包含：`now >= deadline` 即到期。
pub fn is_due(now_ns: u64, deadline_ns: u64) -> bool {
    now_ns >= deadline_ns
}
