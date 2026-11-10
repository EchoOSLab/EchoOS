#![no_std]
#![forbid(unsafe_code)]

//! host 测试用的确定性 `Arch` 实现。
//!
//! 时间只通过 [`MockArch::advance`] / [`MockArch::set`] 改变，不读墙钟。

#[cfg(test)]
extern crate std;

use core::cell::Cell;

use echo_arch_api::Arch;

/// 确定性 mock 时钟。
#[derive(Debug)]
pub struct MockArch {
    now_ns: Cell<u64>,
}

impl MockArch {
    /// 从 0 开始。
    pub const fn new() -> Self {
        Self {
            now_ns: Cell::new(0),
        }
    }

    /// 从指定时刻开始。
    pub const fn with_time(now_ns: u64) -> Self {
        Self {
            now_ns: Cell::new(now_ns),
        }
    }

    /// 前进 `ns` 纳秒；饱和到 `u64::MAX`，保证单调。
    pub fn advance(&self, ns: u64) {
        self.now_ns.set(self.now_ns.get().saturating_add(ns));
    }

    /// 直接设置时刻（用于回绕/边界测试）。
    pub fn set(&self, now_ns: u64) {
        self.now_ns.set(now_ns);
    }
}

impl Default for MockArch {
    fn default() -> Self {
        Self::new()
    }
}

impl Arch for MockArch {
    fn now_ns(&self) -> u64 {
        self.now_ns.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_at_zero() {
        let mock = MockArch::new();
        assert_eq!(mock.now_ns(), 0);
    }

    #[test]
    fn advance_accumulates() {
        let mock = MockArch::new();
        mock.advance(10);
        mock.advance(5);
        assert_eq!(mock.now_ns(), 15);
    }

    #[test]
    fn set_overrides() {
        let mock = MockArch::new();
        mock.set(42);
        assert_eq!(mock.now_ns(), 42);
    }

    #[test]
    fn advance_saturates() {
        let mock = MockArch::new();
        mock.advance(u64::MAX);
        mock.advance(1);
        assert_eq!(mock.now_ns(), u64::MAX);
    }

    #[test]
    fn two_mocks_are_independent() {
        let a = MockArch::new();
        let b = MockArch::new();
        a.advance(100);
        assert_eq!(a.now_ns(), 100);
        assert_eq!(b.now_ns(), 0);
    }
}
