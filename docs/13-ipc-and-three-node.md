# 13 · Mailbox 与 3 节点图（v0）

**状态**：已实现（W2，2026-11-10）｜ **新增测试**：11 个｜ **累计**：24 个 host 测试全绿

> 本文是 `echo-ipc` 与 3 节点图的实现契约。
> 子系统背景见 [06 §6.5](06-subsystems.md#65-ipcecho-ipc)；
> 里程碑与门槛见 [10-development-plan](10-development-plan.md)。

## 1. 范围

v0 只做**一件事**：在单生产者/单消费者之间，用非阻塞、零 `unsafe` 的有界邮箱把 3 个任务串起来，并证明 10⁶ 帧、0 丢包、顺序正确。

**做**：

- 有界 SPSC 邮箱，容量 `N - 1`（保留一个空槽区分满/空）；
- `try_send` / `try_recv` 非阻塞；
- 全部 `AtomicU32` 实现，`#![forbid(unsafe_code)]`，可放进 `static`；
- 与 `CyclicExecutive` 组合成 sensor → control → actuator 的 3 节点图。

**不做**（后续里程碑）：

- ❌ 阻塞发送/接收、超时（M4）
- ❌ 多生产者 / 多消费者（M4）
- ❌ 泛型载荷 `T`（M4；v0 固定 `u32`）
- ❌ 所有权转移（`T: !Copy`）与堆分配
- ❌ ISR 唤醒任务、优先级继承（M4）
- ❌ typed channel / `Queue` / `Channel` 的完整 API

## 2. API

```rust
pub struct Mailbox<const N: usize> { /* AtomicU32 ring */ }

impl<const N: usize> Mailbox<N> {
    pub const fn new() -> Self;
    pub const fn capacity(&self) -> usize;   // N - 1
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn is_full(&self) -> bool;
    pub fn try_send(&self, value: u32) -> Result<(), u32>;
    pub fn try_recv(&self) -> Option<u32>;
}
```

- `N` 必须大于 0；`N = 0` 在编译期常量上下文会提前失败。
- `Mailbox` 由原子字段组成，自动 `Sync`，可直接：

```rust
static S2C: Mailbox<8> = Mailbox::new();
```

## 3. 环形语义

```
        head（生产者写）
         │
         ▼
  ┌───┬───┬───┬───┐
  │ 0 │ 1 │ 2 │ 3 │   N = 4，容量 = 3
  └───┴───┴───┴───┘
         ▲
         │
        tail（消费者读）
```

| 状态 | 判定 | 结果 |
| --- | --- | --- |
| 空 | `head == tail` | `try_recv() == None` |
| 满 | `(head + 1) % N == tail` | `try_send(v) == Err(v)` |
| 元素数 | `(head + N - tail) % N` | `len()` |

**内存序**：

| 操作 | 顺序 | 原因 |
| --- | --- | --- |
| 生产者写 payload | `Relaxed` | 只被 head 的 Release 保护 |
| 生产者更新 head | `Release` | 保证 payload 先于 head 可见 |
| 消费者读 head | `Acquire` | 保证看到 payload |
| 消费者更新 tail | `Release` | 通知生产者槽位可复用 |
| 生产者读 tail | `Acquire` | 保证不覆盖未读数据 |

## 4. 3 节点图

```
sensor ──► Mailbox<8> ──► control ──► Mailbox<8> ──► actuator
  │                          │                          │
  └─ seq++                   └─ value * 2                └─ count++, sum += value
```

- 三个任务放在同一个 `CyclicExecutive` 帧里：`sensor` → `control` → `actuator`；
- 每帧每个任务各执行一次，管道当帧贯通；
- 1,000,000 帧后：
  - `sensor.seq == 1_000_000`
  - `actuator.count == 1_000_000`
  - `actuator.sum == 1e6 × (1e6 − 1)`（对 `u32` 取模）
  - `drops == 0`
  - 两个邮箱均为空

**为什么用 `u32` 载荷**：`AtomicU32` 在所有 32 位目标（thumbv7m/riscv32）上可用；`AtomicU64` 在没有 64 位原子操作的目标上不可用。泛型载荷留到 M4 的池化消息。

## 5. 测试（新增 11 个）

```bash
cargo test
```

| 位置 | 测试 | 证明 |
| --- | --- | --- |
| `echo-ipc` 单测 | `starts_empty` | 初始为空 |
| | `capacity_is_n_minus_one` | 容量语义 |
| | `fifo_order_is_preserved` | FIFO |
| | `full_returns_the_value` | 满时归还值 |
| | `empty_returns_none` | 空时无假数据 |
| | `wraparound_preserves_order` | 跨环顺序 |
| | `len_tracks_send_and_recv` | 计数正确 |
| | `clear_resets_indices` | 测试用清空 |
| | `mailbox_is_sync` | 可放进 `static` |
| `echo-ipc` 集成 | `spsc_across_two_threads_preserves_order` | 真双线程 100k 条 FIFO |
| `echo-conformance` | `three_node_graph_runs_one_million_frames_without_drops` | 10⁶ 帧、0 丢包、和正确 |

## 6. 决策

| # | 决策 | 理由 |
| --- | --- | --- |
| D-005 | v0 载荷固定 `u32`，不引入泛型 `T` | `AtomicU32` 在 32 位目标可用；泛型需要池化所有权，属 M4 |
| D-006 | 保留一个空槽区分满/空 | 无需额外 count 原子；代价是容量 `N-1` |
| D-007 | 全部原子 + `forbid(unsafe_code)` | 守住 R3 的 `echo-ipc` unsafe = 0；可安全放进 `static` |
| D-008 | 3 节点图放 `crates/test/echo-conformance` | 跨 crate 的验收用例集中管理，后续 PAL conformance 也放这里 |

## 7. 文件

| 文件 | 内容 |
| --- | --- |
| `crates/kernel/echo-ipc/src/lib.rs` | `Mailbox` + 9 个单测 |
| `crates/kernel/echo-ipc/tests/spsc.rs` | 双线程 SPSC 测试 |
| `crates/test/echo-conformance/tests/three_node.rs` | 3 节点图 10⁶ 帧 |

## 8. 下一步（W3）

1. PAL 5 个能力域（Task/Time/Memory/Channel/Observability）的 trait 与 mock 后端；
2. `echo-conformance` 增加 PAL 用例骨架；
3. 与 DoraOS 侧对齐 `dora-pal-conformance` 的用例命名，**契约以测试为准**。
