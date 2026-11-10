include!("../../../../dataflows/generated/three_node.rs");

const FRAMES: u32 = 10_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RunResult {
    count: u32,
    sum: u32,
    drops: u32,
    digest: u64,
}

// 两个后端只有通道容量不同；宏在这里反而更绕，直接写两份以保证可读性。
mod cap8 {
    use core::sync::atomic::{AtomicU32, Ordering};

    use echo_arch_mock::MockArch;
    use echo_ipc::Mailbox;
    use echo_sched::CyclicExecutive;

    use super::RunResult;

    static S2C: Mailbox<8> = Mailbox::new();
    static C2A: Mailbox<8> = Mailbox::new();
    static SENSOR_SEQ: AtomicU32 = AtomicU32::new(0);
    static ACTUATOR_COUNT: AtomicU32 = AtomicU32::new(0);
    static ACTUATOR_SUM: AtomicU32 = AtomicU32::new(0);
    static DROPS: AtomicU32 = AtomicU32::new(0);

    fn sensor() {
        let value = SENSOR_SEQ.fetch_add(1, Ordering::Relaxed);
        if S2C.try_send(value).is_err() {
            DROPS.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn control() {
        match S2C.try_recv() {
            Some(value) => {
                if C2A.try_send(value.wrapping_mul(2)).is_err() {
                    DROPS.fetch_add(1, Ordering::Relaxed);
                }
            }
            None => {
                DROPS.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    fn actuator() {
        match C2A.try_recv() {
            Some(value) => {
                ACTUATOR_COUNT.fetch_add(1, Ordering::Relaxed);
                ACTUATOR_SUM.fetch_add(value, Ordering::Relaxed);
            }
            None => {
                DROPS.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    pub fn run(frames: u32) -> RunResult {
        let arch = MockArch::new();
        let exec = CyclicExecutive::new(
            &arch,
            super::task_defs([sensor as fn(), control as fn(), actuator as fn()]),
            super::FRAME_NS,
        )
        .expect("schedule must fit");

        let mut digest = 0u64;
        for _ in 0..frames {
            let stats = exec.run_frame();
            digest = digest.wrapping_mul(1_099_511_628_211)
                ^ stats.frame_start_ns
                ^ stats.slots[0].start_ns
                ^ stats.slots[1].start_ns
                ^ stats.slots[2].start_ns;
            arch.advance(super::FRAME_NS);
        }

        RunResult {
            count: ACTUATOR_COUNT.load(Ordering::Relaxed),
            sum: ACTUATOR_SUM.load(Ordering::Relaxed),
            drops: DROPS.load(Ordering::Relaxed),
            digest,
        }
    }
}

mod cap2 {
    use core::sync::atomic::{AtomicU32, Ordering};

    use echo_arch_mock::MockArch;
    use echo_ipc::Mailbox;
    use echo_sched::CyclicExecutive;

    use super::RunResult;

    static S2C: Mailbox<2> = Mailbox::new();
    static C2A: Mailbox<2> = Mailbox::new();
    static SENSOR_SEQ: AtomicU32 = AtomicU32::new(0);
    static ACTUATOR_COUNT: AtomicU32 = AtomicU32::new(0);
    static ACTUATOR_SUM: AtomicU32 = AtomicU32::new(0);
    static DROPS: AtomicU32 = AtomicU32::new(0);

    fn sensor() {
        let value = SENSOR_SEQ.fetch_add(1, Ordering::Relaxed);
        if S2C.try_send(value).is_err() {
            DROPS.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn control() {
        match S2C.try_recv() {
            Some(value) => {
                if C2A.try_send(value.wrapping_mul(2)).is_err() {
                    DROPS.fetch_add(1, Ordering::Relaxed);
                }
            }
            None => {
                DROPS.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    fn actuator() {
        match C2A.try_recv() {
            Some(value) => {
                ACTUATOR_COUNT.fetch_add(1, Ordering::Relaxed);
                ACTUATOR_SUM.fetch_add(value, Ordering::Relaxed);
            }
            None => {
                DROPS.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    pub fn run(frames: u32) -> RunResult {
        let arch = MockArch::new();
        let exec = CyclicExecutive::new(
            &arch,
            super::task_defs([sensor as fn(), control as fn(), actuator as fn()]),
            super::FRAME_NS,
        )
        .expect("schedule must fit");

        let mut digest = 0u64;
        for _ in 0..frames {
            let stats = exec.run_frame();
            digest = digest.wrapping_mul(1_099_511_628_211)
                ^ stats.frame_start_ns
                ^ stats.slots[0].start_ns
                ^ stats.slots[1].start_ns
                ^ stats.slots[2].start_ns;
            arch.advance(super::FRAME_NS);
        }

        RunResult {
            count: ACTUATOR_COUNT.load(Ordering::Relaxed),
            sum: ACTUATOR_SUM.load(Ordering::Relaxed),
            drops: DROPS.load(Ordering::Relaxed),
            digest,
        }
    }
}

#[test]
fn same_generated_graph_on_two_channel_capacities() {
    let wide = cap8::run(FRAMES);
    let narrow = cap2::run(FRAMES);

    assert_eq!(wide.drops, 0);
    assert_eq!(narrow.drops, 0);
    assert_eq!(wide.count, FRAMES);
    assert_eq!(narrow.count, FRAMES);
    assert_eq!(wide.sum, FRAMES.wrapping_mul(FRAMES - 1));
    assert_eq!(wide, narrow, "schedule/counters/trace digest must match");
}

#[test]
fn generated_constants_are_shared() {
    assert_eq!(FRAME_NS, 1000);
    assert_eq!(NODE_COUNT, 3);
    assert_eq!(NODE_NAMES, ["sensor", "control", "actuator"]);
}
