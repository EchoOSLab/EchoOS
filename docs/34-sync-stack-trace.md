# 34 · 等待队列、栈溢出报告与 trace ring（Q2/Q3）

**状态**：Q2-11 ✅、Q3-01 ✅；Q2-07/08/09 部分
**测试**：101 passed（本轮新增 6）

## 1. `echo-sync` 等待队列

新增：

```rust
pub struct WaitQueue<const N: usize>;
pub enum LockResult { Acquired, Waiting, QueueFull }
pub enum UnlockResult { Released, Woken(u8), InvalidOwner }
pub enum WaitResult { Acquired, Waiting, QueueFull }
```

### Mutex

- `lock_or_wait(task)`：owner 空闲则获取；否则入队等待；
- `unlock_and_wake(task)`：合法 owner 解锁；有等待者则把 owner 直接交给队首；
- `waiting()`：等待者数量；
- 原 `try_lock/unlock` 保留，向后兼容。

### Semaphore

- `wait_or_queue(task)`：计数 >0 则减一；否则入队；
- `post_and_wake()`：优先唤醒队首；没有等待者才增加计数；
- 原 `post/try_wait` 保留。

### 限制

- 等待队列是**单核固定容量**（8）；
- 还没有把“等待任务”接到 QEMU 调度器的 block/wake；
- 没有优先级继承；
- Event 仍是 try-only。

## 2. 栈溢出报告（Q2-11）

`sched_main` 的 canary 检查从 `bool` 改成返回任务索引：

```rust
fn canary_failure() -> Option<usize>;
```

失败输出：

```text
EchoOS MPS2: STACK OVERFLOW task=0
```

验证方式：`stack-fault` feature 在第 500 次切换时故意破坏 task0 的 canary：

```bash
cargo xtask stack
PASS  qemu-mps2 stack overflow report
EchoOS MPS2: STACK OVERFLOW task=0
```

**边界**：这是 canary 破坏注入，不是真实栈溢出；真实溢出仍需 MPU/保护页。

## 3. trace ring（Q3-01）

`echo-trace` 新增：

```rust
pub struct Event { pub id: u16, pub value: u32, pub timestamp_ns: u64 }
pub struct EventRing<const N: usize>;
```

- `push` 满时丢弃并计数；
- `pop` FIFO；
- `len/is_empty/dropped/clear`；
- QEMU PAL 的 `ObservabilityApi` 已改用 `RefCell<EventRing<16>>`，`cargo xtask pal` 仍通过。

测试：

```bash
cargo test -p echo-trace --test ring   3 passed
```

## 3.1 QEMU mutex handoff（本轮）

`sched_main` 里增加了确定性 mutex 竞争：

- task0 先 `lock_or_wait(0)` 成为 owner，设置 `MUTEX_OWNER_READY`；
- task1 等 owner ready 后 `lock_or_wait(1)` → `Waiting`，进入 `block_self`；
- task0 等 task1 真正 `Blocked` 后 `unlock_and_wake(0)`；
- `unlock_and_wake` 把 owner 直接交给 task1，并 `sched.unblock(1)`；
- task1 醒来后已持有锁，再 unlock。

输出增加 `mutex=1`：

```text
EchoOS MPS2: sched OK t0=... t1=... t2=... idle=100 wake=1 mutex=1 ticks=... switches=20000
```

**调试中发现的关键问题**：初版会出现间歇性
`PANIC at crates/kernel/echo-sched/src/preemptive.rs:242`（`transition_to` 索引越界）。
原因是 **SysTick 可以打断线程态/PendSV 中的调度器修改**，导致 `priority_bitmap` 与
`task_mask` 不一致，`pick_next` 算出越界 id。

修复：

1. 线程态 `sleep/block/set_priority/unblock` 用 PRIMASK 临界区包住；
2. PendSV 优先级改为 `0x80`，SysTick 改为 `0xFF`，SysTick 不再抢占 PendSV；
3. mutex 使用“等 waiter 真正 Blocked 再 unlock”的顺序，避免丢唤醒。

修复后连续 15 次 QEMU 运行均 `sched OK`。

## 4. 验证

```text
cargo test                 101 passed
cargo xtask sched          PASS
cargo xtask pal            PASS
cargo xtask stack          PASS
cargo xtask fault          PASS
cargo xtask icount         PASS
cargo xtask graph          PASS
cargo xtask hello          四目标 PASS
```

## 5. 仍缺

| ID | 任务 | 状态 |
|---|---|---|
| Q2-07/08/09 | 等待队列接入 QEMU block/wake、优先级继承、ISR-safe | 🟡 队列 API 完成 |
| Q2-04/05 | 优先级继承 / deadline | ⬜ |
| Q2-12/13/14 | MPU/NVIC/ISR-first | ⬜ |
| Q2-15 | 时间队列 + timeout 集成 | ⬜ |
| Q3-05 | GitHub Actions 首跑确认 | 🟡 |
| Q3-06/07 | 文档 lint、alpha tag | ⬜ |

## 6. 文件

| 文件 | 内容 |
|---|---|
| `crates/kernel/echo-sync/src/lib.rs` | WaitQueue / Mutex / Semaphore |
| `crates/kernel/echo-sync/tests/sync.rs` | 6 个测试 |
| `crates/core/echo-trace/src/lib.rs` | EventRing |
| `crates/core/echo-trace/tests/ring.rs` | 3 个测试 |
| `bsp/echo-bsp-qemu-mps2/src/sched_main.rs` | 栈溢出报告 |
| `bsp/echo-bsp-qemu-mps2/src/pal_main.rs` | trace ring 接入 |
| `tools/scripts/qemu-stack.sh` | Q2-11 验收 |
