# 26 · QEMU-first 开发任务计划

**状态**：Q0–Q3 核心 ✅；Q2 可选 backlog 待清 ｜ **生效日期**：2026-11-11
**目标**：在 H750 板卡到货前，用 QEMU Cortex-M7 把 Phase 1 的**运行时功能**做真；板卡到货后只做板级替换与时序验收。

> 本文是当前**唯一执行计划**。`docs/10-development-plan.md` 的 W1–W12 保留为历史基线。
> 原则：**QEMU 只证明功能，不证明实时性；没有真机数据，D60 不通过。**

## 1. 目标与非目标

### 1.1 目标

1. 在 QEMU 上完成**真实上下文切换**与抢占调度运行时；
2. 补齐同步、内存/MPU、中断/NVIC、时间、PAL、trace、故障处理；
3. 建立 QEMU 自动化回归与 CI；
4. 把 H750 的板级差异压缩到 linker/时钟/UART/内存布局；
5. 板卡到货后，按同一套验收用例完成 H750 bring-up 与 D60 证据。

### 1.2 非目标（本计划不做）

- ❌ 阶段二现场总线（EtherCAT/CAN-FD 选型与实现）；
- ❌ 阶段三 AI bridge / safety envelope；
- ❌ POSIX 分区实现（仅保留 ADR-0020）；
- ❌ SWO/ETM 深度跟踪、多核、TrustZone；
- ❌ 用 QEMU 数据代替真机抖动/WCET。

## 2. 当前基线（2026-11-11）

| 能力 | 状态 | 位置 |
|---|---|---|
| host 测试 | ✅ 82 passed | `cargo test` |
| CyclicExecutive + 3 节点图 | ✅ | `echo-sched`、`echo-ipc` |
| 抢占**决策逻辑** | ✅ | `echo-sched::preemptive` |
| 真实 PendSV/PSP 上下文切换 | ✅ Q1 完成 | `bsp/echo-bsp-qemu-mps2/src/context_main.rs` |
| QEMU runtime（linker/向量/SysTick） | ✅ Q0 完成 | `bsp/echo-bsp-qemu-mps2` |
| QEMU PAL 后端 | ⬜ 仅有 Mock | `echo-conformance` |
| 同步原语 | ⬜ | 无 |
| MPU/栈保护 | ⬜ | 无 |
| NVIC/ISR 注册 | ⬜ | 无 |
| 时间队列与调度集成 | 🟡 独立 | `echo-time` |
| trace | 🟡 semihosting | `echo-trace` |
| 故障处理/故障注入 | 🟡 部分 | H750 有 HardFault 打印 |
| H750 上板 | ⛔ 板卡未到 | `docs/24` |
| H750 PLL 480/Cache/TCM/UART | ⛔ | 无 |
| D60 真机抖动 | ⛔ 无数据 | `docs/20` |

## 3. 阶段与 Gate

```text
Q0 QEMU runtime 骨架
   ↓
Q1 真实上下文切换 + 3 任务
   ↓
Q2 调度运行时 + 同步 + 时间 + PAL QEMU
   ↓
Q3 故障/MPU/栈/CI 回归
   ↓
H1 H750 首启 + 480 MHz + UART
   ↓
H2 H750 真实调度 + PAL
   ↓
H3 HIL + 逻辑分析仪 + D60 证据
```

| Gate | 判据 | 不通过怎么办 |
|---|---|---|
| **GQ0** ✅ | QEMU 启动 + 完整向量 + SysTick + 干净退出 | 已通过（2026-11-11） |
| **GQ1** ✅ | 3 任务抢占/RR 顺序确定；10k 次切换无栈损坏 | 已通过（2026-11-11） |
| **GQ2** | 调度服务 + 同步 + PAL QEMU conformance 全绿 | 缩减范围，先保 P0 |
| **GQ3** | 故障注入可复现；CI 全绿；`-icount` 回归稳定 | 修稳定性，不加功能 |
| **GH1** | H750 首启日志 + 480 MHz + UART | 查 PLL/Flash latency/引脚 |
| **GH2** | H750 3 任务 + PAL conformance | 查板级/中断/栈 |
| **GH3 / D60** | 逻辑分析仪 A/B、P99、72h | 没有数据就不通过 |

## 4. 完整任务列表

### 4.1 Q0 · QEMU runtime 骨架（P0）

| ID | 任务 | 产出/验收 | 依赖 | 状态 | 估算 |
|---|---|---|---|---|---|
| Q0-01 | MPS2-AN500 linker script | 代码/向量/栈/数据分区；`readelf` 可验证 | — | ✅ | 3h |
| Q0-02 | 完整 16 向量 + VTOR | 故障/中断能进正确 handler | Q0-01 | ✅ | 3h |
| Q0-03 | 启动代码：SP、`.data` 拷贝、`.bss` 清零、FPU | 静态变量初值正确；FPU 可用 | Q0-01 | ✅ | 4h |
| Q0-04 | SysTick 时基 + tick 计数 | 计数增长；tick 日志稳定 | Q0-03 | ✅ | 4h |
| Q0-05 | semihosting trace + `SYS_EXIT` | 启动/失败可区分退出码 | Q0-03 | ✅ | 2h |
| Q0-06 | `cargo xtask qemu-runtime` + 脚本 | 一条命令构建/运行/断言 | Q0-01..05 | ✅ | 3h |

### 4.2 Q1 · 真实上下文切换（P0）

| ID | 任务 | 产出/验收 | 依赖 | 状态 | 估算 |
|---|---|---|---|---|---|
| Q1-01 | TCB/栈帧布局 | `TaskContext` 结构 + 栈顶对齐规则 | Q0 | ✅ | 4h |
| Q1-02 | MSP/PSP + 首次任务伪造异常帧 | 第一个任务能从 PSP 启动 | Q1-01 | ✅ | 6h |
| Q1-03 | PendSV 保存/恢复 | 切换后寄存器/栈一致 | Q1-02 | ✅ | 8h |
| Q1-04 | SVC 系统调用入口 | 任务能主动 yield/进入内核 | Q1-03 | ✅ | 4h |
| Q1-05 | 3 任务抢占 + RR 集成 | 顺序与 host 决策测试一致 | Q1-03/04 | ✅ | 8h |
| Q1-06 | 10k 次切换压力 + 栈 canary | 无 HardFault、canary 完好 | Q1-05 | ✅ | 4h |
| Q1-07 | QEMU GDB 调试脚本 | 能在 PendSV/任务断点停下 | Q0 | ✅ | 2h |

### 4.3 Q2 · 调度运行时与内核服务（P0/P1）

| ID | 任务 | 产出/验收 | 依赖 | 状态 | 估算 |
|---|---|---|---|---|---|
| Q2-01 | idle 任务 | 无就绪任务时进 idle | Q1 | ✅ | 2h |
| Q2-02 | SysTick 时间片抢占 | 同优先级按 tick 轮转 | Q2-01 | ✅ | 4h |
| Q2-03 | block/unblock/sleep/yield | 阻塞/唤醒/睡眠顺序正确 | Q2-01 | ✅ | 6h |
| Q2-04 | 优先级继承/天花板 | 无界优先级反转可检测 | Q2-03 | ⬜ | 8h |
| Q2-05 | deadline/overrun 检测 | 超限计数 + trace | Q2-02 | ⬜ | 4h |
| Q2-06 | 调度器 host + QEMU 测试 | 同一场景两处结果一致 | Q2-01..05 | ✅ | 6h |
| Q2-07 | mutex | 互斥 + 优先级继承 | Q2-03 | 🟡 runtime handoff | 6h |
| Q2-08 | semaphore / event flags | 计数/事件等待与唤醒 | Q2-03 | 🟡 sem queue | 6h |
| Q2-09 | 临界区 + ISR-safe API | 关中断/恢复正确 | Q2-03 | 🟡 BSP critical | 4h |
| Q2-10 | 静态内存池（实机语义） | 分配/释放/耗尽路径 | Q2-03 | ✅ host | 5h |
| Q2-11 | 栈 canary/溢出检测 | 溢出能报告任务名 | Q2-10 | ✅ 任务索引 | 4h |
| Q2-12 | MPU 区域配置 | 越界访问触发 MemManage（QEMU 支持范围内） | Q2-10 | ⬜ | 8h |
| Q2-13 | NVIC/IRQ 注册 | 软件触发 IRQ 可进 handler | Q0-02 | ✅ IRQ0 | 5h |
| Q2-14 | ISR-first / 延迟处理 | ISR 只做最小工作，后半段可测 | Q2-13 | 🟡 IRQ 入口 | 6h |
| Q2-15 | 时间队列 + 超时集成 | `sleep_until`/timeout 可唤醒 | Q2-03 | ✅ host | 6h |
| Q2-16 | PAL QEMU `Platform` | 五域实现，任务/时间/池/通道/trace | Q2-01..15 | ✅ | 8h |
| Q2-17 | PAL 双后端 conformance | Mock + QEMU 同一套用例全绿 | Q2-16 | ✅ | 4h |

### 4.4 Q3 · 故障、回归与工程化（P1）

| ID | 任务 | 产出/验收 | 依赖 | 状态 | 估算 |
|---|---|---|---|---|---|
| Q3-01 | trace ring buffer + 事件 ID | 固定容量、无丢事件/可计数 | Q2-16 | ✅ | 6h |
| Q3-02 | HardFault/MemManage/BusFault/UsageFault | 打印 CFSR/HFSR/MMFAR/BFAR | Q0-02 | ✅ UsageFault | 6h |
| Q3-03 | 故障注入测试 | 空指针/非法指令/越界可复现 | Q3-02 | ✅ udf | 4h |
| Q3-04 | `-icount` 确定性回归 | 同一输入指令数稳定；非时序证据 | Q1-06 | ✅ | 5h |
| Q3-05 | GitHub Actions | fmt/clippy/test/QEMU/docs/commit lint | Q0-06 | ✅ run 7 | 6h |
| Q3-06 | 文档/链接/commit lint | 文档链接 0 broken；英文 commit | Q3-05 | ✅ docs+commit | 3h |
| Q3-07 | 第一个公共 alpha tag | `v0.1.0-alpha.1` + release notes | Q3-05 | ✅ | 2h |

### 4.5 H1 · H750 首启与时钟（P0，硬件门）

| ID | 任务 | 产出/验收 | 依赖 | 状态 | 估算 |
|---|---|---|---|---|---|
| H1-01 | 板卡接线 + 首启 | `EchoOS H750: boot` + `graph OK` | 板卡到货 | ⛔ | 4h |
| H1-02 | PLL 480 MHz | DWT/SysTick 时钟一致；`CPU_HZ=480M` | H1-01 | ⛔ | 8h |
| H1-03 | Cache/TCM/Flash latency | 硬实时段放 TCM；Cache 使能后可启动 | H1-02 | ⛔ | 8h |
| H1-04 | UART USART1 + trace 后端 | `/dev/ttyACM` 或 USB-TTL 日志 | H1-01 | ⛔ | 6h |
| H1-05 | H750 linker/向量对齐 | 与 QEMU BSP 共享 context 代码 | H1-02 | ⛔ | 4h |

### 4.6 H2/H3 · 真机运行时与证据（P0，硬件门）

| ID | 任务 | 产出/验收 | 依赖 | 状态 | 估算 |
|---|---|---|---|---|---|
| H2-01 | H750 真实上下文切换 | 3 任务顺序与 QEMU 一致 | H1-05 | ⛔ | 8h |
| H2-02 | H750 SysTick/时间片 | 1 kHz tick 稳定 | H2-01 | ⛔ | 4h |
| H2-03 | H750 PAL 后端 | 同一套 conformance 通过 | H2-01 | ⛔ | 8h |
| H2-04 | H750 trace UART | 事件流可解析 | H1-04 | ⛔ | 4h |
| H3-01 | HIL 测试脚本 | 自动烧录/运行/断言 | H2 | ⛔ | 8h |
| H3-02 | 逻辑分析仪 A/B 实验 | 基线 vs EchoOS 原始 CSV | 逻辑分析仪 | ⛔ | 8h |
| H3-03 | 抖动/72h 报告 | P50/P99/P99.9/max + 构建指纹 | H3-02 | ⛔ | 12h |
| H3-04 | D60 复盘 | 通过/不通过 + 证据包 | H3-03 | ⛔ | 4h |

### 4.7 持续任务（贯穿）

| ID | 任务 | 规则 | 状态 |
|---|---|---|---|
| D-01 | 每个功能写契约文档 | `docs/` + 索引 | 🟡 持续 |
| D-02 | 每个提交写开发日志 | `docs/12-dev-log.md` | 🟡 持续 |
| D-03 | 更新本计划状态 | 每周一次 | 🟡 持续 |
| D-04 | BSP 入门练习 | boot 路径/`readelf`/最小 BSP/故障注入 | ⬜ |
| D-05 | 英文文档/API 文档 | 公共仓库英文优先 | 🟡 持续 |

## 5. 依赖关系

```text
Q0-01 ─┬─> Q0-02 ─> Q0-03 ─> Q0-04 ─> Q0-05 ─> Q0-06
       │
Q1-01 ─┴─> Q1-02 ─> Q1-03 ─> Q1-04 ─> Q1-05 ─> Q1-06
                                      │
Q2-01 ─> Q2-02 ─> Q2-03 ─┬─> Q2-04
                         ├─> Q2-05
                         ├─> Q2-07/08/09
                         ├─> Q2-10 ─> Q2-11/12
                         └─> Q2-15
Q2-13 ─> Q2-14
Q2-16 ─> Q2-17
Q0-02 ─> Q3-02 ─> Q3-03
Q1-06 ─> Q3-04
Q0-06 ─> Q3-05 ─> Q3-06/07

H1-01 ─> H1-02 ─> H1-03
H1-01 ─> H1-04
H1-02 ─> H1-05 ─> H2-01 ─> H2-02/03
H2-01 ─> H3-01
H3-02 ─> H3-03 ─> H3-04
```

## 6. 时间估算（按 10–12 h/周）

| 阶段 | 估算 | 说明 |
|---|---|---|
| Q0 | 1 周 | 16–19 h |
| Q1 | 2 周 | 32–36 h |
| Q2 | 4–5 周 | 64–80 h |
| Q3 | 2 周 | 32–34 h |
| **Q0–Q3 合计** | **9–10 周** | 不含硬件等待 |
| H1–H3 | 6–8 周 | 取决于板卡/逻辑分析仪到货 |

**建议顺序**：Q0 → Q1 → Q2（先 PAL/同步）→ Q3；硬件到货后并行 H1，不阻塞 Q2/Q3。

## 7. 每个任务的“完成定义”

一个任务只有同时满足以下 4 条才算完成：

1. 代码/文档已提交；
2. 有**可执行验收命令**与结果（host/QEMU/HIL）；
3. 已写契约文档 + `docs/12-dev-log.md`；
4. 已更新本计划的 `状态` 与相关索引。

**禁止**：

- 只写代码不写文档；
- 用 host/QEMU 结果冒充真机时序；
- 没有验收命令就标 ✅。

## 8. 调整说明（相对旧计划）

| 调整 | 原因 |
|---|---|
| **QEMU-first** 取代“等 H750 再开发” | H750 未到货；QEMU 可完成 60–70% 功能 |
| 删除私有下游，只保留单一开源版本 | 已决定；降低维护成本 |
| 抢占调度从“决策逻辑”升级为“真实运行时” | W7 只完成了决策，没有 TCB/PendSV |
| PAL 增加 QEMU 后端 | 让 conformance 在无硬件时也能跑 |
| H750 首启锁定 HSI 64 MHz | PLL 480 未验证，先保证能启动 |
| 抖动/WCET 全部后置到 H3 | QEMU 不能提供时序证据 |
| 增加 GitHub CI/文档 lint | 国际化开源的最低质量门 |

## 9. 暂不做的 Backlog

以下功能明确**不进本计划**，避免范围膨胀：

- EtherCAT / CAN-FD 现场总线（阶段二，Q8 未定）；
- 多关节运动控制、安全包络；
- AI bridge / safety；
- POSIX PSE51 分区；
- H743 2 MB 版本 / 多板卡；
- SWO/ETM、多核、TrustZone；
- 动态内存、动态任务创建。

## 10. 下一步

**Q0 ✅（2026-11-11）**：linker、16 向量 + VTOR、启动、SysTick、semihosting、`cargo xtask qemu-runtime` 全部通过。

**Q1 ✅（2026-11-11）**：真实 PendSV/PSP、SVC、3 任务抢占/RR、10k 切换 + canary、GDB 断点全部通过。

**Q2 运行时核心 ✅（2026-11-11）**：

- 调度器：`Sleeping`、`sleep_until`、`wake_due`、idle 约定；4 个 host 测试；
- QEMU：idle、SysTick 时间片、block/sleep/wake；`cargo xtask sched` PASS；
- `echo-sync`：Mutex / Semaphore / Event 的 try-only 版本 + 3 个测试。

**Q2 剩余（可选 backlog）**：

1. 优先级继承/deadline（Q2-04/05）；
2. 信号量等待队列 QEMU 集成（Q2-08）；
3. MPU 区域配置（Q2-12）；
4. ISR 后半段/deferred work（Q2-14）。

**Q2 已完成**：运行时会话、内存池、PAL QEMU + conformance、栈溢出报告。

**Q3 核心完成**：

- ✅ 故障注入（`udf` → UsageFault + CFSR）；
- ✅ `-icount` 双跑确定性；
- ✅ trace ring（host + QEMU PAL）；
- ✅ 文档 link/table lint + commit lint；
- ✅ alpha tag `v0.1.0-alpha.1`；
- ✅ GitHub Actions 已绿（run 8，runner 固定 ubuntu-24.04）。
