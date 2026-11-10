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

1:  b 1b
    .pool

    .section .rodata, "a"
    .align 2
hello_msg:
    .asciz "Hello EchoOS\r\n"
"#
);
