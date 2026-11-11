# 28 · QEMU 真实上下文切换（Q1）

**状态**：✅ Q1 完成（2026-11-11）｜ **验收**：`cargo xtask context` PASS
**目标板**：QEMU MPS2-AN500（Cortex-M7），与 H750 共用 Cortex-M 异常模型。

## 1. 范围

Q1 完成：

- ✅ TCB / 每任务栈 / 栈 canary；
- ✅ MSP（内核）与 PSP（任务）双栈；
- ✅ 首次任务伪造异常帧；
- ✅ PendSV 保存/恢复 R4–R11；
- ✅ SVC 主动 yield；
- ✅ SysTick 强制切换；
- ✅ 3 任务：高优先级抢占 → 降优先级 → 同优先级 RR；
- ✅ 10,000 次切换 + canary 检查；
- ✅ GDB 在 PendSV 断点停下（Q1-07）。

Q1 不包含：

- ❌ FPU 上下文（S0–S31 / EXC_RETURN 0xFFFFFFED）；
- ❌ 阻塞/睡眠/优先级继承（Q2）；
- ❌ MPU/栈溢出硬件保护（Q3）；
- ❌ 任何真机时序结论。

## 2. 栈帧与 PSP

首次任务的栈布局（从高地址向低地址）：

```text
高地址
  ┌──────────────┐
  │ xPSR         │ 0x0100_0000（T bit）
  │ PC           │ task entry
  │ LR           │ task_exit
  │ R12          │ 0
  │ R3           │ 0
  │ R2           │ 0
  │ R1           │ 0
  │ R0           │ 0
  ├──────────────┤ ← frame（硬件异常帧）
  │ R11 .. R4    │ 首次为 0；PendSV 保存区
  ├──────────────┤ ← 初始 PSP = frame - 32
  │ ...          │
  │ canary       │ 栈底
  └──────────────┘
低地址
```

**关键点**：`TCB.psp` 指向 **R4–R11 保存区**，不是硬件异常帧。  
PendSV 恢复时：

```text
ldmia r0!, {r4-r11}   → r0 上移到异常帧基址
msr psp, r0           → 异常返回从异常帧弹 R0–R3/R12/LR/PC/xPSR
```

## 3. PendSV / SVC

```asm
PendSV_Handler:
    mrs r0, psp
    stmdb r0!, {r4-r11}     @ 保存任务 callee-saved
    push {lr}
    bl pendsv_switch        @ 选下一个任务，返回新 PSP
    pop {lr}
    ldmia r0!, {r4-r11}
    msr psp, r0
    bx lr
```

```asm
SVC_Handler:
    push {lr}
    bl svc_handler          @ 置 PendSV pending
    pop {lr}
    bx lr
```

- PendSV 优先级设为最低（0xFF），SysTick 0x80；
- SVC 默认优先级 0；
- 首次启动：PSP 指向 dummy 栈，`CONTROL=2`，触发 PendSV，切换到 task2。

## 4. 3 任务调度场景

| 任务 | 优先级 | 行为 |
|---|---|---|
| task2 | 3 | 先运行 100 轮，然后降到优先级 1 |
| task0 | 2 | 与 task1 同优先级 RR |
| task1 | 2 | 与 task0 同优先级 RR |

结果（一次运行示例）：

```text
EchoOS MPS2: context OK t0=4937 t1=4945 t2=101 switches=10000
```

- `t2≈100`：高优先级先抢占；
- `t0≈t1≈4950`：降级后同优先级 RR；
- `switches=10000`：切换次数；
- canary 全部通过。

## 5. 调试中踩到的关键坑

**错误做法**：

```rust
msr psp, dummy_top
msr control, #2      // 切到 PSP
let shpr3 = read(...); // 之后的 C 局部变量使用 PSP！
```

切到 PSP 后，编译器仍然用 `sp` 保存局部变量，`sp` 已经是 dummy 栈；局部变量直接写进 `DUMMY_STACK` 和相邻 `.bss`，把 `TASK1_COUNT/TASK2_COUNT` 覆盖成 `0xE000ED20` 之类的地址值。

**正确做法**：

- SHPR3 等 C 代码在 **MSP** 上完成；
- 切 PSP、设 CONTROL、触发 PendSV 必须在**同一个汇编块**里完成，之后不再执行任何使用 C 栈的代码。

这条经验对 H750 移植同样成立。

## 6. 验收

```bash
cargo xtask context
```

期望：

```text
PASS  qemu-mps2 context switch (exit=0, 63B)
EchoOS MPS2: context OK t0=... t1=... t2=... switches=10000
```

GDB 断点验证：

```bash
./tools/scripts/qemu-context-gdb.sh
```

期望：

```text
PASS  qemu-mps2 context GDB breakpoint
```

## 7. 限制

- QEMU TCG 非周期精确：只证明**功能**，不证明切换延迟/抖动；
- 未保存 FPU 上下文；任务不得使用浮点；
- 未实现 MSP 独立内核栈大小检查；
- 未实现阻塞/睡眠/优先级继承；
- canary 只检查栈底，不是 MPU 保护。

## 8. 文件

| 文件 | 内容 |
|---|---|
| `bsp/echo-bsp-qemu-mps2/src/context_main.rs` | TCB、PSP/MSP、PendSV/SVC、3 任务 |
| `tools/scripts/qemu-context.sh` | Q1 验收 |
| `tools/scripts/qemu-context-gdb.sh` | Q1-07 GDB 断点 |
| `xtask/src/main.rs` | `context` 子命令 |

## 9. 下一步（Q2）

1. idle 任务；
2. SysTick 时间片；
3. block/unblock/sleep/yield；
4. mutex/semaphore/event；
5. 静态内存池 + 栈 canary；
6. PAL QEMU 后端 + conformance。

Q2 才把“能切换”变成“能作为 RTOS 跑任务”。
