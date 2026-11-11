# 29 · 调度 sleep/wake 与同步原语（Q2 部分）

**状态**：🟡 Q2 进行中（host 逻辑完成；QEMU 运行时集成待做）
**测试**：89 passed（Q2 新增 7）

> Q2 分两步：
> 1. **host 逻辑**：sleep/wake/idle + 同步原语（本文）；
> 2. **QEMU 集成**：idle 任务、SysTick 时间片、block/sleep、PAL QEMU、内存池。

## 1. 调度器新增

`echo-sched::preemptive` 增加：

| API / 状态 | 语义 |
|---|---|
| `TaskState::Sleeping` | 定时睡眠，不参与就绪位图 |
| `sleep_until(id, deadline_ns)` | 清就绪位、记录 deadline、若正在运行则清 `running` |
| `wake_due(now_ns) -> usize` | 唤醒所有 `deadline <= now` 的任务，返回数量 |
| `is_sleeping(id)` | 查询睡眠状态 |
| `block(id)` | 阻塞，并清除 sleep 状态 |
| `unblock(id)` | 恢复就绪，并清除 sleep 状态 |

**idle 的约定**：idle 是一个 **优先级 0 的普通静态任务**，始终 Ready。  
没有其他就绪任务时，`pick_next` 自然选中它；不需要特殊 idle 分支。

## 2. 语义

```text
sleep_until(id, 100)
  → state = Sleeping
  → 从 ready 位图移除
  → running = None（如果当前任务）

wake_due(now)
  for each sleeping:
      if wake_at <= now:
          state = Ready
          mark_ready(priority)
```

规则：

- `block` 优先于 `sleep`：block 会清除 sleeping 位与 deadline；
- `set_priority` 对 Sleeping 任务只改优先级，不把它错误置为 Ready；
- 同优先级 RR 规则不变；
- 唤醒后仍需 `pick_next` 选择，不直接“抢跑”。

## 3. 测试（4 个新增）

```bash
cargo test -p echo-sched --test preemptive
```

| 测试 | 证明 |
|---|---|
| `idle_task_runs_when_worker_blocks` | block 后 idle 被调度 |
| `sleeping_task_wakes_at_deadline` | 99 不醒、100 醒 |
| `block_clears_sleep_state` | block 覆盖 sleep |
| `priority_change_keeps_sleeping_task_parked` | 改优先级不会误唤醒 |

## 4. `echo-sync`（host 版）

新增 crate：`crates/kernel/echo-sync`。

| 原语 | API | 说明 |
|---|---|---|
| `Mutex` | `try_lock(task)` / `unlock(task)` / `owner()` | 单核 owner 记录，**不阻塞** |
| `Semaphore` | `new(n)` / `try_wait()` / `post()` / `count()` | 计数、饱和 post |
| `Event` | `set(mask)` / `clear(mask)` / `try_wait(mask)` / `take(mask)` | 等待全部 bit |

**当前限制**：

- 只有 `try_*` 非阻塞语义；
- 没有 waiter 队列、没有优先级继承、没有 ISR-safe 封装；
- 这些要等 QEMU runtime 集成后接入。

## 5. 测试（3 个新增）

```bash
cargo test -p echo-sync
```

| 测试 | 证明 |
|---|---|
| `mutex_is_exclusive` | 互斥、错误 owner 不能解锁 |
| `semaphore_counts_and_saturates` | 计数/post/wait |
| `event_waits_for_all_bits` | 多 bit 等待与 take |

## 6. Q2 还差什么

| ID | 任务 | 状态 |
|---|---|---|
| Q2-01 | idle 任务（QEMU 集成） | 🟡 host 逻辑完成 |
| Q2-02 | SysTick 时间片 | ⬜ |
| Q2-03 | block/sleep/yield（QEMU 集成） | 🟡 host API 完成 |
| Q2-04 | 优先级继承/天花板 | ⬜ |
| Q2-05 | deadline/overrun | ⬜ |
| Q2-06 | host + QEMU 一致性测试 | 🟡 host 部分 |
| Q2-07/08/09 | mutex/semaphore/event 阻塞集成 | 🟡 try-only |
| Q2-10/11 | 静态池 + 栈溢出报告 | ⬜/🟡 |
| Q2-12/13/14 | MPU/NVIC/ISR-first | ⬜ |
| Q2-15 | 时间队列 + timeout 集成 | ⬜ |
| Q2-16/17 | PAL QEMU 后端 + conformance | ⬜ |

## 7. 下一步

1. 在 `echo-mps2-context` 里加入 idle 任务；
2. SysTick 推进虚拟时间，调用 `wake_due`；
3. 任务通过 `sleep_current(deadline)` 睡眠，PendSV 切换；
4. 增加静态池与栈 overflow 报告；
5. 实现 QEMU PAL 后端并跑同一套 conformance；
6. 再更新 Q2 状态。

## 8. 文件

| 文件 | 内容 |
|---|---|
| `crates/kernel/echo-sched/src/preemptive.rs` | Sleeping / sleep_until / wake_due |
| `crates/kernel/echo-sched/tests/preemptive.rs` | 4 个新测试 |
| `crates/kernel/echo-sync/` | Mutex / Semaphore / Event + 3 测试 |
