#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]

//! Cortex-M 的 `Arch` 实现（W6，DWT 时基）。
//!
//! 设计契约见 `docs/17-h7-bringup.md`。
//!
//! - 时基：DWT `CYCCNT`（周期计数器）+ 编译期 `CPU_HZ`；
//! - 回绕：32 位计数器约 8.9 s @480 MHz 回绕，v0 **未处理**，W8 补；
//! - 该实现需要在真机上验证；QEMU 不保证 DWT 周期精度。

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

/// 基于 DWT 周期计数器的单调时钟。
pub struct DwtClock<const CPU_HZ: u32>;

impl<const CPU_HZ: u32> DwtClock<CPU_HZ> {
    /// 使能 DWT 并清零周期计数器。
    pub fn init() -> Self {
        unsafe {
            enable_dwt();
        }
        Self
    }
}

impl<const CPU_HZ: u32> Arch for DwtClock<CPU_HZ> {
    fn now_ns(&self) -> u64 {
        let cycles = unsafe { core::ptr::read_volatile(DWT_CYCCNT) };
        cycles_to_ns(cycles, CPU_HZ)
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
