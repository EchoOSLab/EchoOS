#![no_std]
#![no_main]

//! QEMU MPS2-AN500 Cortex-M7 runtime（Q0）。
//!
//! 本 bin 只负责最小 runtime 契约：
//! - 完整 16 个系统向量 + VTOR；
//! - `.data` 拷贝 / `.bss` 清零 / FPU；
//! - SysTick 1 kHz（QEMU 虚拟时钟）；
//! - semihosting 输出与 `SYS_EXIT`。
//!
//! 真正的上下文切换在 Q1 加入，见 `docs/26-qemu-first-plan.md`。

use core::panic::PanicInfo;
use core::sync::atomic::{AtomicU32, Ordering};

use echo_trace::{SemihostingSink, write_str};

const SYST_CSR: *mut u32 = 0xE000_E010 as *mut u32;
const SYST_RVR: *mut u32 = 0xE000_E014 as *mut u32;
const SYST_CVR: *mut u32 = 0xE000_E018 as *mut u32;

const SYST_CSR_ENABLE: u32 = 1 << 0;
const SYST_CSR_TICKINT: u32 = 1 << 1;
const SYST_CSR_CLKSOURCE: u32 = 1 << 2;

/// QEMU MPS2-AN500 的标称 CPU 频率；仅用于 SysTick 分频，不作为性能证据。
const CPU_HZ: u32 = 25_000_000;
const TICK_HZ: u32 = 1_000;
const TICKS_TO_PROVE: u32 = 10;

static TICKS: AtomicU32 = AtomicU32::new(0);

fn write_boot(text: &'static str) {
    write_str(&SemihostingSink, text);
}

fn semihosting_exit() -> ! {
    unsafe {
        core::arch::asm!(
            "movs r0, #0x18",
            "movw r1, #0x0026",
            "movt r1, #0x0002",
            "bkpt 0xAB",
            options(noreturn)
        );
    }
}

fn system_init() {
    unsafe {
        // 向量表在 0x0000_0000
        core::ptr::write_volatile(0xE000_ED08 as *mut u32, 0x0000_0000);
        // 使能 FPU：CP10/CP11 全访问
        let cpacr = core::ptr::read_volatile(0xE000_ED88 as *mut u32);
        core::ptr::write_volatile(0xE000_ED88 as *mut u32, cpacr | (0b1111 << 20));
        core::arch::asm!("dsb", "isb");
    }
}

fn systick_init(cpu_hz: u32, tick_hz: u32) {
    let reload = (cpu_hz / tick_hz).max(1);
    unsafe {
        core::ptr::write_volatile(SYST_RVR, reload - 1);
        core::ptr::write_volatile(SYST_CVR, 0);
        core::ptr::write_volatile(
            SYST_CSR,
            SYST_CSR_ENABLE | SYST_CSR_TICKINT | SYST_CSR_CLKSOURCE,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SysTick_Handler() {
    TICKS.fetch_add(1, Ordering::Relaxed);
}

fn fault(name: &'static str) -> ! {
    write_boot(name);
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn HardFault_Handler() -> ! {
    fault("EchoOS MPS2: HARDFAULT\r\n")
}

#[unsafe(no_mangle)]
pub extern "C" fn MemManage_Handler() -> ! {
    fault("EchoOS MPS2: MEMMANAGE\r\n")
}

#[unsafe(no_mangle)]
pub extern "C" fn BusFault_Handler() -> ! {
    fault("EchoOS MPS2: BUSFAULT\r\n")
}

#[unsafe(no_mangle)]
pub extern "C" fn UsageFault_Handler() -> ! {
    fault("EchoOS MPS2: USAGEFAULT\r\n")
}

#[unsafe(no_mangle)]
pub extern "C" fn runtime_main() -> ! {
    system_init();
    write_boot("EchoOS MPS2: boot\r\n");

    systick_init(CPU_HZ, TICK_HZ);
    unsafe {
        core::arch::asm!("cpsie i");
    }

    while TICKS.load(Ordering::Relaxed) < TICKS_TO_PROVE {
        core::hint::spin_loop();
    }

    write_boot("EchoOS MPS2: systick OK\r\n");
    semihosting_exit()
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    write_boot("EchoOS MPS2: PANIC\r\n");
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}

core::arch::global_asm!(
    r#"
    .syntax unified
    .thumb

    .section .vector_table, "a"
    .word _stack_top
    .word Reset_Handler
    .word Default_Handler
    .word HardFault_Handler
    .word MemManage_Handler
    .word BusFault_Handler
    .word UsageFault_Handler
    .word Default_Handler
    .word Default_Handler
    .word Default_Handler
    .word Default_Handler
    .word Default_Handler
    .word Default_Handler
    .word Default_Handler
    .word Default_Handler
    .word SysTick_Handler

    .section .text.Reset_Handler, "ax"
    .global Reset_Handler
    .thumb_func
Reset_Handler:
    ldr r0, =_stack_top
    mov sp, r0

    ldr r0, =_sdata
    ldr r1, =_edata
    ldr r2, =_sidata
1:  cmp r0, r1
    beq 2f
    ldr r3, [r2], #4
    str r3, [r0], #4
    b 1b

2:  ldr r0, =_sbss
    ldr r1, =_ebss
    movs r3, #0
3:  cmp r0, r1
    beq 4f
    str r3, [r0], #4
    b 3b

4:  bl runtime_main
5:  b 5b
    .pool

    .section .text.Default_Handler, "ax"
    .global Default_Handler
    .thumb_func
Default_Handler:
    b Default_Handler
"#
);
