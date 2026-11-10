#![no_std]
#![forbid(unsafe_code)]

//! 有界 SPSC 邮箱（单生产者 / 单消费者）。
//!
//! 设计契约见 `docs/13-ipc-and-three-node.md`。
//!
//! v0 范围：
//! - 载荷固定为 `u32`（typed payload 在 M4 再引入）；
//! - 非阻塞 `try_send` / `try_recv`；
//! - 环形数组保留一个空槽区分满/空，因此可用容量 = `N - 1`；
//! - 全部用 `AtomicU32` 实现，**零 unsafe**，可安全放进 `static`。

use core::sync::atomic::{AtomicU32, Ordering};

/// 有界 SPSC 邮箱。
///
/// `N` 是底层槽位数；可用容量为 `N - 1`（保留一个槽位区分满/空）。
pub struct Mailbox<const N: usize> {
    buffer: [AtomicU32; N],
    head: AtomicU32,
    tail: AtomicU32,
}

impl<const N: usize> Mailbox<N> {
    /// 创建空邮箱。
    ///
    /// `N` 必须大于 0；`N = 0` 会在使用时 panic（编译期常量上下文可提前失败）。
    pub const fn new() -> Self {
        assert!(N > 0);
        Self {
            buffer: [const { AtomicU32::new(0) }; N],
            head: AtomicU32::new(0),
            tail: AtomicU32::new(0),
        }
    }

    /// 可用容量（`N - 1`）。
    pub const fn capacity(&self) -> usize {
        N - 1
    }

    /// 当前元素个数（建议值；并发下只保证单侧观察一致）。
    pub fn len(&self) -> usize {
        let head = self.head.load(Ordering::Acquire) as usize;
        let tail = self.tail.load(Ordering::Acquire) as usize;
        if head >= tail {
            head - tail
        } else {
            head + N - tail
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn is_full(&self) -> bool {
        self.len() == self.capacity()
    }

    /// 非阻塞发送；满时把值原样返回。
    pub fn try_send(&self, value: u32) -> Result<(), u32> {
        let head = self.head.load(Ordering::Relaxed) as usize;
        let tail = self.tail.load(Ordering::Acquire) as usize;
        let next = (head + 1) % N;

        if next == tail {
            return Err(value);
        }

        self.buffer[head].store(value, Ordering::Relaxed);
        self.head.store(next as u32, Ordering::Release);
        Ok(())
    }

    /// 非阻塞接收；空时返回 `None`。
    pub fn try_recv(&self) -> Option<u32> {
        let tail = self.tail.load(Ordering::Relaxed) as usize;
        let head = self.head.load(Ordering::Acquire) as usize;

        if tail == head {
            return None;
        }

        let value = self.buffer[tail].load(Ordering::Relaxed);
        self.tail.store(((tail + 1) % N) as u32, Ordering::Release);
        Some(value)
    }

    /// 仅测试用：清空（要求没有并发生产者/消费者）。
    #[cfg(test)]
    pub(crate) fn clear(&self) {
        self.head.store(0, Ordering::Relaxed);
        self.tail.store(0, Ordering::Relaxed);
    }
}

impl<const N: usize> Default for Mailbox<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_empty() {
        let mailbox = Mailbox::<4>::new();
        assert!(mailbox.is_empty());
        assert!(!mailbox.is_full());
        assert_eq!(mailbox.len(), 0);
    }

    #[test]
    fn capacity_is_n_minus_one() {
        assert_eq!(Mailbox::<2>::new().capacity(), 1);
        assert_eq!(Mailbox::<8>::new().capacity(), 7);
    }

    #[test]
    fn fifo_order_is_preserved() {
        let mailbox = Mailbox::<4>::new();
        mailbox.try_send(1).unwrap();
        mailbox.try_send(2).unwrap();
        mailbox.try_send(3).unwrap();
        assert_eq!(mailbox.try_recv(), Some(1));
        assert_eq!(mailbox.try_recv(), Some(2));
        assert_eq!(mailbox.try_recv(), Some(3));
        assert_eq!(mailbox.try_recv(), None);
    }

    #[test]
    fn full_returns_the_value() {
        let mailbox = Mailbox::<4>::new();
        mailbox.try_send(1).unwrap();
        mailbox.try_send(2).unwrap();
        mailbox.try_send(3).unwrap();
        assert_eq!(mailbox.try_send(4), Err(4));
        assert!(mailbox.is_full());
    }

    #[test]
    fn empty_returns_none() {
        let mailbox = Mailbox::<4>::new();
        assert_eq!(mailbox.try_recv(), None);
    }

    #[test]
    fn wraparound_preserves_order() {
        let mailbox = Mailbox::<4>::new();
        for value in 0..20u32 {
            mailbox.try_send(value).unwrap();
            assert_eq!(mailbox.try_recv(), Some(value));
        }
        assert!(mailbox.is_empty());
    }

    #[test]
    fn len_tracks_send_and_recv() {
        let mailbox = Mailbox::<4>::new();
        mailbox.try_send(7).unwrap();
        mailbox.try_send(8).unwrap();
        assert_eq!(mailbox.len(), 2);
        mailbox.try_recv();
        assert_eq!(mailbox.len(), 1);
    }

    #[test]
    fn clear_resets_indices() {
        let mailbox = Mailbox::<4>::new();
        mailbox.try_send(1).unwrap();
        mailbox.clear();
        assert!(mailbox.is_empty());
        assert_eq!(mailbox.try_recv(), None);
    }

    #[test]
    fn mailbox_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<Mailbox<8>>();
    }
}
