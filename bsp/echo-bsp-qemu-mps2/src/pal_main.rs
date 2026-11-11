#![no_std]
#![no_main]

//! Q2-16/17：QEMU MPS2-AN500 上的 PAL `Platform` 实现。
//! 运行与 host Mock 相同的 `echo_conformance::contract::check_common`。

use core::cell::{Cell, RefCell};
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicU32, Ordering};

use echo_conformance::contract::check_common;
use echo_ipc::Mailbox;
use echo_mem::BlockPool;
use echo_pal_api::{
    Capabilities, ChannelApi, MemoryApi, ObservabilityApi, Platform, PoolBlock, StaticTask,
    TaskApi, TaskError, TimeApi,
};
use echo_trace::{SemihostingSink, write_str};

const SYST_CSR: *mut u32 = 0xE000_E010 as *mut u32;
const SYST_RVR: *mut u32 = 0xE000_E014 as *mut u32;
const SYST_CVR: *mut u32 = 0xE000_E018 as *mut u32;

const CPU_HZ: u32 = 25_000_000;
const TICK_HZ: u32 = 1_000;
const BLOCK_SIZE: usize = 16;
const BLOCKS: usize = 8;
const MAX_TASKS: usize = 4;
const TRACE_CAPACITY: usize = 16;

static TICKS: AtomicU32 = AtomicU32::new(0);

struct QemuPlatform {
    tasks: Cell<usize>,
    pool: RefCell<BlockPool<BLOCK_SIZE, BLOCKS>>,
    channel: Mailbox<8>,
    trace_count: Cell<usize>,
    trace_dropped: Cell<usize>,
}

impl QemuPlatform {
    fn new() -> Self {
        Self {
            tasks: Cell::new(0),
            pool: RefCell::new(BlockPool::new()),
            channel: Mailbox::new(),
            trace_count: Cell::new(0),
            trace_dropped: Cell::new(0),
        }
    }
}

impl Platform for QemuPlatform {
    type Task = Self;
    type Time = Self;
    type Memory = Self;
    type Channel = Self;
    type Trace = Self;

    fn capabilities(&self) -> Capabilities {
        Capabilities::micro()
    }

    fn task(&self) -> &Self::Task {
        self
    }

    fn time(&self) -> &Self::Time {
        self
    }

    fn memory(&self) -> &Self::Memory {
        self
    }

    fn channel(&self) -> &Self::Channel {
        self
    }

    fn trace(&self) -> &Self::Trace {
        self
    }
}

impl TaskApi for QemuPlatform {
    type TaskId = u8;

    fn current(&self) -> Self::TaskId {
        0
    }

    fn task_count(&self) -> usize {
        self.tasks.get()
    }

    fn yield_now(&self) {}

    fn declare(&self, _task: StaticTask) -> Result<Self::TaskId, TaskError> {
        let count = self.tasks.get();
        if count >= MAX_TASKS {
            return Err(TaskError::CapacityExceeded);
        }
        self.tasks.set(count + 1);
        Ok(count as u8)
    }
}

impl TimeApi for QemuPlatform {
    fn now_ns(&self) -> u64 {
        u64::from(TICKS.load(Ordering::Relaxed)) * 1_000_000
    }
}

impl MemoryApi for QemuPlatform {
    fn pool_capacity(&self) -> usize {
        BLOCK_SIZE * BLOCKS
    }

    fn pool_used(&self) -> usize {
        self.pool.borrow().used_bytes()
    }

    fn try_alloc(&self, len: usize) -> Option<PoolBlock> {
        if len == 0 || len > BLOCK_SIZE {
            return None;
        }
        self.pool
            .borrow_mut()
            .try_alloc()
            .ok()
            .map(|block| PoolBlock {
                index: block.index,
                len: BLOCK_SIZE as u16,
            })
    }

    fn free(&self, block: PoolBlock) {
        let _ = self
            .pool
            .borrow_mut()
            .free(echo_mem::PoolBlock { index: block.index });
    }
}

impl ChannelApi for QemuPlatform {
    fn capacity(&self) -> usize {
        self.channel.capacity()
    }

    fn try_send(&self, value: u32) -> Result<(), u32> {
        self.channel.try_send(value)
    }

    fn try_recv(&self) -> Option<u32> {
        self.channel.try_recv()
    }
}

impl ObservabilityApi for QemuPlatform {
    fn trace_event(&self, _id: u16, _value: u32) {
        let count = self.trace_count.get();
        if count < TRACE_CAPACITY {
            self.trace_count.set(count + 1);
        } else {
            self.trace_dropped.set(self.trace_dropped.get() + 1);
        }
    }

    fn event_count(&self) -> usize {
        self.trace_count.get()
    }

    fn dropped_events(&self) -> usize {
        self.trace_dropped.get()
    }
}

fn system_init() {
    unsafe {
        core::ptr::write_volatile(0xE000_ED08 as *mut u32, 0x0000_0000);
        let cpacr = core::ptr::read_volatile(0xE000_ED88 as *mut u32);
        core::ptr::write_volatile(0xE000_ED88 as *mut u32, cpacr | (0b1111 << 20));
        core::arch::asm!("dsb", "isb");
    }
}

fn systick_init() {
    let reload = (CPU_HZ / TICK_HZ).max(1);
    unsafe {
        core::ptr::write_volatile(SYST_RVR, reload - 1);
        core::ptr::write_volatile(SYST_CVR, 0);
        core::ptr::write_volatile(SYST_CSR, 7);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SysTick_Handler() {
    TICKS.fetch_add(1, Ordering::Relaxed);
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

#[unsafe(no_mangle)]
pub extern "C" fn runtime_main() -> ! {
    system_init();
    systick_init();
    unsafe {
        core::arch::asm!("cpsie i");
    }
    while TICKS.load(Ordering::Relaxed) < 2 {
        core::hint::spin_loop();
    }

    let platform = QemuPlatform::new();
    match check_common(&platform) {
        Ok(()) => {
            write_str(&SemihostingSink, "PAL QEMU: OK\r\n");
        }
        Err(reason) => {
            write_str(&SemihostingSink, "PAL QEMU: FAIL ");
            write_str(&SemihostingSink, reason);
            write_str(&SemihostingSink, "\r\n");
        }
    }
    semihosting_exit()
}

#[unsafe(no_mangle)]
pub extern "C" fn HardFault_Handler() -> ! {
    write_str(&SemihostingSink, "PAL QEMU: HARDFAULT\r\n");
    semihosting_exit()
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    write_str(&SemihostingSink, "PAL QEMU: PANIC\r\n");
    semihosting_exit()
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
    .word Default_Handler
    .word Default_Handler
    .word Default_Handler
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
