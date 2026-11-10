use std::sync::Arc;
use std::thread;

use echo_ipc::Mailbox;

const N: usize = 1024;
const ITEMS: u32 = 100_000;

#[test]
fn spsc_across_two_threads_preserves_order() {
    let mailbox = Arc::new(Mailbox::<N>::new());

    let producer = {
        let mailbox = Arc::clone(&mailbox);
        thread::spawn(move || {
            for value in 0..ITEMS {
                let mut pending = value;
                loop {
                    match mailbox.try_send(pending) {
                        Ok(()) => break,
                        Err(rejected) => {
                            pending = rejected;
                            std::hint::spin_loop();
                        }
                    }
                }
            }
        })
    };

    let consumer = {
        let mailbox = Arc::clone(&mailbox);
        thread::spawn(move || {
            let mut expected = 0u32;
            while expected < ITEMS {
                match mailbox.try_recv() {
                    Some(value) => {
                        assert_eq!(value, expected);
                        expected += 1;
                    }
                    None => std::hint::spin_loop(),
                }
            }
            expected
        })
    };

    producer.join().expect("producer panicked");
    assert_eq!(consumer.join().expect("consumer panicked"), ITEMS);
}
