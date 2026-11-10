#![no_std]
#![forbid(unsafe_code)]

//! 循环执行体（Cyclic Executive）——控制域的静态调度。
//!
//! 设计契约见 `docs/11-arch-and-sched.md`。
//!
//! v0 范围：静态槽位、编译期确定的执行顺序、帧内时间偏移可计算；
//! **不含**优先级抢占、实际 WCET 测量、deadline miss 检测（后续里程碑）。

use echo_arch_api::Arch;

/// 一个槽位的定义。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaskDef {
    /// 任务名（用于 trace）。
    pub name: &'static str,
    /// 分配给该槽位的时间预算（纳秒）。
    pub slot_ns: u64,
    /// 任务入口。
    pub run: fn(),
}

impl TaskDef {
    pub const fn new(name: &'static str, slot_ns: u64, run: fn()) -> Self {
        Self { name, slot_ns, run }
    }
}

/// 单个槽位的执行记录。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotStat {
    pub name: &'static str,
    pub start_ns: u64,
    pub end_ns: u64,
}

/// 一帧的执行结果，可直接用于确定性断言。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameStats<const N: usize> {
    pub frame_start_ns: u64,
    pub frame_end_ns: u64,
    pub idle_ns: u64,
    pub slots: [SlotStat; N],
}

/// 构造错误。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchedError {
    /// 帧长必须大于 0。
    ZeroFrame,
    /// 槽位预算之和超过帧长。
    BudgetExceeded { used_ns: u64, frame_ns: u64 },
}

/// 静态循环执行体。
///
/// `SCHEDULE` 是编译期固定的 `[TaskDef; N]`；每帧按数组顺序执行槽位，
/// 槽位起始时刻 = 帧起始时刻 + 前面槽位的预算之和。
pub struct CyclicExecutive<'a, A: Arch, const N: usize> {
    arch: &'a A,
    tasks: [TaskDef; N],
    frame_ns: u64,
    slot_total_ns: u64,
}

impl<'a, A: Arch, const N: usize> CyclicExecutive<'a, A, N> {
    /// 构造并校验：`frame_ns > 0` 且槽位预算之和 ≤ `frame_ns`。
    pub fn new(arch: &'a A, tasks: [TaskDef; N], frame_ns: u64) -> Result<Self, SchedError> {
        if frame_ns == 0 {
            return Err(SchedError::ZeroFrame);
        }

        let mut slot_total_ns = 0u64;
        for task in &tasks {
            slot_total_ns =
                slot_total_ns
                    .checked_add(task.slot_ns)
                    .ok_or(SchedError::BudgetExceeded {
                        used_ns: u64::MAX,
                        frame_ns,
                    })?;
        }

        if slot_total_ns > frame_ns {
            return Err(SchedError::BudgetExceeded {
                used_ns: slot_total_ns,
                frame_ns,
            });
        }

        Ok(Self {
            arch,
            tasks,
            frame_ns,
            slot_total_ns,
        })
    }

    pub fn frame_ns(&self) -> u64 {
        self.frame_ns
    }

    pub fn slot_total_ns(&self) -> u64 {
        self.slot_total_ns
    }

    pub fn idle_ns(&self) -> u64 {
        self.frame_ns - self.slot_total_ns
    }

    pub fn tasks(&self) -> &[TaskDef; N] {
        &self.tasks
    }

    /// 执行一帧。
    ///
    /// 时间语义：读取一次 `Arch::now_ns()` 作为帧起点；槽位结束时刻由
    /// `slot_ns` 预算推导，不读取实际执行时间。时钟推进由调用者负责
    /// （真机由硬件时基推进，host 测试由 `MockArch::advance` 推进）。
    pub fn run_frame(&self) -> FrameStats<N> {
        let frame_start_ns = self.arch.now_ns();
        let mut slots = [SlotStat {
            name: "",
            start_ns: 0,
            end_ns: 0,
        }; N];

        let mut cursor = frame_start_ns;
        for (index, task) in self.tasks.iter().enumerate() {
            let start_ns = cursor;
            (task.run)();
            cursor = cursor.saturating_add(task.slot_ns);
            slots[index] = SlotStat {
                name: task.name,
                start_ns,
                end_ns: cursor,
            };
        }

        FrameStats {
            frame_start_ns,
            frame_end_ns: frame_start_ns.saturating_add(self.frame_ns),
            idle_ns: self.idle_ns(),
            slots,
        }
    }
}
