use core::sync::atomic::{AtomicU32, Ordering};

use echo_arch_mock::MockArch;
use echo_sched::{CyclicExecutive, FrameStats, SchedError, TaskDef};

fn noop() {}

fn three_tasks() -> [TaskDef; 3] {
    [
        TaskDef::new("sensor", 200, noop),
        TaskDef::new("control", 100, noop),
        TaskDef::new("actuator", 50, noop),
    ]
}

#[test]
fn rejects_zero_frame() {
    let arch = MockArch::new();
    match CyclicExecutive::new(&arch, three_tasks(), 0) {
        Err(SchedError::ZeroFrame) => {}
        other => panic!("expected ZeroFrame, got {:?}", other.map(|_| ())),
    }
}

#[test]
fn rejects_budget_overrun() {
    let arch = MockArch::new();
    let tasks = [TaskDef::new("a", 600, noop), TaskDef::new("b", 500, noop)];
    match CyclicExecutive::new(&arch, tasks, 1000) {
        Err(SchedError::BudgetExceeded {
            used_ns: 1100,
            frame_ns: 1000,
        }) => {}
        other => panic!("expected BudgetExceeded, got {:?}", other.map(|_| ())),
    }
}

#[test]
fn accepts_exact_budget() {
    let arch = MockArch::new();
    let tasks = [TaskDef::new("a", 600, noop), TaskDef::new("b", 400, noop)];
    let exec = CyclicExecutive::new(&arch, tasks, 1000).expect("exact budget must fit");
    assert_eq!(exec.slot_total_ns(), 1000);
    assert_eq!(exec.idle_ns(), 0);
}

#[test]
fn slot_offsets_are_derived_from_budgets() {
    let arch = MockArch::new();
    let exec = CyclicExecutive::new(&arch, three_tasks(), 1000).unwrap();
    let frame = exec.run_frame();

    assert_eq!(frame.frame_start_ns, 0);
    assert_eq!(frame.frame_end_ns, 1000);
    assert_eq!(frame.idle_ns, 650);

    assert_eq!(frame.slots[0].name, "sensor");
    assert_eq!((frame.slots[0].start_ns, frame.slots[0].end_ns), (0, 200));
    assert_eq!(frame.slots[1].name, "control");
    assert_eq!((frame.slots[1].start_ns, frame.slots[1].end_ns), (200, 300));
    assert_eq!(frame.slots[2].name, "actuator");
    assert_eq!((frame.slots[2].start_ns, frame.slots[2].end_ns), (300, 350));
}

#[test]
fn frame_start_advances_by_frame_ns() {
    let arch = MockArch::new();
    let exec = CyclicExecutive::new(&arch, three_tasks(), 1000).unwrap();

    for frame_index in 0..10_000u64 {
        let frame = exec.run_frame();
        assert_eq!(frame.frame_start_ns, frame_index * 1000);
        arch.advance(1000);
    }
}

#[test]
fn two_runs_produce_identical_traces() {
    let arch_a = MockArch::new();
    let arch_b = MockArch::new();
    let exec_a = CyclicExecutive::new(&arch_a, three_tasks(), 1000).unwrap();
    let exec_b = CyclicExecutive::new(&arch_b, three_tasks(), 1000).unwrap();

    for _ in 0..1_000 {
        let a = exec_a.run_frame();
        let b = exec_b.run_frame();
        assert_eq!(a, b);
        arch_a.advance(1000);
        arch_b.advance(1000);
    }
}

#[test]
fn slots_actually_execute_tasks() {
    static COUNTERS: [AtomicU32; 3] = [AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0)];

    fn run_0() {
        COUNTERS[0].fetch_add(1, Ordering::Relaxed);
    }
    fn run_1() {
        COUNTERS[1].fetch_add(1, Ordering::Relaxed);
    }
    fn run_2() {
        COUNTERS[2].fetch_add(1, Ordering::Relaxed);
    }

    for counter in &COUNTERS {
        counter.store(0, Ordering::Relaxed);
    }

    let tasks = [
        TaskDef::new("sensor", 200, run_0),
        TaskDef::new("control", 100, run_1),
        TaskDef::new("actuator", 50, run_2),
    ];
    let arch = MockArch::new();
    let exec = CyclicExecutive::new(&arch, tasks, 1000).unwrap();

    for _ in 0..3 {
        exec.run_frame();
        arch.advance(1000);
    }

    assert_eq!(COUNTERS[0].load(Ordering::Relaxed), 3);
    assert_eq!(COUNTERS[1].load(Ordering::Relaxed), 3);
    assert_eq!(COUNTERS[2].load(Ordering::Relaxed), 3);
}

#[test]
fn empty_schedule_is_allowed() {
    let arch = MockArch::new();
    let exec = CyclicExecutive::<_, 0>::new(&arch, [], 1000).unwrap();
    let frame: FrameStats<0> = exec.run_frame();
    assert_eq!(frame.idle_ns, 1000);
    assert_eq!(frame.frame_end_ns, 1000);
}
