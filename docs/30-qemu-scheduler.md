# 30 · QEMU 调度运行时：idle / tick / block / sleep（Q2 续）

**状态**：🟡 Q2 运行时核心完成；PAL QEMU / 内存池 / Q3 仍未完成
**验收**：`cargo xtask sched` PASS

> Q2 的 host 部分见 [29](29-scheduler-sync.md)；本文是 QEMU 侧的真实切换集成。

## 1. 场景

4 个静态任务：

| 任务 | 优先级 | 行为 |
|---|---|---|
| task2 | 3 | 先跑 100 轮，降到优先级 1，然后 block |
| task0 | 2 | 跑 100 轮后 block；被 idle 唤醒后 sleep 2 ticks |
| task1 | 2 | 跑 100 轮后 block；被 idle 唤醒后继续 |
| idle | 0 | 累加 idle 计数；到 100 次后 unblock task0/1/2；之后继续 yield |

目标：

- 高优先级 task2 先抢占；
- task0/1 同优先级 RR；
- 所有 worker 阻塞后 **idle 运行**；
- idle 唤醒 worker；
- task0 通过 `sleep_until` 睡 2 个 tick，由 SysTick `wake_due` 唤醒；
- 20,000 次切换后退出。

## 2. 调度器接入

### 2.1 sleep

```rust
fn sleep_current(ticks: u32) {
    let now_ns = TICKS * 1_000_000;
    let deadline = now_ns + ticks * 1_000_000;
    SCHED.sleep_until(CURRENT, deadline);
    pend_pendsv();
    SLEEP_WAKE_COUNT += 1;
}
```

- `sleep_until` 清就绪位、状态设为 `Sleeping`、清 `running`；
- 主动触发 PendSV 切走；
- 被 `wake_due` 唤醒后，任务从 `pend_pendsv()` 之后继续执行。

### 2.2 tick

```rust
SysTick_Handler:
    TICKS += 1
    SCHED.wake_due(TICKS * 1_000_000)
    pend_pendsv()
```

- SysTick 同时承担**时间片抢占**：每次 tick 都 pend PendSV；
- PendSV 优先级最低，保证异常返回顺序正确。

### 2.3 PendSV 的状态判断

```rust
if current != 0xFF {
    save_psp(current);
    if state(current) == Running {
        sched.yield_now();   // 只有 Running 才主动让出
    }
}
```

**关键**：如果任务已经被 `sleep_until` 或 `block` 置为 `Sleeping/Blocked`，  
PendSV 不能再调用 `yield_now`，否则会把睡眠/阻塞任务错误地重新置为 Ready。

否则会出现：任务刚睡下又被自己“让出”唤醒，idle 永远不跑。

## 3. 验收

```bash
cargo xtask sched
```

期望：

```text
PASS  qemu-mps2 scheduler (exit=0, 87B)
EchoOS MPS2: sched OK t0=9494 t1=10260 t2=100 idle=100 wake=1 ticks=54 switches=20001
```

字段含义：

| 字段 | 含义 |
|---|---|
| `t0/t1/t2` | 三个 worker 的执行次数 |
| `idle=100` | idle 被调度 100 次并唤醒 worker |
| `wake=1` | task0 从 sleep 中醒来的次数 |
| `ticks=54` | SysTick 次数 |
| `switches=20001` | 上下文切换次数 |

## 4. 覆盖的 Q2 任务

| ID | 状态 |
|---|---|
| Q2-01 idle 任务 | ✅ QEMU 验证 |
| Q2-02 SysTick 时间片 | ✅ tick + PendSV |
| Q2-03 block/sleep/yield | ✅ QEMU 验证 |
| Q2-06 host + QEMU 调度测试 | ✅ 场景验证 |

仍未完成：

| ID | 任务 |
|---|---|
| Q2-04 | 优先级继承/天花板 |
| Q2-05 | deadline/overrun |
| Q2-07/08/09 | 同步原语的阻塞等待队列 |
| Q2-10/11 | 静态内存池 + 栈溢出报告 |
| Q2-12/13/14 | MPU/NVIC/ISR-first |
| Q2-15 | 时间队列 + timeout 集成 |
| Q2-16/17 | PAL QEMU 后端 + conformance |

## 5. 限制

- QEMU TCG 非周期精确：idle/sleep 是功能验证，不是延迟/抖动数据；
- 时间源是 SysTick 虚拟 tick，不是墙钟；
- 未保存 FPU 上下文；
- 同步原语仍是 try-only；
- 无内存池/MPU；
- 未接 PAL QEMU。

## 6. 文件

| 文件 | 内容 |
|---|---|
| `bsp/echo-bsp-qemu-mps2/src/sched_main.rs` | idle/tick/block/sleep 运行时 |
| `tools/scripts/qemu-sched.sh` | Q2 验收 |
| `xtask/src/main.rs` | `sched` 子命令 |

## 7. 下一步

1. 静态内存池 + 栈溢出报告（Q2-10/11）；
2. `echo-sync` 阻塞等待队列（Q2-07/08/09）；
3. PAL QEMU 后端 + conformance（Q2-16/17）；
4. Q3：异常状态、故障注入、CI、`-icount` 回归。
