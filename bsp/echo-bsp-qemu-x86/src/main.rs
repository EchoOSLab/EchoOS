#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[inline(always)]
unsafe fn outb(port: u16, val: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") port, in("al") val);
    }
}

#[inline(always)]
unsafe fn inb(port: u16) -> u8 {
    let val: u8;
    unsafe {
        core::arch::asm!("in al, dx", in("dx") port, out("al") val);
    }
    val
}

#[inline(always)]
unsafe fn outw(port: u16, val: u16) {
    unsafe {
        core::arch::asm!("out dx, ax", in("dx") port, in("ax") val);
    }
}

unsafe fn serial_init() {
    unsafe {
        outb(0x3F9, 0x00); // disable interrupts
        outb(0x3FB, 0x80); // DLAB on
        outb(0x3F8, 0x03); // divisor 3 (38400 baud)
        outb(0x3F9, 0x00);
        outb(0x3FB, 0x03); // 8N1
        outb(0x3FA, 0xC7); // FIFO enable/clear
        outb(0x3FC, 0x03); // DTR/RTS
    }
}

unsafe fn putc(byte: u8) {
    unsafe {
        while inb(0x3FD) & 0x20 == 0 {}
        outb(0x3F8, byte);
    }
}

unsafe fn puts(s: &[u8]) {
    for &byte in s {
        unsafe { putc(byte) };
    }
}

/// UEFI entry point. QEMU + OVMF 负责加载，固件跳到这里。
#[unsafe(export_name = "efi_main")]
pub extern "efiapi" fn efi_main(
    _image_handle: *mut core::ffi::c_void,
    _system_table: *mut core::ffi::c_void,
) -> usize {
    unsafe {
        serial_init();
        puts(b"Hello EchoOS\r\n");
        // QEMU isa-debug-exit：写 0x10 → 退出码 (0x10 << 1) | 1 = 33
        outw(0x501, 0x10);
    }
    loop {
        core::hint::spin_loop();
    }
}
