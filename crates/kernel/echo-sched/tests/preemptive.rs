use echo_sched::SchedError;
use echo_sched::preemptive::{
    MAX_PRIORITIES, MAX_TASKS, PreemptTaskDef, PreemptiveScheduler, TaskState,
};

fn noop() {}

fn task(name: &'static str, priority: u8) -> PreemptTaskDef {
    PreemptTaskDef::new(name, priority, noop)
}

#[test]
fn empty_scheduler_returns_none() {
    let mut scheduler = PreemptiveScheduler::<8>::new();
    assert_eq!(scheduler.pick_next(), None);
}

#[test]
fn highest_priority_wins() {
    let mut scheduler = PreemptiveScheduler::<8>::new();
    scheduler.declare(task("low", 1)).unwrap();
    let high = scheduler.declare(task("high", 5)).unwrap();
    assert_eq!(scheduler.pick_next(), Some(high));
}

#[test]
fn higher_priority_preempts_running_task() {
    let mut scheduler = PreemptiveScheduler::<8>::new();
    let low = scheduler.declare(task("low", 1)).unwrap();
    assert_eq!(scheduler.pick_next(), Some(low));

    let high = scheduler.declare(task("high", 5)).unwrap();
    assert_eq!(scheduler.pick_next(), Some(high));
    assert_eq!(scheduler.state(low), Some(TaskState::Ready));
}

#[test]
fn lower_priority_does_not_preempt() {
    let mut scheduler = PreemptiveScheduler::<8>::new();
    let high = scheduler.declare(task("high", 5)).unwrap();
    assert_eq!(scheduler.pick_next(), Some(high));

    scheduler.declare(task("low", 1)).unwrap();
    assert_eq!(scheduler.pick_next(), Some(high));
}

#[test]
fn same_priority_round_robins() {
    let mut scheduler = PreemptiveScheduler::<8>::new();
    for _ in 0..3 {
        scheduler.declare(task("worker", 2)).unwrap();
    }

    let mut sequence = [0u8; 4];
    for slot in &mut sequence {
        *slot = scheduler.pick_next().expect("ready task");
        scheduler.yield_now();
    }
    assert_eq!(sequence, [0, 1, 2, 0]);
}

#[test]
fn blocked_task_is_skipped() {
    let mut scheduler = PreemptiveScheduler::<8>::new();
    let a = scheduler.declare(task("a", 2)).unwrap();
    let b = scheduler.declare(task("b", 2)).unwrap();
    let c = scheduler.declare(task("c", 2)).unwrap();

    assert_eq!(scheduler.pick_next(), Some(a));
    scheduler.block(b);
    scheduler.yield_now();
    assert_eq!(scheduler.pick_next(), Some(c));
    assert_eq!(scheduler.state(b), Some(TaskState::Blocked));

    scheduler.unblock(b);
    assert_eq!(scheduler.state(b), Some(TaskState::Ready));
}

#[test]
fn priority_change_reschedules() {
    let mut scheduler = PreemptiveScheduler::<8>::new();
    let a = scheduler.declare(task("a", 1)).unwrap();
    let b = scheduler.declare(task("b", 2)).unwrap();

    assert_eq!(scheduler.pick_next(), Some(b));
    scheduler.set_priority(a, 3).unwrap();
    assert_eq!(scheduler.pick_next(), Some(a));
}

#[test]
fn task_table_is_bounded() {
    let mut scheduler = PreemptiveScheduler::<MAX_TASKS>::new();
    for _ in 0..MAX_TASKS {
        scheduler.declare(task("t", 1)).unwrap();
    }
    assert_eq!(
        scheduler.declare(task("overflow", 1)).unwrap_err(),
        SchedError::TaskCapacityExceeded
    );
}

#[test]
fn invalid_priority_is_rejected() {
    let mut scheduler = PreemptiveScheduler::<4>::new();
    assert_eq!(
        scheduler
            .declare(task("bad", MAX_PRIORITIES as u8))
            .unwrap_err(),
        SchedError::InvalidPriority {
            priority: MAX_PRIORITIES as u8
        }
    );
}

#[test]
fn highest_priority_bit_is_usable() {
    let mut scheduler = PreemptiveScheduler::<4>::new();
    let id = scheduler
        .declare(task("top", (MAX_PRIORITIES - 1) as u8))
        .unwrap();
    assert_eq!(scheduler.pick_next(), Some(id));
}
