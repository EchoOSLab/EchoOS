#![no_std]
#![forbid(unsafe_code)]

//! PAL（Platform Abstraction Layer）契约 —— Micro 子集 v0。
//!
//! 设计契约见 `docs/14-pal-contract.md`。
//!
//! **抽象的是语义，不是 OS 原语**：
//! - 不出现 `pthread_create` / `epoll` / `mmap` / `dlopen`；
//! - 任务在编译期声明（[`TaskApi::declare`]），运行期不动态创建；
//! - 内存是静态池（[`MemoryApi`]），热路径零分配；
//! - 通道容量编译期确定（[`ChannelApi`]），非阻塞；
//! - 能力是编译期常量（[`Capabilities`]），不是运行期探测。

/// PAL 契约版本。冻结后只允许加法；破坏性变更必须提升 major。
pub const PAL_VERSION: (u8, u8, u8) = (0, 1, 0);

/// 编译期能力声明。
///
/// 数据流用到不支持的能力时，应在**编译期**报错，而不是运行时崩溃。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capabilities {
    /// 支持编译期静态任务（Micro 必须为 true）。
    pub static_tasks: bool,
    /// 支持运行期动态创建任务（Micro 必须为 false）。
    pub dynamic_tasks: bool,
    /// 支持阻塞睡眠（控制域槽位内必须为 false）。
    pub blocking_sleep: bool,
    /// 支持通用动态分配（Micro 必须为 false）。
    pub dynamic_memory: bool,
    /// 支持有界通道。
    pub channels: bool,
    /// 支持 trace / 事件记录。
    pub trace: bool,
    /// 支持 POSIX 兼容分区（ADR-0020；90 天内恒为 false）。
    pub posix_partition: bool,
}

impl Capabilities {
    /// Micro Profile 的官方能力组合。
    pub const fn micro() -> Self {
        Self {
            static_tasks: true,
            dynamic_tasks: false,
            blocking_sleep: false,
            dynamic_memory: false,
            channels: true,
            trace: true,
            posix_partition: false,
        }
    }
}

/// 平台总入口：五个能力域 + 能力矩阵。
pub trait Platform {
    type Task: TaskApi;
    type Time: TimeApi;
    type Memory: MemoryApi;
    type Channel: ChannelApi;
    type Trace: ObservabilityApi;

    fn capabilities(&self) -> Capabilities;
    fn task(&self) -> &Self::Task;
    fn time(&self) -> &Self::Time;
    fn memory(&self) -> &Self::Memory;
    fn channel(&self) -> &Self::Channel;
    fn trace(&self) -> &Self::Trace;
}

/// 编译期任务声明。
///
/// 由 `dora-codegen` / BSP 在启动阶段提交；运行期不允许再创建任务。
#[derive(Clone, Copy)]
pub struct StaticTask {
    pub name: &'static str,
    pub priority: u8,
    pub entry: fn(),
}

impl StaticTask {
    pub const fn new(name: &'static str, priority: u8, entry: fn()) -> Self {
        Self {
            name,
            priority,
            entry,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskError {
    /// 静态任务表已满。
    CapacityExceeded,
}

/// 任务域（静态语义）。
pub trait TaskApi {
    type TaskId: Copy + Eq + core::fmt::Debug;

    /// 当前任务 id（单任务 v0 恒为 0）。
    fn current(&self) -> Self::TaskId;

    /// 已声明的静态任务数。
    fn task_count(&self) -> usize;

    /// 让出 CPU（协作式；抢占由调度器负责）。
    fn yield_now(&self);

    /// 声明一个静态任务；只允许在启动阶段调用。
    fn declare(&self, task: StaticTask) -> Result<Self::TaskId, TaskError>;
}

/// 时间域。
pub trait TimeApi {
    /// 单调纳秒时基。
    fn now_ns(&self) -> u64;
}

/// 静态池中的一个块。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PoolBlock {
    pub index: u16,
    pub len: u16,
}

/// 内存域（静态池语义，无通用分配器）。
pub trait MemoryApi {
    /// 池总字节数。
    fn pool_capacity(&self) -> usize;
    /// 已用字节数。
    fn pool_used(&self) -> usize;
    /// 申请一个块；`len` 超过单块大小或池耗尽时返回 `None`。
    fn try_alloc(&self, len: usize) -> Option<PoolBlock>;
    /// 归还块；重复归还应是幂等的。
    fn free(&self, block: PoolBlock);
}

/// 通道域（有界、非阻塞、单通道 v0）。
pub trait ChannelApi {
    fn capacity(&self) -> usize;
    fn try_send(&self, value: u32) -> Result<(), u32>;
    fn try_recv(&self) -> Option<u32>;
}

/// 可观测性域。
pub trait ObservabilityApi {
    /// 记录一个事件；`id` 与 `value` 由上层解释。
    fn trace_event(&self, id: u16, value: u32);
    /// 已记录事件数。
    fn event_count(&self) -> usize;
    /// 因缓冲满而丢弃的事件数。
    fn dropped_events(&self) -> usize;
}
