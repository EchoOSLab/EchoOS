//! 静态优先级抢占调度（服务域，W7）。
//!
//! 设计契约见 `docs/18-preemptive-sched.md`。
//!
//! 纯决策逻辑，不依赖具体架构；上下文切换由 `echo-arch-cortex-m` 的
//! PendSV 实现（当前为占位）。

use crate::SchedError;

pub const MAX_TASKS: usize = 32;
pub const MAX_PRIORITIES: usize = 32;
pub type TaskId = u8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,
}

#[derive(Clone, Copy, Debug)]
pub struct PreemptTaskDef {
    pub name: &'static str,
    pub priority: u8,
    pub entry: fn(),
}

impl PreemptTaskDef {
    pub const fn new(name: &'static str, priority: u8, entry: fn()) -> Self {
        Self {
            name,
            priority,
            entry,
        }
    }
}

#[derive(Clone, Copy)]
struct TaskSlot {
    def: PreemptTaskDef,
    state: TaskState,
}

/// 静态优先级抢占调度器（固定任务表）。
pub struct PreemptiveScheduler<const N: usize> {
    tasks: [Option<TaskSlot>; N],
    task_mask: [u64; MAX_PRIORITIES],
    priority_bitmap: u32,
    rr_cursor: [u8; MAX_PRIORITIES],
    running: Option<TaskId>,
    count: usize,
}

impl<const N: usize> PreemptiveScheduler<N> {
    pub const fn new() -> Self {
        Self {
            tasks: [None; N],
            task_mask: [0; MAX_PRIORITIES],
            priority_bitmap: 0,
            rr_cursor: [0; MAX_PRIORITIES],
            running: None,
            count: 0,
        }
    }

    pub fn task_count(&self) -> usize {
        self.count
    }

    pub fn current(&self) -> Option<TaskId> {
        self.running
    }

    pub fn state(&self, id: TaskId) -> Option<TaskState> {
        self.tasks[id as usize].map(|slot| slot.state)
    }

    pub fn declare(&mut self, def: PreemptTaskDef) -> Result<TaskId, SchedError> {
        if def.priority as usize >= MAX_PRIORITIES {
            return Err(SchedError::InvalidPriority {
                priority: def.priority,
            });
        }
        if self.count >= N {
            return Err(SchedError::TaskCapacityExceeded);
        }

        let Some(id) = self.tasks.iter().position(Option::is_none) else {
            return Err(SchedError::TaskCapacityExceeded);
        };
        if id >= MAX_TASKS {
            return Err(SchedError::TaskCapacityExceeded);
        }

        self.tasks[id] = Some(TaskSlot {
            def,
            state: TaskState::Ready,
        });
        self.mark_ready(id, def.priority);
        self.count += 1;
        Ok(id as TaskId)
    }

    /// 选择下一个要运行的任务；无就绪任务时返回 `None`。
    ///
    /// 规则：最高优先级就绪任务优先；同优先级按 RR 轮转；
    /// 若当前运行任务优先级严格高于最高就绪任务，保持当前任务。
    pub fn pick_next(&mut self) -> Option<TaskId> {
        if self.priority_bitmap == 0 {
            self.running = None;
            return None;
        }

        let priority = (31 - self.priority_bitmap.leading_zeros()) as usize;

        if let Some(run) = self.running
            && let Some(slot) = self.tasks[run as usize]
            && slot.def.priority as usize > priority
        {
            return Some(run);
        }

        let mask = self.task_mask[priority];
        let cursor = u32::from(self.rr_cursor[priority]) % 32;
        let rotated = mask.rotate_right(cursor);
        let offset = rotated.trailing_zeros();
        let id = ((cursor + offset) % 32) as usize;
        self.rr_cursor[priority] = ((id as u32 + 1) % 32) as u8;

        self.transition_to(id)
    }

    /// 当前任务主动让出 CPU。
    pub fn yield_now(&mut self) {
        if let Some(run) = self.running {
            if let Some(slot) = self.tasks[run as usize].as_mut() {
                slot.state = TaskState::Ready;
            }
            self.running = None;
        }
    }

    pub fn block(&mut self, id: TaskId) {
        let Some(slot) = self.tasks[id as usize].as_mut() else {
            return;
        };
        let priority = slot.def.priority;
        slot.state = TaskState::Blocked;
        self.clear_ready(id as usize, priority);
        if self.running == Some(id) {
            self.running = None;
        }
    }

    pub fn unblock(&mut self, id: TaskId) {
        let Some(slot) = self.tasks[id as usize].as_mut() else {
            return;
        };
        let priority = slot.def.priority;
        slot.state = TaskState::Ready;
        self.mark_ready(id as usize, priority);
    }

    pub fn set_priority(&mut self, id: TaskId, priority: u8) -> Result<(), SchedError> {
        if priority as usize >= MAX_PRIORITIES {
            return Err(SchedError::InvalidPriority { priority });
        }
        let Some(slot) = self.tasks[id as usize].as_mut() else {
            return Err(SchedError::TaskCapacityExceeded);
        };
        let old = slot.def.priority;
        let blocked = slot.state == TaskState::Blocked;
        slot.def.priority = priority;

        self.clear_ready(id as usize, old);
        if !blocked {
            self.mark_ready(id as usize, priority);
        }
        Ok(())
    }

    fn transition_to(&mut self, id: usize) -> Option<TaskId> {
        if let Some(run) = self.running
            && run as usize != id
            && let Some(slot) = self.tasks[run as usize].as_mut()
            && slot.state == TaskState::Running
        {
            slot.state = TaskState::Ready;
        }
        if let Some(slot) = self.tasks[id].as_mut() {
            slot.state = TaskState::Running;
        }
        self.running = Some(id as TaskId);
        Some(id as TaskId)
    }

    fn mark_ready(&mut self, id: usize, priority: u8) {
        let bit = 1u64 << id;
        self.task_mask[priority as usize] |= bit;
        self.priority_bitmap |= 1u32 << priority;
    }

    fn clear_ready(&mut self, id: usize, priority: u8) {
        let bit = 1u64 << id;
        self.task_mask[priority as usize] &= !bit;
        if self.task_mask[priority as usize] == 0 {
            self.priority_bitmap &= !(1u32 << priority);
        }
    }
}

impl<const N: usize> Default for PreemptiveScheduler<N> {
    fn default() -> Self {
        Self::new()
    }
}
