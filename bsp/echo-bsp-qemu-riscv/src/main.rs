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
    .section .text.entry
    .global _start
_start:
    la      sp, _stack_top
    la      a0, hello_msg
    call    puts

1:  wfi
    j       1b

    # NS16550A UART0 on QEMU virt
puts:
    li      t0, 0x10000000
2:  lbu     t1, 0(a0)
    beqz    t1, 3f
    sb      t1, 0(t0)
    addi    a0, a0, 1
    j       2b
3:  ret

    .section .rodata
    .align  2
hello_msg:
    .asciz  "Hello EchoOS\r\n"
"#
);
