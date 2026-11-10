#![no_std]
#![no_main]

//! WeAct MiniSTM32H750VBT6 板级支持（H750，128 KB Flash / 1 MB RAM）。
//!
//! **首启时钟**：H750 复位后默认 HSI 64 MHz；本 BSP 不配置 PLL，
//! 因此 `CPU_HZ = 64_000_000`。480 MHz 需要后续 PLL 配置并在板上验证。
//!
//! 调试方式：ST-Link SWD + semihosting；烧录/调试步骤见
//! `docs/24-weact-h750-bringup.md`。

use core::panic::PanicInfo;
use core::sync::atomic::{AtomicU32, Ordering};

use echo_arch_cortex_m::DwtClock;
use echo_ipc::Mailbox;
use echo_sched::CyclicExecutive;
use echo_trace::{SemihostingSink, write_str};

/// H750 复位默认 HSI 64 MHz；配置 PLL 前不要改成 480 MHz。
const CPU_HZ: u32 = 64_000_000;
const FRAMES: u32 = 10_000;

include!("../../../dataflows/generated/three_node.rs");

static S2C: Mailbox<8> = Mailbox::new();
static C2A: Mailbox<8> = Mailbox::new();
static SENSOR_SEQ: AtomicU32 = AtomicU32::new(0);
static ACTUATOR_COUNT: AtomicU32 = AtomicU32::new(0);
static ACTUATOR_SUM: AtomicU32 = AtomicU32::new(0);
static DROPS: AtomicU32 = AtomicU32::new(0);

fn sensor() {
    let value = SENSOR_SEQ.fetch_add(1, Ordering::Relaxed);
    if S2C.try_send(value).is_err() {
        DROPS.fetch_add(1, Ordering::Relaxed);
    }
}

fn control() {
    match S2C.try_recv() {
        Some(value) => {
            if C2A.try_send(value.wrapping_mul(2)).is_err() {
                DROPS.fetch_add(1, Ordering::Relaxed);
            }
        }
        None => {
            DROPS.fetch_add(1, Ordering::Relaxed);
        }
    }
}

fn actuator() {
    match C2A.try_recv() {
        Some(value) => {
            ACTUATOR_COUNT.fetch_add(1, Ordering::Relaxed);
            ACTUATOR_SUM.fetch_add(value, Ordering::Relaxed);
        }
        None => {
            DROPS.fetch_add(1, Ordering::Relaxed);
        }
    }
}

fn system_init() {
    unsafe {
        // VTOR 指向内部 Flash 起始地址
        core::ptr::write_volatile(0xE000_ED08 as *mut u32, 0x0800_0000);
        // 使能 FPU（CP10/CP11 全访问）
        let cpacr = core::ptr::read_volatile(0xE000_ED88 as *mut u32);
        core::ptr::write_volatile(0xE000_ED88 as *mut u32, cpacr | (0b1111 << 20));
        core::arch::asm!("dsb", "isb");
    }
}

fn run_graph(clock: &DwtClock<CPU_HZ>) {
    let exec = match CyclicExecutive::new(
        clock,
        task_defs([sensor as fn(), control as fn(), actuator as fn()]),
        FRAME_NS,
    ) {
        Ok(exec) => exec,
        Err(_) => {
            write_str(&SemihostingSink, "EchoOS H750: schedule FAIL\r\n");
            return;
        }
    };

    for _ in 0..FRAMES {
        exec.run_frame();
    }

    let count = ACTUATOR_COUNT.load(Ordering::Relaxed);
    let sum = ACTUATOR_SUM.load(Ordering::Relaxed);
    let drops = DROPS.load(Ordering::Relaxed);
    let expected_sum = FRAMES.wrapping_mul(FRAMES - 1);

    if drops == 0 && count == FRAMES && sum == expected_sum {
        write_str(&SemihostingSink, "EchoOS H750: 3-node graph OK\r\n");
    } else {
        write_str(&SemihostingSink, "EchoOS H750: 3-node graph FAIL\r\n");
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn echo_main() -> ! {
    system_init();
    let clock = DwtClock::<CPU_HZ>::init();
    write_str(
        &SemihostingSink,
        "EchoOS H750: boot (HSI 64MHz, 128KB flash)\r\n",
    );
    run_graph(&clock);
    halt()
}

fn halt() -> ! {
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    write_str(&SemihostingSink, "EchoOS H750: PANIC\r\n");
    halt()
}

#[unsafe(no_mangle)]
pub extern "C" fn HardFault_Handler() -> ! {
    write_str(&SemihostingSink, "EchoOS H750: HARDFAULT\r\n");
    halt()
}

core::arch::global_asm!(
    r#"
    .syntax unified
    .thumb

    .section .vector_table, "a"
    .word 0x20020000
    .word Reset_Handler
    .word Default_Handler
    .word HardFault_Handler
    .word HardFault_Handler
    .word HardFault_Handler
    .word HardFault_Handler
    .word Default_Handler
    .word Default_Handler
    .word Default_Handler
    .word Default_Handler
    .word Default_Handler
    .word Default_Handler
    .word Default_Handler
    .word Default_Handler
    .word Default_Handler

    .section .text.Reset_Handler, "ax"
    .global Reset_Handler
    .thumb_func
Reset_Handler:
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
4:  bl echo_main
5:  b 5b
    .pool

    .section .text.Default_Handler, "ax"
    .global Default_Handler
    .thumb_func
Default_Handler:
    b Default_Handler
"#
);
