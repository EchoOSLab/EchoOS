#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]

//! Cortex-M 的 `Arch` 实现（W6/W8，DWT 时基）。
//!
//! 设计契约见 `docs/17-h7-bringup.md` 与 `docs/19-time-and-bench.md`。
//!
//! - 时基：DWT `CYCCNT`（周期计数器）+ 编译期 `CPU_HZ`；
//! - 回绕：32 位计数器每 `2^32` 周期回绕；W8 用 `epoch_ns` 累加回绕周期，
//!   保证 `now_ns` 单调；
//! - 该实现需要在真机上验证；QEMU 不保证 DWT 周期精度。

use core::cell::Cell;

use echo_arch_api::Arch;

#[cfg(target_arch = "arm")]
mod context;

const DEMCR: *mut u32 = 0xE000_EDFC as *mut u32;
const DWT_CTRL: *mut u32 = 0xE000_1000 as *mut u32;
const DWT_CYCCNT: *mut u32 = 0xE000_1004 as *mut u32;

const TRCENA: u32 = 1 << 24;
const CYCCNTENA: u32 = 1;

/// 周期数 → 纳秒（向下取整）。
///
/// `cpu_hz = 0` 时返回 0，避免除零；`cycles * 1e9` 在 u64 内不溢出
/// （最大 4.29e9 × 1e9 ≈ 4.29e18 < u64::MAX）。
pub fn cycles_to_ns(cycles: u32, cpu_hz: u32) -> u64 {
    if cpu_hz == 0 {
        return 0;
    }
    (u64::from(cycles) * 1_000_000_000) / u64::from(cpu_hz)
}

/// 一个 DWT 回绕周期对应的纳秒数（`2^32` 周期）。
pub fn wrap_period_ns(cpu_hz: u32) -> u64 {
    if cpu_hz == 0 {
        return 0;
    }
    ((1u64 << 32) * 1_000_000_000) / u64::from(cpu_hz)
}

/// 扩展回绕：若本次周期数小于上次，说明发生一次回绕，`epoch_ns` 增加一个周期。
pub fn extend_monotonic(last_cycles: u32, epoch_ns: u64, cycles: u32, cpu_hz: u32) -> (u32, u64) {
    let epoch = if cycles < last_cycles {
        epoch_ns.saturating_add(wrap_period_ns(cpu_hz))
    } else {
        epoch_ns
    };
    (cycles, epoch)
}

/// 基于 DWT 周期计数器的单调时钟。
pub struct DwtClock<const CPU_HZ: u32> {
    last_cycles: Cell<u32>,
    epoch_ns: Cell<u64>,
}

impl<const CPU_HZ: u32> DwtClock<CPU_HZ> {
    /// 使能 DWT 并清零周期计数器。
    pub fn init() -> Self {
        unsafe {
            enable_dwt();
        }
        Self {
            last_cycles: Cell::new(0),
            epoch_ns: Cell::new(0),
        }
    }
}

impl<const CPU_HZ: u32> Arch for DwtClock<CPU_HZ> {
    fn now_ns(&self) -> u64 {
        let cycles = unsafe { core::ptr::read_volatile(DWT_CYCCNT) };
        let (last, epoch) =
            extend_monotonic(self.last_cycles.get(), self.epoch_ns.get(), cycles, CPU_HZ);
        self.last_cycles.set(last);
        self.epoch_ns.set(epoch);
        epoch.saturating_add(cycles_to_ns(cycles, CPU_HZ))
    }
}

unsafe fn enable_dwt() {
    unsafe {
        let demcr = core::ptr::read_volatile(DEMCR);
        core::ptr::write_volatile(DEMCR, demcr | TRCENA);

        core::ptr::write_volatile(DWT_CYCCNT, 0);
        let ctrl = core::ptr::read_volatile(DWT_CTRL);
        core::ptr::write_volatile(DWT_CTRL, ctrl | CYCCNTENA);
    }
}
