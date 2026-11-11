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
