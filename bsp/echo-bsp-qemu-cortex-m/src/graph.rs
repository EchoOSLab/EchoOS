#![no_std]
#![no_main]

//! QEMU Cortex-M7 上的 3 节点图功能集成。
//!
//! 说明：QEMU 不模拟周期精确的时钟，本 demo 用 `TickArch`（每帧显式推进）
//! 只验证 **调度 + IPC + 生成图的语义**；抖动的真实证据必须来自 H7 真机（W6+）。

use core::cell::Cell;
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicU32, Ordering};

use echo_arch_api::Arch;
use echo_ipc::Mailbox;
use echo_sched::CyclicExecutive;

// 与 host conformance 共用同一份 codegen 产物：同一 YAML → 同一静态调度表。
include!("../../../dataflows/generated/three_node.rs");

const FRAMES: u32 = 10_000;

static SENSOR_TO_CONTROL: Mailbox<8> = Mailbox::new();
static CONTROL_TO_ACTUATOR: Mailbox<8> = Mailbox::new();

static SENSOR_SEQ: AtomicU32 = AtomicU32::new(0);
static ACTUATOR_COUNT: AtomicU32 = AtomicU32::new(0);
static ACTUATOR_SUM: AtomicU32 = AtomicU32::new(0);
static DROPS: AtomicU32 = AtomicU32::new(0);

/// QEMU 用的确定性 tick 时钟（每帧推进，不读硬件）。
struct TickArch {
    now_ns: Cell<u64>,
}

impl TickArch {
    const fn new() -> Self {
        Self {
            now_ns: Cell::new(0),
        }
    }

    fn advance(&self, ns: u64) {
        self.now_ns.set(self.now_ns.get() + ns);
    }
}

impl Arch for TickArch {
    fn now_ns(&self) -> u64 {
        self.now_ns.get()
    }
}

fn semihosting_write0(message: &'static [u8]) {
    unsafe {
        core::arch::asm!(
            "movs r0, #4",
            "bkpt 0xAB",
            in("r1") message.as_ptr(),
            out("r0") _,
            options(nostack)
        );
    }
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

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    semihosting_write0(b"EchoOS: PANIC\r\n\0");
    loop {
        core::hint::spin_loop();
    }
}

fn sensor() {
    let value = SENSOR_SEQ.fetch_add(1, Ordering::Relaxed);
    if SENSOR_TO_CONTROL.try_send(value).is_err() {
        DROPS.fetch_add(1, Ordering::Relaxed);
    }
}

fn control() {
    match SENSOR_TO_CONTROL.try_recv() {
        Some(value) => {
            if CONTROL_TO_ACTUATOR.try_send(value.wrapping_mul(2)).is_err() {
                DROPS.fetch_add(1, Ordering::Relaxed);
            }
        }
        None => {
            DROPS.fetch_add(1, Ordering::Relaxed);
        }
    }
}

fn actuator() {
    match CONTROL_TO_ACTUATOR.try_recv() {
        Some(value) => {
            ACTUATOR_COUNT.fetch_add(1, Ordering::Relaxed);
            ACTUATOR_SUM.fetch_add(value, Ordering::Relaxed);
        }
        None => {
            DROPS.fetch_add(1, Ordering::Relaxed);
        }
    }
}

#[unsafe(no_mangle)]
extern "C" fn echo_main() -> ! {
    let arch = TickArch::new();
    let tasks = task_defs([sensor as fn(), control as fn(), actuator as fn()]);

    let exec = match CyclicExecutive::new(&arch, tasks, FRAME_NS) {
        Ok(exec) => exec,
        Err(_) => {
            semihosting_write0(b"EchoOS: schedule error\r\n\0");
            loop {
                core::hint::spin_loop();
            }
        }
    };

    for _ in 0..FRAMES {
        exec.run_frame();
        arch.advance(FRAME_NS);
    }

    let ok = DROPS.load(Ordering::Relaxed) == 0
        && ACTUATOR_COUNT.load(Ordering::Relaxed) == FRAMES
        && ACTUATOR_SUM.load(Ordering::Relaxed) == FRAMES.wrapping_mul(FRAMES - 1);

    if ok {
        semihosting_write0(b"EchoOS: 3-node graph OK\r\n\0");
    } else {
        semihosting_write0(b"EchoOS: 3-node graph FAIL\r\n\0");
    }
    semihosting_exit();
}

core::arch::global_asm!(
    r#"
    .syntax unified
    .thumb

    .section .vector_table, "a"
    .word 0x20004000
    .word Reset_Handler

    .section .text.Reset_Handler, "ax"
    .global Reset_Handler
    .thumb_func
Reset_Handler:
    movw r0, #0x4000
    movt r0, #0x2000
    mov  sp, r0
    bl   echo_main

1:  b 1b
    .pool
"#
);
