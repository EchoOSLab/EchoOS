#![no_std]
#![no_main]

//! Q3 故障注入：主动执行 `udf`，验证 UsageFault/HardFault 状态可读且能干净退出。

use core::panic::PanicInfo;

use echo_trace::{SemihostingSink, write_bytes, write_str};

const CFSR: *const u32 = 0xE000_ED28 as *const u32;
const HFSR: *const u32 = 0xE000_ED2C as *const u32;
const SHCSR: *mut u32 = 0xE000_ED24 as *mut u32;

fn write_boot(text: &'static str) {
    write_str(&SemihostingSink, text);
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

fn report_fault(name: &'static str) -> ! {
    write_boot(name);
    write_boot(" cfsr=");
    write_hex(unsafe { core::ptr::read_volatile(CFSR) });
    write_boot(" hfsr=");
    write_hex(unsafe { core::ptr::read_volatile(HFSR) });
    write_boot("\r\n");
    semihosting_exit()
}

fn system_init() {
    unsafe {
        core::ptr::write_volatile(0xE000_ED08 as *mut u32, 0x0000_0000);
        let cpacr = core::ptr::read_volatile(0xE000_ED88 as *mut u32);
        core::ptr::write_volatile(0xE000_ED88 as *mut u32, cpacr | (0b1111 << 20));
        // 使能 UsageFault，确保 udf 进入 UsageFault_Handler。
        let shcsr = core::ptr::read_volatile(SHCSR);
        core::ptr::write_volatile(SHCSR, shcsr | (1 << 18));
        core::arch::asm!("dsb", "isb");
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn runtime_main() -> ! {
    system_init();
    write_boot("EchoOS MPS2: fault injection (udf)\r\n");
    unsafe {
        core::arch::asm!("udf #0");
    }
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn HardFault_Handler() -> ! {
    report_fault("EchoOS MPS2: HARDFAULT")
}

#[unsafe(no_mangle)]
pub extern "C" fn MemManage_Handler() -> ! {
    report_fault("EchoOS MPS2: MEMMANAGE")
}

#[unsafe(no_mangle)]
pub extern "C" fn BusFault_Handler() -> ! {
    report_fault("EchoOS MPS2: BUSFAULT")
}

#[unsafe(no_mangle)]
pub extern "C" fn UsageFault_Handler() -> ! {
    report_fault("EchoOS MPS2: USAGEFAULT")
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    write_boot("EchoOS MPS2: PANIC\r\n");
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
    .word Default_Handler

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
