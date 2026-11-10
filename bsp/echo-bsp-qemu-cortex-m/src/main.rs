#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

core::arch::global_asm!(
    r#"
    .syntax unified
    .thumb

    .section .vector_table, "a"
    .word 0x20004000          @ initial SP
    .word Reset_Handler       @ reset vector

    .section .text.Reset_Handler, "ax"
    .global Reset_Handler
    .thumb_func
Reset_Handler:
    movw r0, #0x4000
    movt r0, #0x2000          @ SP = 0x20004000 (16 KiB into SRAM)
    mov  sp, r0

    @ ARM semihosting SYS_WRITE0 (op = 0x04, arg = string)
    movs r0, #4
    ldr  r1, =hello_msg
    bkpt 0xAB

    @ 退出或常驻由 Rust 侧按 feature 决定
    bl   platform_after_print

1:  b 1b
    .pool

    .section .rodata, "a"
    .align 2
hello_msg:
    .asciz "Hello EchoOS\r\n"
"#
);

#[unsafe(no_mangle)]
extern "C" fn platform_after_print() -> ! {
    #[cfg(not(feature = "gui"))]
    unsafe {
        // ARM semihosting SYS_EXIT (0x18), ADP_Stopped_ApplicationExit
        core::arch::asm!(
            "movs r0, #0x18",
            "movw r1, #0x0026",
            "movt r1, #0x0002",
            "bkpt 0xAB",
            options(noreturn)
        );
    }
    loop {
        core::hint::spin_loop();
    }
}
