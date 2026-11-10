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

#[cfg(not(feature = "gui"))]
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

#[allow(dead_code)]
#[repr(C)]
struct TableHeader {
    signature: u64,
    revision: u32,
    header_size: u32,
    crc32: u32,
    reserved: u32,
}

#[allow(dead_code)]
#[repr(C)]
struct SimpleTextOutput {
    reset: usize,
    output_string: unsafe extern "efiapi" fn(*mut SimpleTextOutput, *const u16) -> usize,
    test_string: usize,
    query_mode: usize,
    set_mode: usize,
    set_attribute: usize,
    clear_screen: usize,
    set_cursor_position: usize,
    enable_cursor: usize,
    mode: usize,
}

/// 只定义到 `ConOut` 为止；后续字段不使用。
#[allow(dead_code)]
#[repr(C)]
struct SystemTable {
    hdr: TableHeader,
    firmware_vendor: *const u16,
    firmware_revision: u32,
    _pad: u32,
    console_in_handle: *mut core::ffi::c_void,
    console_in: *mut core::ffi::c_void,
    console_out_handle: *mut core::ffi::c_void,
    console_out: *mut SimpleTextOutput,
}

unsafe fn con_out_puts(system_table: *mut SystemTable, s: &[u8]) {
    if system_table.is_null() {
        return;
    }
    let con_out = unsafe { (*system_table).console_out };
    if con_out.is_null() {
        return;
    }

    let mut buf = [0u16; 128];
    let mut n = 0;
    for &byte in s {
        if n == buf.len() - 1 {
            break;
        }
        buf[n] = byte as u16;
        n += 1;
    }
    buf[n] = 0;

    let output_string = unsafe { (*con_out).output_string };
    unsafe { output_string(con_out, buf.as_ptr()) };
}

/// UEFI entry point. QEMU + OVMF 负责加载，固件跳到这里。
#[unsafe(export_name = "efi_main")]
extern "efiapi" fn efi_main(
    _image_handle: *mut core::ffi::c_void,
    system_table: *mut SystemTable,
) -> usize {
    unsafe {
        // COM1：无头 CI 用
        serial_init();
        puts(b"Hello EchoOS\r\n");

        // ConOut：图形窗口（OVMF 的 VGA/GOP 控制台）里可见
        con_out_puts(system_table, b"Hello EchoOS\r\n");

        // 非 gui：QEMU isa-debug-exit 退出；gui：停在原地保持窗口
        #[cfg(not(feature = "gui"))]
        outw(0x501, 0x10);
    }
    loop {
        core::hint::spin_loop();
    }
}
