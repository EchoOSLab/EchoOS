use core::cell::Cell;

use echo_trace::{Sink, write_bytes, write_str};

struct CountingSink {
    count: Cell<usize>,
    last: Cell<u8>,
}

impl CountingSink {
    fn new() -> Self {
        Self {
            count: Cell::new(0),
            last: Cell::new(0),
        }
    }
}

impl Sink for CountingSink {
    fn write_byte(&self, byte: u8) {
        self.count.set(self.count.get() + 1);
        self.last.set(byte);
    }
}

#[test]
fn writes_every_byte_in_order() {
    let sink = CountingSink::new();
    write_str(&sink, "EchoOS");
    assert_eq!(sink.count.get(), 6);
    assert_eq!(sink.last.get(), b'S');
}

#[test]
fn empty_string_writes_nothing() {
    let sink = CountingSink::new();
    write_str(&sink, "");
    assert_eq!(sink.count.get(), 0);
}

#[test]
fn bytes_api_matches_str_api() {
    let sink = CountingSink::new();
    write_bytes(&sink, b"Hello EchoOS\r\n");
    assert_eq!(sink.count.get(), 14);
    assert_eq!(sink.last.get(), b'\n');
}
