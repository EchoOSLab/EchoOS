use core::sync::atomic::{AtomicU32, Ordering};

use echo_arch_mock::MockArch;
use echo_ipc::Mailbox;
use echo_sched::{CyclicExecutive, TaskDef};

static SENSOR_TO_CONTROL: Mailbox<8> = Mailbox::new();
static CONTROL_TO_ACTUATOR: Mailbox<8> = Mailbox::new();

static SENSOR_SEQ: AtomicU32 = AtomicU32::new(0);
static ACTUATOR_COUNT: AtomicU32 = AtomicU32::new(0);
static ACTUATOR_SUM: AtomicU32 = AtomicU32::new(0);
static DROPS: AtomicU32 = AtomicU32::new(0);

fn sensor() {
    let value = SENSOR_SEQ.fetch_add(1, Ordering::Relaxed);
    if SENSOR_TO_CONTROL.try_send(value).is_err() {
        DROPS.fetch_add(1, Ordering::Relaxed);
    }
}

fn control() {
    match SENSOR_TO_CONTROL.try_recv() {
        Some(value) => {
            if CONTROL_TO_ACTUATOR.try_send(value.wrapping_mul(2)).is_err() {
                DROPS.fetch_add(1, Ordering::Relaxed);
            }
        }
        None => {
            DROPS.fetch_add(1, Ordering::Relaxed);
        }
    }
}

fn actuator() {
    match CONTROL_TO_ACTUATOR.try_recv() {
        Some(value) => {
            ACTUATOR_COUNT.fetch_add(1, Ordering::Relaxed);
            ACTUATOR_SUM.fetch_add(value, Ordering::Relaxed);
        }
        None => {
            DROPS.fetch_add(1, Ordering::Relaxed);
        }
    }
}

fn reset() {
    SENSOR_SEQ.store(0, Ordering::Relaxed);
    ACTUATOR_COUNT.store(0, Ordering::Relaxed);
    ACTUATOR_SUM.store(0, Ordering::Relaxed);
    DROPS.store(0, Ordering::Relaxed);
    while SENSOR_TO_CONTROL.try_recv().is_some() {}
    while CONTROL_TO_ACTUATOR.try_recv().is_some() {}
}

/// sensor → control → actuator，10⁶ 帧，0 丢包。
#[test]
fn three_node_graph_runs_one_million_frames_without_drops() {
    const FRAMES: u32 = 1_000_000;

    reset();

    let arch = MockArch::new();
    let exec = CyclicExecutive::new(
        &arch,
        [
            TaskDef::new("sensor", 200, sensor),
            TaskDef::new("control", 100, control),
            TaskDef::new("actuator", 50, actuator),
        ],
        1000,
    )
    .expect("schedule must fit");

    for frame in 0..FRAMES {
        let stats = exec.run_frame();
        if frame % 1000 == 0 {
            assert_eq!(stats.frame_start_ns, u64::from(frame) * 1000);
        }
        arch.advance(1000);
    }

    assert_eq!(SENSOR_SEQ.load(Ordering::Relaxed), FRAMES);
    assert_eq!(ACTUATOR_COUNT.load(Ordering::Relaxed), FRAMES);
    assert_eq!(
        ACTUATOR_SUM.load(Ordering::Relaxed),
        FRAMES.wrapping_mul(FRAMES - 1),
        "sum of 2*i for i in 0..FRAMES must match"
    );
    assert_eq!(DROPS.load(Ordering::Relaxed), 0, "no drops allowed");
    assert!(SENSOR_TO_CONTROL.is_empty());
    assert!(CONTROL_TO_ACTUATOR.is_empty());
}
