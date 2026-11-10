use echo_arch_mock::MockArch;
use echo_sched::{CyclicExecutive, TaskDef};

include!("data/generated_three_node.rs");

fn noop() {}

fn hand_written() -> [TaskDef; 3] {
    [
        TaskDef::new("sensor", 200, noop),
        TaskDef::new("control", 100, noop),
        TaskDef::new("actuator", 50, noop),
    ]
}

#[test]
fn generated_constants_match_yaml() {
    assert_eq!(FRAME_NS, 1000);
    assert_eq!(NODE_COUNT, 3);
    assert_eq!(NODE_NAMES, ["sensor", "control", "actuator"]);
    assert_eq!(NODE_SLOTS_NS, [200, 100, 50]);
    assert_eq!(EDGES, [("sensor", "control"), ("control", "actuator")]);
}

#[test]
fn generated_schedule_matches_hand_written() {
    let arch = MockArch::new();
    let generated = CyclicExecutive::new(&arch, task_defs([noop; NODE_COUNT]), FRAME_NS)
        .expect("generated schedule must fit");
    let hand = CyclicExecutive::new(&arch, hand_written(), FRAME_NS)
        .expect("hand-written schedule must fit");

    for _ in 0..1_000 {
        let generated_frame = generated.run_frame();
        let hand_frame = hand.run_frame();
        assert_eq!(generated_frame, hand_frame);
        arch.advance(FRAME_NS);
    }
}
