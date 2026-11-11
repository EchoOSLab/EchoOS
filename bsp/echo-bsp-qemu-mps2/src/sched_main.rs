#![no_std]
#![no_main]

//! QEMU MPS2-AN500 Cortex-M7 真实上下文切换（Q1）。
//!
//! - MSP/PSP 双栈；
//! - 首次任务伪造异常帧；
//! - PendSV 保存/恢复 R4-R11；
//! - SVC 主动 yield；
//! - SysTick 强制切换；
//! - 3 任务：task2 高优先级先抢占，随后降优先级，task0/1 同优先级 RR；
//! - 10,000 次切换后检查栈 canary 并退出。

use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};

use echo_sched::preemptive::{PreemptTaskDef, PreemptiveScheduler};
use echo_trace::{SemihostingSink, write_bytes, write_str};

const SYST_CSR: *mut u32 = 0xE000_E010 as *mut u32;
const SYST_RVR: *mut u32 = 0xE000_E014 as *mut u32;
const SYST_CVR: *mut u32 = 0xE000_E018 as *mut u32;
const ICSR: *mut u32 = 0xE000_ED04 as *mut u32;
const SHPR3: *mut u32 = 0xE000_ED20 as *mut u32;

const SYST_CSR_ENABLE: u32 = 1 << 0;
const SYST_CSR_TICKINT: u32 = 1 << 1;
const SYST_CSR_CLKSOURCE: u32 = 1 << 2;

const CPU_HZ: u32 = 25_000_000;
const TICK_HZ: u32 = 1_000;
const TASK_COUNT: usize = 4;
const STACK_WORDS: usize = 256;
const DUMMY_WORDS: usize = 64;
const CANARY: u32 = 0xDEAD_BEEF;
const SWITCHES: u32 = 20_000;
const BOOST_ROUNDS: u32 = 100;
const IDLE_UNBLOCK_AFTER: u32 = 100;
const SLEEP_TICKS: u32 = 2;

#[derive(Clone, Copy)]
struct Tcb {
    psp: *mut u32,
}

impl Tcb {
    const EMPTY: Self = Self {
        psp: core::ptr::null_mut(),
    };
}

static mut TCBS: [Tcb; TASK_COUNT] = [Tcb::EMPTY; TASK_COUNT];
static mut STACKS: [[u32; STACK_WORDS]; TASK_COUNT] = [[0; STACK_WORDS]; TASK_COUNT];
static mut DUMMY_STACK: [u32; DUMMY_WORDS] = [0; DUMMY_WORDS];
static mut SCHED: PreemptiveScheduler<TASK_COUNT> = PreemptiveScheduler::new();

static CURRENT: AtomicU8 = AtomicU8::new(0xFF);
static SWITCH_COUNT: AtomicU32 = AtomicU32::new(0);
static TASK0_COUNT: AtomicU32 = AtomicU32::new(0);
static TASK1_COUNT: AtomicU32 = AtomicU32::new(0);
static TASK2_COUNT: AtomicU32 = AtomicU32::new(0);
static IDLE_COUNT: AtomicU32 = AtomicU32::new(0);
static SLEEP_WAKE_COUNT: AtomicU32 = AtomicU32::new(0);
static TICKS: AtomicU32 = AtomicU32::new(0);
static FINISHED: AtomicBool = AtomicBool::new(false);

fn write_boot(text: &'static str) {
    write_str(&SemihostingSink, text);
}

fn write_u32(mut value: u32) {
    if value == 0 {
        write_bytes(&SemihostingSink, b"0");
        return;
    }
    let mut buffer = [0u8; 10];
    let mut index = buffer.len();
    while value > 0 {
        index -= 1;
        buffer[index] = b'0' + (value % 10) as u8;
        value /= 10;
    }
    write_bytes(&SemihostingSink, &buffer[index..]);
}

fn write_hex(mut value: u32) {
    let digits = b"0123456789abcdef";
    let mut buffer = [b'0'; 8];
    for index in (0..8).rev() {
        buffer[index] = digits[(value & 0xF) as usize];
        value >>= 4;
    }
    write_bytes(&SemihostingSink, b"0x");
    write_bytes(&SemihostingSink, &buffer);
}

fn report_fault(name: &'static str) -> ! {
    write_boot(name);
    write_boot(" CFSR=");
    write_hex(unsafe { core::ptr::read_volatile(0xE000_ED28 as *const u32) });
    write_boot(" HFSR=");
    write_hex(unsafe { core::ptr::read_volatile(0xE000_ED2C as *const u32) });
    write_boot(" MMFAR=");
    write_hex(unsafe { core::ptr::read_volatile(0xE000_ED34 as *const u32) });
    write_boot(" BFAR=");
    write_hex(unsafe { core::ptr::read_volatile(0xE000_ED38 as *const u32) });
    write_boot("\r\n");
    semihosting_exit()
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

fn fatal(message: &'static str) -> ! {
    write_boot(message);
    semihosting_exit()
}

fn system_init() {
    unsafe {
        core::ptr::write_volatile(0xE000_ED08 as *mut u32, 0x0000_0000);
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

fn pend_pendsv() {
    unsafe {
        core::ptr::write_volatile(ICSR, 1 << 28);
        core::arch::asm!("dsb", "isb");
    }
}

fn task_stack_base(index: usize) -> *mut u32 {
    unsafe { (core::ptr::addr_of_mut!(STACKS).cast::<u32>()).add(index * STACK_WORDS) }
}

fn tcb_ptr(index: usize) -> *mut Tcb {
    unsafe { (core::ptr::addr_of_mut!(TCBS).cast::<Tcb>()).add(index) }
}

fn task_exit() -> ! {
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}

fn init_task_stack(index: usize, entry: fn()) {
    let base = task_stack_base(index);
    let top = ((base as usize) + STACK_WORDS * core::mem::size_of::<u32>()) & !7;
    let frame = (top - 8 * core::mem::size_of::<u32>()) as *mut u32;

    // PendSV 保存的是 R4-R11 区域；首次任务的 PSP 必须指向这个区域，
    // 其上方才是硬件异常帧（R0-R3, R12, LR, PC, xPSR）。
    unsafe {
        let saved = frame.sub(8);
        frame.add(0).write(0);
        frame.add(1).write(0);
        frame.add(2).write(0);
        frame.add(3).write(0);
        frame.add(4).write(0);
        frame
            .add(5)
            .write(((task_exit as *const () as usize) | 1) as u32);
        frame.add(6).write(((entry as usize) | 1) as u32);
        frame.add(7).write(0x0100_0000);

        for slot in 0..8 {
            saved.add(slot).write(0);
        }

        base.write(CANARY);
        (*tcb_ptr(index)).psp = saved;
    }
}

fn init_scheduler() {
    let sched = unsafe { &mut *core::ptr::addr_of_mut!(SCHED) };
    sched
        .declare(PreemptTaskDef::new("task0", 2, task0))
        .expect("task0");
    sched
        .declare(PreemptTaskDef::new("task1", 2, task1))
        .expect("task1");
    sched
        .declare(PreemptTaskDef::new("task2", 3, task2))
        .expect("task2");
    sched
        .declare(PreemptTaskDef::new("idle", 0, idle_task))
        .expect("idle");

    init_task_stack(0, task0);
    init_task_stack(1, task1);
    init_task_stack(2, task2);
    init_task_stack(3, idle_task);
}

fn canaries_ok() -> bool {
    for index in 0..TASK_COUNT {
        if unsafe { task_stack_base(index).read_volatile() } != CANARY {
            return false;
        }
    }
    true
}

fn finish() -> ! {
    if FINISHED.swap(true, Ordering::SeqCst) {
        loop {
            unsafe {
                core::arch::asm!("wfi");
            }
        }
    }

    let c0 = TASK0_COUNT.load(Ordering::Relaxed);
    let c1 = TASK1_COUNT.load(Ordering::Relaxed);
    let c2 = TASK2_COUNT.load(Ordering::Relaxed);
    let idle = IDLE_COUNT.load(Ordering::Relaxed);
    let wake = SLEEP_WAKE_COUNT.load(Ordering::Relaxed);
    let ticks = TICKS.load(Ordering::Relaxed);
    let switches = SWITCH_COUNT.load(Ordering::Relaxed);

    if c0 == 0 || c1 == 0 || c2 == 0 || idle < IDLE_UNBLOCK_AFTER || wake == 0 || ticks == 0 {
        write_boot("EchoOS MPS2: sched FAIL\r\n");
        semihosting_exit();
    }

    write_boot("EchoOS MPS2: sched OK t0=");
    write_u32(c0);
    write_boot(" t1=");
    write_u32(c1);
    write_boot(" t2=");
    write_u32(c2);
    write_boot(" idle=");
    write_u32(idle);
    write_boot(" wake=");
    write_u32(wake);
    write_boot(" ticks=");
    write_u32(ticks);
    write_boot(" switches=");
    write_u32(switches);
    write_boot("\r\n");
    semihosting_exit()
}

fn done() -> bool {
    SWITCH_COUNT.load(Ordering::Relaxed) >= SWITCHES
}

fn yield_now() {
    unsafe {
        core::arch::asm!("svc 0");
    }
}

fn lower_task2_priority() {
    let sched = unsafe { &mut *core::ptr::addr_of_mut!(SCHED) };
    sched.set_priority(2, 1).expect("lower task2");
}

fn sleep_current(ticks: u32) {
    let now_ns = u64::from(TICKS.load(Ordering::Relaxed)) * 1_000_000;
    let deadline = now_ns + u64::from(ticks) * 1_000_000;
    let sched = unsafe { &mut *core::ptr::addr_of_mut!(SCHED) };
    sched
        .sleep_until(CURRENT.load(Ordering::Relaxed), deadline)
        .expect("sleep");
    pend_pendsv();
    SLEEP_WAKE_COUNT.fetch_add(1, Ordering::Relaxed);
}

fn block_self() {
    let sched = unsafe { &mut *core::ptr::addr_of_mut!(SCHED) };
    sched.block(CURRENT.load(Ordering::Relaxed));
    pend_pendsv();
}

fn task0() {
    let mut rounds = 0;
    while rounds < BOOST_ROUNDS {
        TASK0_COUNT.fetch_add(1, Ordering::Relaxed);
        yield_now();
        rounds += 1;
    }
    block_self();

    sleep_current(SLEEP_TICKS);
    loop {
        TASK0_COUNT.fetch_add(1, Ordering::Relaxed);
        yield_now();
        if done() {
            finish();
        }
    }
}

fn task1() {
    let mut rounds = 0;
    while rounds < BOOST_ROUNDS {
        TASK1_COUNT.fetch_add(1, Ordering::Relaxed);
        yield_now();
        rounds += 1;
    }
    block_self();

    loop {
        TASK1_COUNT.fetch_add(1, Ordering::Relaxed);
        yield_now();
        if done() {
            finish();
        }
    }
}

fn task2() {
    let mut rounds = 0;
    while rounds < BOOST_ROUNDS {
        TASK2_COUNT.fetch_add(1, Ordering::Relaxed);
        yield_now();
        rounds += 1;
    }
    lower_task2_priority();
    block_self();

    loop {
        TASK2_COUNT.fetch_add(1, Ordering::Relaxed);
        yield_now();
        if done() {
            finish();
        }
    }
}

fn idle_task() {
    loop {
        let count = IDLE_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
        if count == IDLE_UNBLOCK_AFTER {
            let sched = unsafe { &mut *core::ptr::addr_of_mut!(SCHED) };
            sched.unblock(0);
            sched.unblock(1);
            sched.unblock(2);
        }
        yield_now();
        if done() {
            finish();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn svc_handler() {
    pend_pendsv();
}

#[unsafe(no_mangle)]
pub extern "C" fn SysTick_Handler() {
    let ticks = TICKS.fetch_add(1, Ordering::Relaxed) + 1;
    let now_ns = u64::from(ticks) * 1_000_000;
    let sched = unsafe { &mut *core::ptr::addr_of_mut!(SCHED) };
    sched.wake_due(now_ns);
    pend_pendsv();
}

#[unsafe(no_mangle)]
pub extern "C" fn pendsv_switch(psp: *mut u32) -> *mut u32 {
    let sched = unsafe { &mut *core::ptr::addr_of_mut!(SCHED) };
    let current = CURRENT.load(Ordering::Relaxed);

    if current != 0xFF {
        unsafe {
            (*tcb_ptr(current as usize)).psp = psp;
        }
        if sched.state(current) == Some(echo_sched::preemptive::TaskState::Running) {
            sched.yield_now();
        }
    }

    let Some(next) = sched.pick_next() else {
        fatal("EchoOS MPS2: no ready task\r\n");
    };
    let next = next as usize;
    CURRENT.store(next as u8, Ordering::Relaxed);
    SWITCH_COUNT.fetch_add(1, Ordering::Relaxed);

    if !canaries_ok() {
        fatal("EchoOS MPS2: CANARY FAIL\r\n");
    }

    unsafe { (*tcb_ptr(next)).psp }
}

fn start_first_task() -> ! {
    let dummy_base = core::ptr::addr_of_mut!(DUMMY_STACK).cast::<u32>();
    let dummy_top = ((dummy_base as usize) + DUMMY_WORDS * core::mem::size_of::<u32>()) & !7;

    unsafe {
        // PendSV 最低优先级，SysTick 次之；此处在 MSP 上完成。
        let shpr3 = core::ptr::read_volatile(SHPR3);
        core::ptr::write_volatile(SHPR3, (shpr3 & 0x0000_FFFF) | (0xFF << 16) | (0x80 << 24));

        // 关键：切到 PSP 后不能再执行任何使用 C 栈的代码，
        // 否则局部变量会写进 DUMMY_STACK/相邻 .bss（Q1 调试中实际踩过）。
        core::arch::asm!(
            "msr psp, {dummy}",
            "movs r0, #2",
            "msr control, r0",
            "isb",
            "movw r0, #0xED04",
            "movt r0, #0xE000",
            "movs r1, #1",
            "lsls r1, r1, #28",
            "str r1, [r0]",
            "dsb",
            "isb",
            "cpsie i",
            "2: b 2b",
            dummy = in(reg) dummy_top as u32,
            options(noreturn)
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn runtime_main() -> ! {
    system_init();
    init_scheduler();
    systick_init(CPU_HZ, TICK_HZ);
    start_first_task()
}

#[unsafe(no_mangle)]
pub extern "C" fn HardFault_Handler() -> ! {
    report_fault("EchoOS MPS2: HARDFAULT")
}

#[unsafe(no_mangle)]
pub extern "C" fn MemManage_Handler() -> ! {
    fatal("EchoOS MPS2: MEMMANAGE\r\n")
}

#[unsafe(no_mangle)]
pub extern "C" fn BusFault_Handler() -> ! {
    fatal("EchoOS MPS2: BUSFAULT\r\n")
}

#[unsafe(no_mangle)]
pub extern "C" fn UsageFault_Handler() -> ! {
    fatal("EchoOS MPS2: USAGEFAULT\r\n")
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    fatal("EchoOS MPS2: PANIC\r\n")
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
    .word SVC_Handler
    .word Default_Handler
    .word Default_Handler
    .word PendSV_Handler
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

    .section .text.SVC_Handler, "ax"
    .global SVC_Handler
    .thumb_func
SVC_Handler:
    push {{lr}}
    bl svc_handler
    pop {{lr}}
    bx lr

    .section .text.PendSV_Handler, "ax"
    .global PendSV_Handler
    .thumb_func
PendSV_Handler:
    mrs r0, psp
    stmdb r0!, {{r4-r11}}
    push {{lr}}
    bl pendsv_switch
    pop {{lr}}
    ldmia r0!, {{r4-r11}}
    msr psp, r0
    bx lr

    .section .text.Default_Handler, "ax"
    .global Default_Handler
    .thumb_func
Default_Handler:
    b Default_Handler
"#
);
