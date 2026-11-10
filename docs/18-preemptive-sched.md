# 18 · 抢占调度与上下文切换（W7）

**状态**：✅ host 决策逻辑完成；⚠️ Cortex-M 上下文切换为占位（硬件门）｜ **新增测试**：10｜ **累计**：58

> 本文是 `echo_sched::preemptive` 的实现契约。
> 设计背景见 [06 §6.1](06-subsystems.md#61-调度器echo-sched) 与 [ADR-0004](05-design-decisions.md#adr-0004--线程模型作为调度基础async-仅作可选适配层)。

## 1. 范围

**做**：

- 静态优先级抢占的**决策逻辑**（纯 host 可测）；
- 同优先级 RR；
- block / unblock / yield / set_priority；
- 32 优先级 × 32 任务的位图数据结构。

**不做**（后续）：

- ❌ 真实上下文切换（PendSV/PSP/TCB 栈帧）
- ❌ 时间片 tick 抢占（当前 RR 由 `yield_now` 驱动）
- ❌ 优先级继承（M4）
- ❌ idle 任务、栈分配、MPU 隔离
- ❌ ISR 注入（W8 的 `MockArch` 扩展）

## 2. API

```rust
pub struct PreemptiveScheduler<const N: usize>;

impl<const N: usize> PreemptiveScheduler<N> {
    pub const fn new() -> Self;
    pub fn declare(&mut self, def: PreemptTaskDef) -> Result<TaskId, SchedError>;
    pub fn pick_next(&mut self) -> Option<TaskId>;
    pub fn yield_now(&mut self);
    pub fn block(&mut self, id: TaskId);
    pub fn unblock(&mut self, id: TaskId);
    pub fn set_priority(&mut self, id: TaskId, priority: u8) -> Result<(), SchedError>;
    pub fn current(&self) -> Option<TaskId>;
    pub fn state(&self, id: TaskId) -> Option<TaskState>;
    pub fn task_count(&self) -> usize;
}

pub struct PreemptTaskDef {
    pub name: &'static str,
    pub priority: u8,
    pub entry: fn(),
}
```

`TaskId = u8`；`MAX_TASKS = 32`，`MAX_PRIORITIES = 32`。

## 3. 数据结构与复杂度

```
priority_bitmap: u32                     // 位 p = 1 表示优先级 p 有就绪任务
task_mask:       [u64; 32]               // 每个优先级一个 32 位任务位图
rr_cursor:       [u8; 32]                // 每个优先级的 RR 轮转位置
running:         Option<TaskId>
```

| 操作 | 复杂度 | 实现 |
| --- | --- | --- |
| 找最高优先级 | **O(1)** | `31 - leading_zeros(priority_bitmap)` |
| 同优先级选下一个 | **O(1)** | `mask.rotate_right(cursor)` + `trailing_zeros` |
| block / unblock / set_priority | O(1) | 位图置位/清位 |
| 声明任务 | O(N) 找空位（N≤32） | 固定表 |

全部静态存储，无堆分配。

## 4. 调度语义

| 场景 | 行为 |
| --- | --- |
| 更高优先级就绪 | 抢占当前任务（`pick_next` 返回高优先级） |
| 同优先级多个就绪 | 按 `rr_cursor` 轮转；`yield_now` 让出后取下一个 |
| 当前任务优先级**严格高于**所有就绪任务 | 保持当前任务 |
| 无就绪任务 | `None` |
| block | 清就绪位；若正在运行则 `running = None` |
| unblock | 重新置位，状态回 `Ready` |
| set_priority | 迁移位图；blocked 任务不会被错误置为 ready |
| 非法优先级（≥32）/ 表满 | 返回 `SchedError` |

## 5. 测试（10 个）

```bash
cargo test -p echo-sched --test preemptive
```

| 测试 | 证明 |
| --- | --- |
| `empty_scheduler_returns_none` | 空调度器 |
| `highest_priority_wins` | 最高优先级优先 |
| `higher_priority_preempts_running_task` | 高优先级抢占 |
| `lower_priority_does_not_preempt` | 低优先级不抢占 |
| `same_priority_round_robins` | RR 顺序 `0,1,2,0` |
| `blocked_task_is_skipped` | 阻塞任务被跳过，unblock 恢复 |
| `priority_change_reschedules` | 改优先级后重新调度 |
| `task_table_is_bounded` | 32 任务上限 |
| `invalid_priority_is_rejected` | 优先级 32 被拒 |
| `highest_priority_bit_is_usable` | 优先级 31 的 CLZ 边界 |

## 6. Cortex-M 上下文切换：占位，未完成

`crates/arch/echo-arch-cortex-m/src/context.rs` 目前只有：

```asm
PendSV_Handler: bx lr
SVC_Handler:    bx lr
```

**这不是可用的上下文切换**，只保证符号与汇编可通过编译。

真实实现需要：

1. TCB 与栈帧布局（R4–R11、S0–S31、EXC_RETURN）；
2. MSP/PSP 双栈与首次任务启动的伪造异常帧；
3. `SVC` 系统调用入口；
4. PendSV 里的 `stmia/ldmia` 保存/恢复；
5. 栈 canary 与 MPU 守卫。

这些依赖 **H7 真机**，与 W6 硬件门一起处理；host 上无法验证。

## 7. 决策

| # | 决策 | 理由 |
| --- | --- | --- |
| D-026 | 用 `priority_bitmap + per-priority task_mask` | 最高优先级 O(1)，RR O(1)，无链表/无堆 |
| D-027 | 调度决策与 `Arch` 解耦 | 纯 host 可测；上下文切换是另一层 |
| D-028 | PendSV/SVC 先占位、不假装完成 | 真机验证前不把 `bx lr` 写成"上下文切换已实现" |
| D-029 | RR 由 `yield_now` 驱动，暂不做时间片 | 时间片需要 SysTick/Tick 域，W8 再引入 |

## 8. 文件

| 文件 | 内容 |
| --- | --- |
| `crates/kernel/echo-sched/src/preemptive.rs` | 调度器 + 10 个集成测试 |
| `crates/arch/echo-arch-cortex-m/src/context.rs` | PendSV/SVC 占位 |
| `crates/kernel/echo-sched/tests/preemptive.rs` | 行为测试 |

## 9. 下一步（W8）

1. `echo-time`：单调钟、延时链表、`sleep_until` 语义（host + MockArch）；
2. `echo-bench`：P50/P99/P99.9/max 直方图数学与 JSON/CSV 输出（host 可测）；
3. DWT/SysTick 时基的回绕处理（真机门）；
4. 新增 `docs/19-time-and-bench.md`，追加 `W8-001`。
