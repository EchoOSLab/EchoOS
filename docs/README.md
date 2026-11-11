# EchoOS 项目文档与状态

> [English README](../README.md) | 中文文档

EchoOS 是一个**从零用 Rust 实现**、面向**具身智能机器人**的**硬实时操作系统（RTOS，阶段一）**。
它运行在机器人的**小脑层（协调）与脊髓层（关节执行）**，在非确定性的 AI 栈与必须可靠的物理执行器之间建立实时边界。

> **最后更新：2026-11-11** ｜ 阶段一 ｜ **82 tests passed** ｜ QEMU 四目标 PASS ｜ PAL v0.1 冻结 ｜ **D90：条件 Go**
> **真机：WeAct STM32H750VBT6 BSP 编译通过，板卡未到货——没有启动日志、没有抖动数据。**

## 0. 一分钟状态

| 维度 | 状态 | 证据 |
|---|---|---|
| host 内核 / 调度 / IPC / 时间 | ✅ | `cargo test`：82 passed |
| codegen（YAML → 静态调度图） | ✅ | 生成物防漂移测试 |
| PAL v0.1 + 第二后端 | ✅ | 4 个双后端 conformance |
| QEMU 功能验证 | ✅ | `cargo xtask hello` 四目标；`cargo xtask graph` M7 10k 帧 |
| WeAct H750 BSP | ⚠️ 编译通过 | `cargo build ... -p echo-bsp-weact-h750vb` |
| H750 上板首启 | ⛔ 未做 | 无板卡 |
| 真实 PendSV 上下文切换 | ⛔ 占位 | W7 仍是 `bx lr` |
| 480 MHz / UART | ⛔ 未做 | 首启仅 HSI 64 MHz |
| 真机抖动 / deadline（D60） | ⛔ 无数据 | 无 H750 + 逻辑分析仪实测 |
| D90 评审 | ⚠️ 条件 Go | [23-d90-review.md](23-d90-review.md) |

## 1. 先读哪几篇

| 目的 | 文档 |
|---|---|
| 了解为什么做、边界在哪 | [02 需求基线](02-embodied-ai-robot-profile.md) |
| 看完整排期与完成度 | [10 开发计划](10-development-plan.md) |
| 看 D90 结论与证据 | [23 D90 评审](23-d90-review.md) |
| 板子到了怎么上板/调试 | [24 WeAct H750 上板](24-weact-h750-bringup.md) |
| 看每次提交做了什么 | [12 开发日志](12-dev-log.md) |

## 2. 文档索引

| 文档 | 内容 | 读者 |
| --- | --- | --- |
| [01-vision-and-scope.md](01-vision-and-scope.md) | 定位、设计目标、非目标、竞品对标与差异化、依赖边界（L1） | 所有人 |
| **[02-embodied-ai-robot-profile.md](02-embodied-ai-robot-profile.md)** | **★ 需求基线：具身智能机器人场景分析** | **所有人（先读这篇）** |
| [03-architecture.md](03-architecture.md) | 分层架构、内核形态、特权模型、核心机制 | 内核开发者 |
| [04-crate-layout.md](04-crate-layout.md) | crate 清单、依赖图、目录树、依赖规则 | 所有人 |
| [05-design-decisions.md](05-design-decisions.md) | 关键技术决策记录（ADR） | 内核开发者 |
| [06-subsystems.md](06-subsystems.md) | 调度/内存/同步/IPC/时间/驱动 子系统设计 | 子系统负责人 |
| [07-toolchain-and-testing.md](07-toolchain-and-testing.md) | 工具链、构建配置、测试与实时性验证 | 所有人 |
| [08-roadmap.md](08-roadmap.md) | 里程碑 M0–M12、垂直切片、交付物、验收标准 | 所有人 |
| [09-risks-and-open-questions.md](09-risks-and-open-questions.md) | 风险登记、待确认问题、下一步 | 决策者 |
| [10-development-plan.md](10-development-plan.md) | **执行排期**：M0–M7 的 12 周计划、工作分解、门槛与止损 | 所有人 |
| [11-arch-and-sched.md](11-arch-and-sched.md) | `Arch` 抽象与循环执行体 v0：契约、时间模型、测试与限制 | 内核开发者 |
| [12-dev-log.md](12-dev-log.md) | **开发日志**：每次提交的目标、改动、验证、决策与下一步 | 所有人 |
| [13-ipc-and-three-node.md](13-ipc-and-three-node.md) | SPSC `Mailbox` 与 3 节点图：环形语义、内存序、10⁶ 帧验收 | 内核开发者 |
| [14-pal-contract.md](14-pal-contract.md) | PAL 5 域契约：语义抽象、能力矩阵、`MockPlatform`、7 个 conformance | 内核 / DoraOS 开发者 |
| [15-codegen.md](15-codegen.md) | YAML → 静态调度图：输入子集、校验、生成物防漂移、9 个测试 | 内核 / 工具链开发者 |
| [16-qemu-integration.md](16-qemu-integration.md) | QEMU Cortex-M7 集成：生成图、`TickArch` 边界、D35 功能验收 | 内核开发者 |
| [17-h7-bringup.md](17-h7-bringup.md) | H7 bring-up：DWT 时基、trace Sink、硬件门与上板验收清单 | 内核 / 硬件开发者 |
| [18-preemptive-sched.md](18-preemptive-sched.md) | 抢占调度：位图 + RR、10 个测试、PendSV 占位与边界 | 内核开发者 |
| [19-time-and-bench.md](19-time-and-bench.md) | 时间队列、bench 直方图（P50/P99/P99.9）、DWT 回绕 | 内核开发者 |
| [20-h7-evidence.md](20-h7-evidence.md) | H7/H750 抖动实验设计、验收阈值、报告模板（⛔ 硬件门，无数据） | 内核 / 硬件开发者 |
| [21-pal-freeze.md](21-pal-freeze.md) | PAL v0.1 冻结：版本策略、第二后端、双后端 conformance | 内核 / DoraOS 开发者 |
| [22-two-backends.md](22-two-backends.md) | 同一份 YAML：宽/窄通道与 QEMU 摘要对比 | 内核 / DoraOS 开发者 |
| [23-d90-review.md](23-d90-review.md) | **D90 评审**：证据表、D35/D60/D90 判定、条件 Go 与 No-Go | 决策者 |
| [24-weact-h750-bringup.md](24-weact-h750-bringup.md) | WeAct H750VBT6 上板：SWD 接线、OpenOCD/GDB、首启与限制 | 内核 / 硬件开发者 |
| [25-repo-model.md](25-repo-model.md) | 单一开源版本：remote 命名、分支、发布与代码边界 | 维护者 |
| [26-qemu-first-plan.md](26-qemu-first-plan.md) | **QEMU-first 任务计划**：阶段、任务 ID、依赖、验收与硬件门 | 所有人 |
| [27-qemu-runtime.md](27-qemu-runtime.md) | Q0 QEMU MPS2-AN500 runtime：linker、向量、SysTick、semihosting | 内核开发者 |
| [28-qemu-context.md](28-qemu-context.md) | Q1 真实 PendSV/PSP、SVC、3 任务抢占/RR、10k 切换 | 内核开发者 |
| [29-scheduler-sync.md](29-scheduler-sync.md) | Q2 部分：sleep/wake/idle + Mutex/Semaphore/Event（host） | 内核开发者 |
| [30-qemu-scheduler.md](30-qemu-scheduler.md) | Q2 QEMU 运行时会话：idle、SysTick、block/sleep/wake | 内核开发者 |
| [31-memory-pool.md](31-memory-pool.md) | Q2 静态固定块内存池：分配/释放/错误路径 | 内核开发者 |
| [32-qemu-faults-ci.md](32-qemu-faults-ci.md) | Q3 故障注入、`-icount` 确定性、GitHub Actions | 内核 / 维护者 |
| [33-pal-qemu.md](33-pal-qemu.md) | Q2 PAL QEMU 后端 + 共享 conformance 契约 | 内核 / DoraOS 开发者 |
| [34-sync-stack-trace.md](34-sync-stack-trace.md) | Q2 等待队列/栈溢出报告 + Q3 trace ring | 内核开发者 |

## 3. 项目摘要

**要做什么**：一个类 FreeRTOS 的硬实时操作系统——精简宏内核 + 清晰移植层 + 可裁剪组件，但用 Rust 把 FreeRTOS 里靠约定维持的部分变成编译期保证，并加上面向实时控制的增强（零抖动循环执行体、抖动可量化、ISR-first 控制环）。**L1 零外部 Rust crate 依赖**，API 面向 Rust 原生。

**分阶段交付（只做阶段一）**：

| 阶段 | 内容 | 状态 |
| --- | --- | --- |
| **一 · 实时控制内核** | 调度/同步/IPC/内存/时间/驱动 + 循环执行体 | ★ 当前 |
| 二 · 机器人运动控制 | 现场总线 + 多关节协调 | 后续（Q8 未定） |
| 三 · AI 接入 | `echo-bridge` + `echo-safety` | 后续 |

**已经落地的核心取舍**：

- **两种调度模式**：控制域静态循环执行体；服务域静态优先级抢占 + RR。抢占决策逻辑已在 host 验证，真实上下文切换待真机。
- **抖动优先于延迟**：`echo-bench` 报告 P50/P99/P99.9/max 分布，而不是单值。
- **Cortex-M7 + D-Cache/TCM**：硬实时资产放 TCM；H750/H743 都满足，H750 是当前低成本选择。
- **POSIX 是可选的 PSE51 子集分区**：硬实时域不链接 picolibc；当前只做 ADR 与边界，不做实现。
- **内核泛型化 over `Arch`**：同一套内核逻辑可在 host（MockArch）与 Cortex-M（DWT）上跑。
- **零外部 crate**：`cargo test` 与裸机镜像均无第三方 Rust 依赖。

## 4. 当前实现边界

**已实现并验证**：

- `echo-sched`：循环执行体 + 抢占决策（位图/RR）；host 10+8 测试。
- `echo-ipc`：SPSC `Mailbox`，10⁶ 帧 3 节点图 0 丢包。
- `echo-time` / `echo-bench`：延时队列、DWT 回绕、P50/P99/P99.9。
- `echo-pal-api`：PAL v0.1 冻结，两后端 conformance。
- `tools/codegen`：YAML → 静态调度图，生成物防漂移。
- QEMU：Cortex-M3/M7、RISC-V、x86_64 UEFI 四目标 Hello；M7 生成图 10k 帧。
- `echo-bsp-weact-h750vb`：H750 启动汇编、向量表、FPU/VTOR、DWT 64 MHz、生成图 10k 帧、semihosting。

**明确未完成**：

- ❌ H750 上板首启（板卡未到货）；
- ❌ PLL 480 MHz、UART（PA9/PA10）、真实 PendSV/PSP 上下文切换；
- ❌ H750/H7 真机抖动、deadline、72h 稳定性；
- ❌ 现场总线（EtherCAT/CAN-FD）与多关节控制（阶段二）；
- ❌ POSIX 分区实现（阶段三前只保留 ADR）。

## 5. 当前阻塞与下一步

1. **H750 到货后首启**：看到 `EchoOS H750: boot` + `3-node graph OK`，见 [24](24-weact-h750-bringup.md)。
2. **PLL 480 MHz + UART**：改 `CPU_HZ`、加 USART1；在此之前数据只能标 `H750 @64 MHz`。
3. **真实上下文切换**：PendSV/PSP/TCB 栈帧，替换当前 `bx lr` 占位。
4. **W9 真机抖动**：买 8 通道逻辑分析仪，按 [20](20-h7-evidence.md) 做 A/B 实验；D60 未通过前，D90 维持"条件 Go"。
5. **Q8 现场总线**：阶段二的决策，当前不影响内核。

## 6. 验证命令

```bash
cargo test                  # 82 passed
cargo xtask hello           # QEMU 四目标
cargo xtask graph           # QEMU M7：生成图 10k 帧
cargo build --target thumbv7em-none-eabihf -p echo-bsp-weact-h750vb
./tools/scripts/h750-run.sh # 板子到货后：OpenOCD + semihosting
```

环境：Rust 1.99.0、QEMU 6.2.0、OpenOCD、gdb-multiarch、probe-rs 均已安装。

## 7. 状态口径（防止自欺）

- host/QEMU 通过 **不等于** 真机实时性通过；两者只证明功能与语义。
- H750 数据必须标注 **板型 + 时钟 + 构建指纹**；64 MHz 数据不能写成 480 MHz。
- 没有逻辑分析仪/CSV 的"抖动数字"一律不写。
- D90 当前是 **条件 Go**，不是完成；D60 未通过。
