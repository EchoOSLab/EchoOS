# 10 · 开发计划（执行排期）

> [08-roadmap.md](08-roadmap.md) 定义**能力与验收标准**；本文把它排成**可执行的周计划**。
> 两者冲突时，以 08 + ADR 为准；本文只回答"下周做什么、做到什么算完成"。
>
> 制定日期：2026-11-10　目标：阶段一（实时控制内核）的最小可验证路径。

## 0. 起点与约束

### 已完成（2026-11-10）

| 项 | 状态 |
| --- | --- |
| 工具链 | rustc 1.99.0；4 个 target（thumbv7m / thumbv7em / riscv32imac / x86_64-uefi）已装 |
| QEMU / 调试 | QEMU 6.2（mps2-an385/an500、riscv virt、q35）、OVMF、`gdb-multiarch` 可用 |
| 工程骨架 | 虚拟 workspace；`default-members = ["xtask"]`；`cargo xtask hello` |
| BSP | ARM M3/M7、RISC-V RV32、x86_64 UEFI 四目标输出 `Hello EchoOS` |
| 设计基线 | 9 篇设计文档 + ADR-0020（POSIX 兼容分区，v1/M12 实施） |

### M0 尚未完成

`echo-util`、`echo-vcell`、`echo-arch-api`、`echo-arch-mock`、`tools/svdgen` 骨架、CI、`tools/scripts/setup.sh`。

### 约束

- **单人**，按 **10–12 h/周** 估算；8 h/周则整体拉长到 15 周。
- 只做**阶段一**；POSIX（ADR-0020）、EtherCAT、42 个 crate、文件系统/OTA、SMP、用户态全部不做。
- DoraOS 是上层平台；本仓库只交付 **Micro 底座 + PAL 实现 + 证据**，不实现 DoraOS 运行时。

### 目标（按时间）

| 时间 | 产出 |
| --- | --- |
| **D35** | 一条链路：`dataflow.yaml` → 静态图 → 在 host mock 与 QEMU 上跑出同一份 trace |
| **D60** | H7 真机上的 1 kHz 调度 + 抖动直方图（P50/P99/max） |
| **D90** | 证据包 + PAL 契约冻结 + 第二后端 conformance + Go/No-Go |
| **M12（约 12 个月）** | M0–M7 最小集：单关节 V1 + 抖动报告 |

---

## 1. 路线与门槛

| 阶段 | 周 | 主题 | 出口判据 |
| --- | --- | --- | --- |
| **A · 链路** | W1–W5 | host 链路 + QEMU | D35：同一 YAML 在 mock 与 QEMU 上 trace 一致 |
| **B · 证据** | W6–W9 | H7 真机 + 测量 | D60：真机 P99 抖动数字（不接受纸面数据） |
| **C · 契约** | W10–W12 | PAL 冻结 + 第二后端 | D90：证据包可复现 + Go/No-Go |
| **D · 内核** | 月 4–8 | M3–M6 最小集 | 三任务抢占、时间、IPC、内存池、第二架构 |
| **E · 闭环** | 月 8–12 | M7 单关节 | 定时器→ADC→控制律→PWM + 抖动报告 |

**顺序红线**：A/B/C 不做完，不进入 D；D 不做完，不碰阶段二/三。

---

## 2. 12 周周计划（10–12 h/周）

| 周 | 主题 | 交付物 | 验收 |
| --- | --- | --- | --- |
| **W1** ✅ | 基础设施 | `echo-arch-api` + `echo-arch-mock` + `echo-sched`（`CyclicExecutive`）；`echo-util` 延后到第一个消费者 | 13 个 host 测试全绿；两次 trace 一致 |
| **W2** ✅ | IPC + 3 节点图 | `echo-ipc`（SPSC `Mailbox`）；`echo-conformance` 的 3 节点图 | 10⁶ 帧 0 丢包；24 tests 全绿 |
| **W3** ✅ | PAL 契约 | PAL 5 域（Task/Time/Memory/Channel/Observability）；`echo-conformance` 的 MockPlatform | 7 个 conformance 测试；31 tests 全绿 |
| **W4** ✅ | 代码生成 | `tools/codegen`：YAML 子集 → `static_graph.rs`；生成物 check in + 漂移测试 | 生成图与手写图 trace 一致；40 tests 全绿 |
| **W5** ✅ | QEMU 集成 | QEMU M7 跑 **codegen 生成的** 3 节点图（10k 帧）；`echo-arch-cortex-m` 真机时基移到 W6 | `cargo xtask graph` PASS；D35 功能部分通过 |
| **W6** ⚠️ | H7 bring-up | Cortex-M DWT 时基 + `echo-trace` + H7 骨架；UART 待硬件 | 代码编译通过；**硬件门未过**（无探针/板卡），无真机日志 |
| **W7** ⚠️ | 抢占调度 | `PreemptiveScheduler`（位图 + RR）+ 10 host tests；PendSV/SVC 占位 | 决策逻辑通过；真机上下文切换待硬件门 |
| **W8** ✅ host | 时间与基准 | `echo-time` 延时队列 + `echo-bench` 直方图 + DWT 回绕 | 76 tests；真机时序仍待硬件门 |
| **W9** ⛔ | 真机证据 | H7 上 3 节点图 + 72h 稳定性；`evidence/` v1 | **硬件门未过**：无板卡/探针，无数据；D60 未判 |
| **W10** | PAL 冻结 | PAL v0.1 冻结；`dora-pal-conformance` 用例对接；Linux/mock 第二后端 | 两后端 conformance 100% |
| **W11** | 同一 YAML | 同一份 dataflow 在 EchoOS-H7 与第二后端运行；对比报告 | trace 语义一致；差异有解释 |
| **W12** | 复盘 | 证据包 v1 + 路线决策；更新 08/10 | **D90 Go/No-Go** |

> 若某周的任务超过 **2 个晚上**仍未完成，砍范围而不是延期：先保链路，后保功能。

---

## 3. 工作分解（crate 级）

| crate | 依赖 | 内容 | 验收 |
| --- | --- | --- | --- |
| `echo-util` | — | bitmap、侵入式链表、SPSC ring、固定容量容器 | host 单测 + Miri |
| `echo-config` | — | 编译期常量与 `const_assert!` | 配置错误编译期失败 |
| `echo-arch-api` | — | `trait Arch`（时间、临界区、上下文切换占位） | mock 可测 |
| `echo-arch-mock` | arch-api | 确定性时间、中断注入、调用记录 | 所有内核逻辑 host 可测 |
| `echo-sched` | arch-api, util | 就绪位图、`CyclicExecutive`、抢占、PIP 预留 | ≥ 15 个语义测试 |
| `echo-ipc` | util | `Mailbox`、`Queue`、`try_*` | 满/空/竞争测试 |
| `echo-critical` | arch-api | `BASEPRI` RAII guard | QEMU 上中断延迟测试 |
| `echo-kernel` | 上述 | 内核门面、首次调度 | 单任务→多任务启动 |
| `echo-arch-cortex-m` | arch-api, regs | 向量表、PendSV/SVC/SysTick、MPU 预留 | H7 真机 |
| `echo-arch-riscv` | arch-api, regs | CLINT/PLIC、M 模式陷出 | QEMU virt + conformance |
| `echo-rt` | arch | `#[echo::main]`、`.data/.bss` 初始化 | 冷启动 |
| `echo-trace` | arch | semihosting / UART / COM1 后端 | 统一 `print!` |
| `echo-bench` | arch | DWT 周期计数、P50/P99/max、JSON/CSV | 报告可回归对比 |
| `echo-conformance` | 全部 | 跨架构一致性测试 | CI 中所有目标通过 |
| `echo-vcell` | — | 寄存器内部可变性封装 | 生成代码可用 |
| `tools/svdgen` | — | SVD → 寄存器代码（host 工具） | A7：H743 SVD 生成 GPIO 并通过编译 |
| `tools/codegen` | — | YAML → 静态图（host 工具） | 快照测试稳定 |

---

## 4. 测试与 CI

| 层 | 手段 | 频率 |
| --- | --- | --- |
| L2 · 内核逻辑 | host + `echo-arch-mock`，`cargo test` | 每个提交 |
| QEMU 集成 | `cargo xtask hello`；后续 `test --layer qemu` | 每个 PR |
| HIL 真机 | H7 + 示波器/逻辑分析仪 | 每周 |
| 基准回归 | `echo-bench` JSON 对比，劣化 > 10% 告警 | 每个 PR |
| 静态检查 | `fmt`、`clippy`、`unsafe` 白名单、依赖规则 | 每个 PR |

CI：`ubuntu-22.04`（与本地 QEMU 6.2 对齐），`apt install qemu-system-arm qemu-system-misc ovmf`，跑 host 测试 + QEMU 矩阵。

---

## 5. 里程碑验收（可测）

| 里程碑 | 通过判据 |
| --- | --- |
| **M0** | `cargo test` 绿；`cargo xtask hello` 四目标 PASS；CI 绿 |
| **M1** | QEMU + H7 启动，串口输出 4 行日志；故意 panic 能报 PC/LR |
| **M2** | 3 任务抢占顺序确定；循环执行体 10⁶ 帧无相位漂移；上下文切换有基线 |
| **M3** | 1 kHz 周期误差 < 1%；超限检测触发；时基回绕测试通过 |
| **M4** | Mailbox/Queue 满空边界正确；优先级反转测试通过 |
| **M5** | 栈溢出被 MPU/canary 捕获；内存池耗尽返回 `None` |
| **M6** | 第二架构通过同一套 conformance；换芯片只换 SVD/BSP |
| **M7** | 单关节闭环；1 kHz 抖动 P99 < 5 µs；TCM 对比数据 |

**D90 必须回答的四个数字**：P99 抖动、deadline miss 率、PAL conformance 通过率、codegen 覆盖的能力数。

---

## 6. 与 DoraOS 的接口交付

| 交付 | 归属 | 说明 |
| --- | --- | --- |
| `echoos-micro-api` | EchoOS | 静态任务/mailbox/时间的稳定 API，codegen 的落地目标 |
| PAL 实现 | EchoOS | `dora-pal-echoos` 的后端；硬实时语义 |
| `dora-pal-conformance` | DoraOS | 契约仲裁者；EchoOS 先实现，Zephyr/Linux 做第二实现 |
| `dora-codegen` Micro 后端 | DoraOS | YAML → 静态图；EchoOS 只提供目标 API |
| 证据包 | EchoOS | 构建指纹、调度表、抖动 CSV、复现脚本 |
| POSIX 分区 | EchoOS（M12） | ADR-0020；**90 天内不写一行实现** |

**边界**：DoraOS 不实现内核；EchoOS 不实现数据流运行时。两者只通过 PAL + 生成代码 + 证据格式耦合。

---

## 7. 风险与止损

| 风险 | 触发信号 | 动作 |
| --- | --- | --- |
| **A7 · svdgen 不可行** | W4 结束仍无法从 H743 SVD 生成可编译的 GPIO | 手写最小 UART/TIM 寄存器先过 H7 启动；触发 ADR-0014 复议 |
| **D35 链路不通** | mock/QEMU 无法跑同一份图 | 停内核；Micro 底座改用 Zephyr，EchoOS 降级为契约 + 证据 |
| **D60 无真机数据** | 板子未到或测不出 | 借/租板子或外包测量；**不接受纸面抖动** |
| **单人时间滑移** | 任一阶段超过计划 2 倍 | 砍 M5/M6，保 M7 单关节 V1 |
| **QEMU ≠ 真机** | QEMU 绿、真机挂 | 明确区分逻辑正确性与实时性；HIL 为唯一抖动依据 |
| **DoraOS 依赖** | 第二后端迟迟不到位 | W3 先冻结 PAL；EchoOS 不阻塞，用 mock 做第二实现 |
| **范围蔓延** | 出现"顺便做个 X" | 以本文第 8 节为准；新增走新 ADR |

---

## 8. 12 个月内明确不做

- ❌ POSIX 实现（只保留 ADR-0020 规范与 capability 位）
- ❌ EtherCAT 从站（阶段二再决策；先 CAN-FD/串口）
- ❌ 42 个 crate 全铺开（只做 M0–M7 所需）
- ❌ 文件系统、OTA、Shell 完整版
- ❌ SMP、用户态、系统调用
- ❌ AI 桥接 / 安全包络（阶段三）
- ❌ 功能安全认证（只保留可追溯性与证据格式）

---

## 9. 未来 7 天（可直接执行）

- [x] `echo-arch-api`：`trait Arch { fn now_ns(&self) -> u64; }`
- [x] `echo-arch-mock`：`MockArch`（`advance`/`set`，饱和推进）
- [x] `echo-sched`：`CyclicExecutive` + `FrameStats`，8 个集成测试
- [x] `cargo test` 13 passed；`cargo fmt --check` OK
- [x] `cargo xtask hello` 四目标回归 PASS
- [x] 三个 crate 加入 workspace `members`
- [x] 契约写入 [11-arch-and-sched.md](11-arch-and-sched.md)，记录写入 [12-dev-log.md](12-dev-log.md)
- [ ] `echo-util`：延后到第一个消费者（M2 就绪队列 / W2 IPC）
- [x] W2：`echo-ipc` 的 `Mailbox` + 3 节点图（10⁶ 帧、0 丢包）
- [x] W3：PAL 5 域 trait + mock 后端 + conformance 骨架
- [x] W4：`tools/codegen`（YAML → `static_graph.rs`），生成图与手写图 trace 一致
- [x] W5：QEMU M7 上跑 codegen 生成的 3 节点图 + 1 kHz（D35 功能部分）
- [x] W6 代码路径：`echo-arch-cortex-m`（DWT）+ `echo-trace` + H7 骨架编译通过
- [ ] W6 硬件门：H7 上板启动（缺 ST-Link / 板卡，无真机日志）
- [x] W7 决策逻辑：抢占 + RR 位图调度（host 10 个测试）
- [ ] W7 硬件门：PendSV/PSP 真实上下文切换（缺 H7）
- [x] W8 host：`echo-time` 延时队列 + `echo-bench` 直方图 + DWT 回绕（76 tests）
- [ ] W9：H7 真机证据（⛔ 硬件门：无板卡/探针，无数据）
- [ ] W10：PAL v0.1 冻结 + 第二后端 conformance（host 可完成）
- [ ] 下单 `NUCLEO-H743ZI2` + 8 通道逻辑分析仪（若尚未下单）

---

## 10. 时间预算

| 每周投入 | 12 周计划实际周期 | 说明 |
| --- | --- | --- |
| 8 h | 15 周 | 砍掉 W11 的第二后端对比，延到 D120 |
| **10–12 h** | **12 周** | 本文默认 |
| 14–16 h | 9–10 周 | 仅当能保证连续投入 |

**每日节奏**：1 h 编码 → 1 个 commit → 周五 30 min 写证据笔记（≤ 5 行）→ 周日 30 min 定下周 3 件事。

---

## 11. 执行进度（滚动更新）

| 日期 | 周 | 完成 | 证据 |
| --- | --- | --- | --- |
| 2026-11-10 | W1（部分） | `Arch` / `MockArch` / `CyclicExecutive` + 13 tests | `cargo test`；[12-dev-log](12-dev-log.md) |
| 2026-11-10 | W2 | SPSC `Mailbox` + 3 节点图 | 24 tests；[13-ipc](13-ipc-and-three-node.md) |
| 2026-11-10 | W3 | PAL 5 域 + `MockPlatform` | 31 tests；[14-pal](14-pal-contract.md) |
| 2026-11-10 | W4 | YAML → 静态图 codegen | 40 tests；[15-codegen](15-codegen.md) |
| 2026-11-11 | W5 | QEMU M7 跑生成图（10k 帧 0 丢包） | [16-qemu](16-qemu-integration.md) |
| 2026-11-11 | W6（部分） | DWT 时基 + trace + H7 骨架；硬件门未过 | 48 tests；[17-h7](17-h7-bringup.md) |
| 2026-11-11 | W7（部分） | 抢占决策 + RR；PendSV 占位 | 58 tests；[18-preempt](18-preemptive-sched.md) |
| 2026-11-11 | W8（host） | 时间队列 + 直方图 + DWT 回绕 | 76 tests；[19-time](19-time-and-bench.md) |
| 2026-11-11 | W9 | ⛔ 硬件门：无板卡/探针，无数据 | [20-h7-evidence](20-h7-evidence.md) |
