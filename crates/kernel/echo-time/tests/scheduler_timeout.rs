use echo_sched::preemptive::{PreemptTaskDef, PreemptiveScheduler};
use echo_time::DelayQueue;

fn noop() {}

fn task(name: &'static str, priority: u8) -> PreemptTaskDef {
    PreemptTaskDef::new(name, priority, noop)
}

#[test]
fn timeout_queue_wakes_sleeping_task() {
    let mut scheduler = PreemptiveScheduler::<4>::new();
    let idle = scheduler.declare(task("idle", 0)).unwrap();
    let worker = scheduler.declare(task("worker", 1)).unwrap();
    let mut timeout = DelayQueue::<4>::new();

    assert_eq!(scheduler.pick_next(), Some(worker));
    scheduler.sleep_until(worker, 100).unwrap();
    timeout.insert(100, u16::from(worker)).unwrap();
    assert_eq!(scheduler.pick_next(), Some(idle));

    assert_eq!(timeout.pop_due(99), None);
    assert_eq!(timeout.pop_due(100), Some(u16::from(worker)));
    assert_eq!(scheduler.wake_due(100), 1);
    assert_eq!(scheduler.pick_next(), Some(worker));
}
