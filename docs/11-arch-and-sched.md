# 11 · Arch 抽象与循环执行体（v0 实现契约）

**状态**：已实现（W1，2026-11-10）｜ **对应里程碑**：M0/M2 最小集｜ **测试**：13 个 host 测试全绿

> 本文是 `echo-arch-api` / `echo-arch-mock` / `echo-sched` 三个 crate 的**实现契约**。
> 设计动机见 [ADR-0002](05-design-decisions.md#adr-0002--内核泛型化-over-arch-trait) 与
> [ADR-0016](05-design-decisions.md#adr-0016--循环执行体作为一等调度模式)；
> 子系统背景见 [06 §6.1](06-subsystems.md#61-调度器echo-sched)。

## 1. `trait Arch`（v0）

```rust
pub trait Arch {
    fn now_ns(&self) -> u64;
}
```

**为什么只有 `now_ns`**：按"消费者驱动"原则，只抽象当前有消费者的能力。

| 能力 | 加入时机 | 消费者 |
| --- | --- | --- |
| `now_ns` | v0（已加入） | 循环执行体、bench、deadline 检测 |
| 中断保存/恢复 | M2 | `echo-critical`、抢占调度 |
| 上下文切换 | M2 | `echo-arch-cortex-m` |
| 缓存/MPU 操作 | M5 | `echo-mm` |

**不变量**：

1. 单调递增（实现负责回绕处理；32 位计数器回绕在 M3 测试）。
2. 单位纳秒；不承诺与墙钟对齐。
3. 时间推进由硬件/调用者负责，`Arch` 本身**不推进时间**。

## 2. `MockArch`

```rust
pub struct MockArch { now_ns: Cell<u64> }

impl MockArch {
    pub const fn new() -> Self;
    pub const fn with_time(now_ns: u64) -> Self;
    pub fn advance(&self, ns: u64);   // 饱和到 u64::MAX
    pub fn set(&self, now_ns: u64);   // 用于回绕/边界测试
}
```

- **唯一时间来源是显式 `advance`/`set`**，不读墙钟 → 测试可完全复现。
- `advance` 饱和，保证单调；`set` 允许倒退，仅供测试回绕。
- `Cell` 意味着 `!Sync`：测试单线程使用，不用于并发场景。

## 3. `CyclicExecutive`

### 3.1 API

```rust
pub struct TaskDef {
    pub name: &'static str,
    pub slot_ns: u64,
    pub run: fn(),
}

pub struct SlotStat {
    pub name: &'static str,
    pub start_ns: u64,
    pub end_ns: u64,
}

pub struct FrameStats<const N: usize> {
    pub frame_start_ns: u64,
    pub frame_end_ns: u64,
    pub idle_ns: u64,
    pub slots: [SlotStat; N],
}

pub enum SchedError {
    ZeroFrame,
    BudgetExceeded { used_ns: u64, frame_ns: u64 },
}

impl<'a, A: Arch, const N: usize> CyclicExecutive<'a, A, N> {
    pub fn new(arch: &'a A, tasks: [TaskDef; N], frame_ns: u64)
        -> Result<Self, SchedError>;
    pub fn run_frame(&self) -> FrameStats<N>;
    pub fn frame_ns(&self) -> u64;
    pub fn slot_total_ns(&self) -> u64;
    pub fn idle_ns(&self) -> u64;
    pub fn tasks(&self) -> &[TaskDef; N];
}
```

### 3.2 时间模型

```
frame_start = arch.now_ns()

slot[0].start = frame_start
slot[1].start = slot[0].end
slot[i].start = frame_start + Σ slot[0..i].slot_ns
slot[i].end   = slot[i].start + slot[i].slot_ns

frame_end = frame_start + frame_ns
idle_ns   = frame_ns - Σ slot[i].slot_ns
```

- 每帧**只读一次时钟**；槽位偏移由预算推导，不读实际执行时间。
- 时钟推进由调用者负责：真机由硬件时基推进，host 测试由 `MockArch::advance` 推进。
- 因此同一份调度表在 host 与真机上的**语义一致**，只是时间来源不同。

### 3.3 不变量与校验

| 不变量 | 检查 |
| --- | --- |
| `frame_ns > 0` | 构造时返回 `ZeroFrame` |
| `Σ slot_ns ≤ frame_ns` | 构造时返回 `BudgetExceeded`（含实际值） |
| 槽位按数组顺序执行 | `run_frame` 顺序遍历 |
| 时间不回退 | `saturating_add`，且 `Arch` 契约要求单调 |
| 空调度表合法 | `N = 0` 时整帧 idle |

### 3.4 `slot_ns` 的语义

**是时间预算，不是实测执行时间。**

- v0 用预算推进时间，目的是先固定**调度语义**；
- 真正的执行时间与超限检测在 M3/W8 引入（读 DWT/硬件计数器）；
- 这也是后续 `deadline miss` 与 `FrameStats` 扩展的入口。

### 3.5 明确限制（v0 不做）

- ❌ 优先级抢占（`run_frame` 内不被抢占）
- ❌ 实际 WCET / 超限检测
- ❌ 任务间阻塞/唤醒
- ❌ 每任务栈、上下文切换
- ❌ 多帧统计聚合

这些按 [10-development-plan](10-development-plan.md) 的 W7/W8 引入。

## 4. 测试（13 个）

复现：

```bash
cargo test
```

| 模块 | 测试 | 证明 |
| --- | --- | --- |
| `echo-arch-mock` | `starts_at_zero` | 初始时刻为 0 |
| | `advance_accumulates` | 时间按显式推进累加 |
| | `set_overrides` | 可设置边界时刻 |
| | `advance_saturates` | 不回绕、不 panic |
| | `two_mocks_are_independent` | 实例之间无共享状态 |
| `echo-sched` | `rejects_zero_frame` | 非法帧长被拒 |
| | `rejects_budget_overrun` | 预算超限被拒且带数值 |
| | `accepts_exact_budget` | 预算恰好等于帧长合法 |
| | `slot_offsets_are_derived_from_budgets` | 槽位起点/终点/空闲时间正确 |
| | `frame_start_advances_by_frame_ns` | 10⁴ 帧无相位漂移 |
| | `two_runs_produce_identical_traces` | 两次运行 trace 完全一致 |
| | `slots_actually_execute_tasks` | 槽位确实调用任务函数 |
| | `empty_schedule_is_allowed` | `N=0` 合法 |

> 没有用 `Vec` 或墙钟；测试本身也是 `no_std` 友好的。

## 5. 文件

| 文件 | 内容 |
| --- | --- |
| `crates/arch/echo-arch-api/src/lib.rs` | `trait Arch` |
| `crates/arch/echo-arch-mock/src/lib.rs` | `MockArch` + 5 个单测 |
| `crates/kernel/echo-sched/src/lib.rs` | `TaskDef` / `SlotStat` / `FrameStats` / `CyclicExecutive` |
| `crates/kernel/echo-sched/tests/cyclic.rs` | 8 个集成测试 |

## 6. 本步决策

| # | 决策 | 理由 |
| --- | --- | --- |
| D-001 | `echo-util` 延后到第一个消费者 | bitmap 等就绪队列（M2 抢占），ring 等 IPC/trace；先写工具库会脱离验证 |
| D-002 | `run_frame` 只读一次时钟 | 保证 host 与真机语义一致；时间推进由调用者负责 |
| D-003 | `slot_ns` 是预算而非实测 | v0 先固定调度语义，WCET 在 M3 引入 |
| D-004 | 集成测试放 `tests/` | `no_std` crate 的测试 harness 更干净，且只测公开 API |

## 7. 下一步

W2 已完成：`echo-ipc` 的 `Mailbox` + 3 节点图（10⁶ 帧、0 丢包），见 [13](13-ipc-and-three-node.md)。
下一步 W3：PAL 5 域 + conformance 骨架。
