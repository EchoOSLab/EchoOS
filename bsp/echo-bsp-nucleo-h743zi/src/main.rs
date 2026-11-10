#![no_std]
#![no_main]

//! STM32H743（Nucleo-H743ZI2）bring-up 骨架（W6）。
//!
//! **硬件门未过**：本仓库当前没有 ST-Link / 板卡，因此本 bin 只做编译验证。
//! 首次上板用 semihosting 输出（需调试器）；UART 后端在拿到板子后替换。
//! 验收清单见 `docs/17-h7-bringup.md`。

use core::panic::PanicInfo;

use echo_arch_api::Arch;
use echo_arch_cortex_m::DwtClock;
use echo_trace::{SemihostingSink, write_str};

const CPU_HZ: u32 = 480_000_000;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    write_str(&SemihostingSink, "EchoOS H7: PANIC\r\n");
    halt()
}

fn halt() -> ! {
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}

#[unsafe(no_mangle)]
extern "C" fn echo_main() -> ! {
    let clock = DwtClock::<CPU_HZ>::init();
    let _ = clock.now_ns();
    write_str(&SemihostingSink, "EchoOS H7: boot (DWT clock enabled)\r\n");
    halt()
}

core::arch::global_asm!(
    r#"
    .syntax unified
    .thumb

    .section .vector_table, "a"
    .word 0x20020000          @ SP = DTCM 顶部
    .word Reset_Handler

    .section .text.Reset_Handler, "ax"
    .global Reset_Handler
    .thumb_func
Reset_Handler:
    movw r0, #0x0000
    movt r0, #0x2002
    mov  sp, r0
    bl   echo_main

1:  b 1b
    .pool
"#
);
