#![no_std]

//! trace 输出抽象（W6）。
//!
//! 设计契约见 `docs/17-h7-bringup.md`。
//!
//! 内核只依赖 [`Sink`]，具体后端由 BSP 决定：
//! - QEMU / H7 首次 bring-up：semihosting；
//! - H7 正式运行：UART；
//! - host 测试：自定义计数 sink。

/// 字节输出后端。
pub trait Sink {
    fn write_byte(&self, byte: u8);
}

/// 写字符串（逐字节，不做分配）。
pub fn write_str<S: Sink + ?Sized>(sink: &S, text: &str) {
    write_bytes(sink, text.as_bytes());
}

/// 写字节切片。
pub fn write_bytes<S: Sink + ?Sized>(sink: &S, bytes: &[u8]) {
    for &byte in bytes {
        sink.write_byte(byte);
    }
}

/// ARM semihosting 后端（仅 QEMU 或挂着调试器的真机可用）。
#[cfg(target_arch = "arm")]
pub struct SemihostingSink;

#[cfg(target_arch = "arm")]
impl Sink for SemihostingSink {
    fn write_byte(&self, byte: u8) {
        // SYS_WRITEC (0x03)，r1 = 字符指针
        unsafe {
            core::arch::asm!(
                "movs r0, #3",
                "bkpt 0xAB",
                in("r1") &byte as *const u8,
                out("r0") _,
            );
        }
    }
}

/// 一条 trace 事件。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    pub id: u16,
    pub value: u32,
    pub timestamp_ns: u64,
}

/// 固定容量事件环形缓冲。
pub struct EventRing<const N: usize> {
    entries: [Option<Event>; N],
    head: usize,
    len: usize,
    dropped: usize,
}

impl<const N: usize> EventRing<N> {
    pub const fn new() -> Self {
        assert!(N > 0, "EventRing capacity must be > 0");
        Self {
            entries: [None; N],
            head: 0,
            len: 0,
            dropped: 0,
        }
    }

    pub fn push(&mut self, event: Event) -> bool {
        if self.len >= N {
            self.dropped += 1;
            return false;
        }
        let index = (self.head + self.len) % N;
        self.entries[index] = Some(event);
        self.len += 1;
        true
    }

    pub fn pop(&mut self) -> Option<Event> {
        if self.len == 0 {
            return None;
        }
        let event = self.entries[self.head].take();
        self.head = (self.head + 1) % N;
        self.len -= 1;
        event
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn dropped(&self) -> usize {
        self.dropped
    }

    pub fn clear(&mut self) {
        self.entries = [None; N];
        self.head = 0;
        self.len = 0;
    }
}

impl<const N: usize> Default for EventRing<N> {
    fn default() -> Self {
        Self::new()
    }
}
