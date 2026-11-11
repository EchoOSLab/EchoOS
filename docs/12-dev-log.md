# 12 · 开发日志（滚动更新）

> **规则**：每个改变代码或设计的提交，必须在本文件追加一条记录。
> 每条记录至少包含：目标、改动、验证、决策/偏差、下一步。
> 功能契约写在对应的专题文档（如 [11](11-arch-and-sched.md)），本文件只记录"发生了什么"。

## 索引

| 日期 | ID | 类型 | 摘要 | commit |
| --- | --- | --- | --- | --- |
| 2026-11-10 | DOC-001 | 设计 | ADR-0020 POSIX 兼容分区 + 交叉引用同步 | `a7d4a5c` |
| 2026-11-10 | M0-001 | 工程 | workspace + 四目标 QEMU `Hello EchoOS` | `58082b2`…`8e6c7b9` |
| 2026-11-10 | W1-001 | 功能 | `trait Arch` + `MockArch` + `CyclicExecutive` | `301caa2` |
| 2026-11-10 | W1-002 | 文档 | W1 日志与计划进度跟踪 | 本条目 |
| 2026-11-10 | W2-001 | 功能 | SPSC Mailbox + 3 节点图（10⁶ 帧 0 丢包） | `c2846fd` |
| 2026-11-10 | W3-001 | 功能 | PAL 5 域契约 + MockPlatform + 7 个 conformance | `348796a` |
| 2026-11-10 | W4-001 | 功能 | YAML → 静态图 codegen（生成物 check in + 漂移测试） | `dd0aba2` |
| 2026-11-11 | W5-001 | 功能 | QEMU M7 跑生成图（10k 帧 0 丢包） | `e63afcb` |
| 2026-11-11 | W6-001 | 功能/阻塞 | DWT 时基 + trace + H7 骨架（硬件门未过） | `94d9602` |
| 2026-11-11 | W7-001 | 功能/部分 | 抢占决策 + RR；PendSV/SVC 占位 | `fd90d87` |
| 2026-11-11 | W8-001 | 功能 | 时间队列 + bench 直方图 + DWT 回绕 | `42751a0` |
| 2026-11-11 | W9-001 | 文档/阻塞 | H7 证据实验设计与报告模板（无数据） | 本条目（docs） |
| 2026-11-11 | W10-001 | 功能 | PAL v0.1 冻结 + 第二后端（80 tests） | `f137233` |
| 2026-11-11 | W11-001 | 功能 | 同一生成图宽/窄通道对比（82 tests） | `83685a4` |
| 2026-11-11 | W12-001 | 评审 | D90：条件 Go；D60 未通过 | 本条目（docs） |
| 2026-11-11 | H750-001 | 功能/待上板 | WeAct H750 BSP + OpenOCD 脚本（编译通过） | `0465e1b` |
| 2026-11-11 | PLAN-001 | 规划 | QEMU-first 开发任务计划（26） | 本条目（docs） |
| 2026-11-11 | Q0-001 | 功能 | QEMU MPS2 runtime：linker/向量/SysTick/semihosting | `f321b74` |
| 2026-11-11 | Q1-001 | 功能 | QEMU 真实 PendSV/PSP + 3 任务 10k 切换 | `65a6630` |
| 2026-11-11 | Q2-001 | 功能/部分 | sleep/wake/idle + Mutex/Semaphore/Event（host） | `54aaec5` |
| 2026-11-11 | Q2-002 | 功能 | QEMU idle/tick/block/sleep 运行时 | `8246590` |

---

## 2026-11-10 · DOC-001 · ADR-0020

**目标**：给 POSIX 兼容性一个明确边界，避免"支持 POSIX"变成范围爆炸。

**改动**：
- `docs/05-design-decisions.md`：新增 ADR-0020（PSE51 子集、时间分区、picolibc 受控例外、16 条验收用例）。
- `docs/01`、`docs/04`、`docs/08`、`docs/09`、`docs/README.md`：同步非目标、crate 树、R8 规则、M12、风险 T25–T28。

**决策**：
- POSIX 只作为 M12 的可选服务域；硬实时域保持无 POSIX、无堆。
- 90 天内不写一行 POSIX 实现。

**下一步**：回到 EchoOS 主线，先打通 host 链路。

---

## 2026-11-10 · M0-001 · QEMU 四目标启动

**目标**：在没有任何内核的情况下，先证明"工具链 + QEMU + 工程结构"可用。

**改动**：
- 虚拟 workspace；`default-members` 只含 host 可编译 crate。
- `bsp/echo-bsp-qemu-cortex-m`（M3/M7，semihosting）
- `bsp/echo-bsp-qemu-riscv-virt`（RV32，UART0）
- `bsp/echo-bsp-qemu-x86`（x86_64 UEFI + OVMF，ConOut/COM1）
- `xtask` + `tools/scripts/qemu-hello.sh`（矩阵冒烟）

**验证**：
```text
cargo xtask hello
PASS arm-cortex-m3 (exit=0)   PASS arm-cortex-m7 (exit=0)
PASS riscv32 (exit=0)        PASS x86_64 (exit=33, 正常)
```
四目标总耗时 ~3 s（改为 guest 自行退出后，从 70 s 降下来）。

**决策/偏差**：
- 串口输出改为**文件后端**，绕开终端 stdio 捕获问题（`8e6c7b9`）。
- 三个 guest 自行退出：ARM semihosting `SYS_EXIT`、RISC-V `sifive_test`、x86 `isa-debug-exit`。
- x86 图形窗口额外用 UEFI `ConOut` 输出，避免只写 COM1 时窗口里看不到。

**下一步**：W1 的 host 垂直切片。

---

## 2026-11-10 · W1-001 · Arch + Mock + 循环执行体

**目标**：建立"能在 host 上确定性测试"的最小内核切片，为 W2 的 3 节点图打地基。

**改动**：
- `crates/arch/echo-arch-api`：`trait Arch { fn now_ns(&self) -> u64; }`
- `crates/arch/echo-arch-mock`：`MockArch`（`new/with_time/advance/set`，饱和推进）
- `crates/kernel/echo-sched`：
  - `TaskDef` / `SlotStat` / `FrameStats` / `SchedError`
  - `CyclicExecutive::new` 校验 `frame_ns > 0`、`Σ slot_ns ≤ frame_ns`
  - `run_frame` 每帧读一次时钟，槽位偏移由预算推导
- `docs/11-arch-and-sched.md`：实现契约与限制

**验证**：
```text
cargo test
echo-arch-mock: 5 passed
echo-sched (tests/cyclic.rs): 8 passed
合计 13 passed, 0 failed
cargo fmt --check: OK
cargo xtask hello: 四目标仍 PASS（回归）
```

**决策/偏差**：
- **D-001**：`echo-util` 延后——bitmap 等就绪队列（M2），ring 等 IPC/trace；先写工具库没有消费者。
- **D-002**：`run_frame` 只读一次时钟；时钟推进由调用者负责（真机硬件 / host mock），保证两端语义一致。
- **D-003**：`slot_ns` 是时间预算，不是实测执行时间；WCET 与超限检测放 W8。
- **D-004**：测试放 `tests/` 集成测试，只测公开 API；`no_std` 库本身零测试 harness 依赖。

**偏差**：W1 原计划先写 `echo-util`；实际按"消费者驱动"调整为先做 Arch/Mock/Sched。计划文档已同步。

**下一步（W2）**：
1. `echo-ipc`：`Mailbox<T, const N: usize>`（`try_send`/`try_recv`，满/空语义）。
2. 手写 3 节点图：sensor → control → actuator。
3. 10⁶ 帧、0 丢包、trace 可复现。
4. 把结果写回本日志与 `docs/11`。

---

## 2026-11-10 · W1-002 · 文档跟踪机制

**目标**：让"每个功能都有对应文档、每次提交都有记录"成为可执行的规则，而不是口号。

**改动**：
- 新增本文件 `docs/12-dev-log.md`（索引 + 条目模板 + 滚动记录）。
- `docs/README.md`：索引加入 10/11/12。
- `docs/10-development-plan.md`：W1 实际执行顺序、进度表、7 天清单勾选状态。

**验证**：文档间相对链接与锚点检查通过（0 broken）。

**下一步**：W2 的 `Mailbox` 与 3 节点图；完成后追加 W2-001。

---

## 2026-11-10 · W2-001 · SPSC Mailbox + 3 节点图

**目标**：把 sensor → control → actuator 串成静态图，证明 10⁶ 帧、0 丢包、顺序正确。

**改动**：
- `crates/kernel/echo-ipc/src/lib.rs`：`Mailbox<const N>`（u32 载荷、`try_send`/`try_recv`、`AtomicU32` 环形、零 `unsafe`）+ 9 个单测。
- `crates/kernel/echo-ipc/tests/spsc.rs`：双线程 SPSC 100k 条 FIFO 测试。
- `crates/test/echo-conformance/tests/three_node.rs`：3 节点图 10⁶ 帧。
- `docs/13-ipc-and-three-node.md`：契约、内存序、测试与限制。

**验证**：
```text
cargo test
echo-ipc 单测:            9 passed
echo-ipc spsc:            1 passed（双线程 100k，FIFO）
echo-conformance:         1 passed（10^6 帧，0 丢包，sum 正确）
累计 24 passed, 0 failed
cargo fmt --check:        OK
cargo clippy:             无警告
cargo xtask hello:        四目标回归 PASS
```

**决策/偏差**：
- **D-005**：v0 载荷固定 `u32`。`AtomicU64` 在 thumbv7m/riscv32 上没有 64 位原子操作，泛型载荷留到 M4 的池化消息。
- **D-006**：保留一个空槽区分满/空，容量 = `N-1`；不引入额外 count 原子。
- **D-007**：全原子实现 + `forbid(unsafe_code)`，守住 R3 的 `echo-ipc` unsafe = 0，可直接放 `static`。
- **D-008**：3 节点图放 `crates/test/echo-conformance`，跨 crate 验收集中管理。
- **偏差**：W2 原计划还包含 `CyclicExecutive`，它已在 W1 完成；W2 实际只做 IPC + 图。

**下一步（W3）**：
1. PAL 5 域（Task/Time/Memory/Channel/Observability）trait 与 mock 后端；
2. `echo-conformance` 增加 PAL 用例骨架；
3. 与 DoraOS 侧对齐 `dora-pal-conformance` 的用例命名。

---

## 2026-11-10 · W3-001 · PAL 5 域契约 + MockPlatform

**目标**：把 Micro 底座能力收敛成稳定契约，并让 conformance 只依赖契约、不依赖具体后端。

**改动**：
- `crates/api/echo-pal-api/src/lib.rs`：`Platform` + Task/Time/Memory/Channel/Observability + `Capabilities` + `StaticTask`。
- `crates/test/echo-conformance/src/mock.rs`：`MockPlatform`（静态任务表 8、trace 64、池 16B×16、通道 8 槽）。
- `crates/test/echo-conformance/tests/pal.rs`：7 个 conformance 测试。
- `docs/14-pal-contract.md`：契约、能力矩阵、Mock 常量、测试与限制。

**验证**：
```text
cargo test
echo-pal-api:     编译通过（0 测试）
echo-conformance: 7 passed（PAL conformance）
累计 31 passed, 0 failed
cargo fmt --check: OK
cargo clippy:      无警告
cargo xtask hello: 四目标回归 PASS
```

**决策/偏差**：
- **D-009**：`declare(StaticTask)` 取代 `spawn(closure)`。Micro 任务编译期确定；运行期 `spawn` 会破坏静态保证。
- **D-010**：v0 每个 `Platform` 只有一个 channel；多通道/端点留到 M4。
- **D-011**：内存域用 16 B × 16 固定块池；typed pool 留到 M4。
- **D-012**：契约以测试为准；DoraOS 的 `dora-pal-conformance` 必须镜像同一组用例。
- **偏差**：`Platform` 不是 `dyn` 安全的（关联类型 + 静态分发），这是刻意的零成本取舍。
- **结构变化**：`echo-conformance` 从纯测试宿主变成带 mock 的 host 库，新增正常依赖（arch-api/arch-mock/ipc/pal-api）。

**下一步（W4）**：
1. `tools/codegen`：YAML 子集 → 静态图代码；
2. 任务声明走 `TaskApi::declare`，生成图与手写 3 节点图 trace 一致；
3. 新增 `docs/15-codegen.md`，追加 `W4-001`。

---

## 2026-11-10 · W4-001 · YAML → 静态调度图

**目标**：把 `dataflows/*.yaml` 编译成静态调度表，并证明生成图与手写图语义一致。

**改动**：
- `tools/codegen`：受限 YAML 解析、校验、代码生成 + CLI（`echo-codegen <in.yaml> <out.rs>`）。
- `dataflows/three_node.yaml`：3 节点输入（frame 1000，槽位 200/100/50）。
- `crates/test/echo-conformance/tests/data/generated_three_node.rs`：生成物，**check in**。
- `crates/test/echo-conformance/tests/generated_graph.rs`：生成图 vs 手写图 1000 帧 trace 对比。
- `docs/15-codegen.md`：YAML schema、校验规则、再生成命令、决策与限制。

**验证**：
```text
cargo test
echo-codegen:        7 passed
generated_graph:     2 passed
累计 40 passed, 0 failed
cargo fmt --check:   OK
cargo clippy:        无警告
cargo xtask hello:   四目标回归 PASS
```

**决策/偏差**：
- **D-013**：YAML 顺序 = 调度顺序，不做拓扑重排；静态图的可分析性优先。
- **D-014**：生成物 check in + `checked_in_conformance_file_is_current` 漂移测试（CI 可守）。
- **D-015**：只生成声明（调度表/常量/边元数据），任务体由应用注入。
- **D-016**：自写 YAML 子集解析（不引第三方 crate），与 L1 一致。
- **偏差**：`EDGES` 目前只是元数据，尚未自动接 Mailbox/PAL Channel；多帧速率、DAG 重排均未做。

**下一步（W5）**：
1. `echo-arch-cortex-m`：向量表、link.x、`Arch` 实现、semihosting/UART trace；
2. QEMU `mps2-an500` 上跑 3 节点图 + 1 kHz 槽位；
3. 新增 `docs/16-qemu-integration.md`，追加 `W5-001`。

---

## 2026-11-11 · W5-001 · QEMU M7 跑生成图

**目标**：把 codegen 生成的静态调度表放到 Cortex-M7 裸机上跑，验证调度 + IPC + 生成物集成。

**改动**：
- `bsp/echo-bsp-qemu-cortex-m/src/graph.rs`：独立 `echo-arm-graph` bin，TickArch + 生成图 + semihosting。
- `dataflows/generated/three_node.rs`：生成物移到共享目录，host conformance 与 BSP 共用同一份。
- `tools/scripts/qemu-graph.sh` + `cargo xtask graph`。
- `docs/16-qemu-integration.md`：结构、边界、决策。

**验证**：
```text
cargo xtask graph
PASS  qemu-m7 3-node graph (exit=0, 25B)

cargo xtask hello
四目标 PASS

cargo test:        40 passed, 0 failed
cargo fmt --check: OK
cargo clippy:      无警告
```

**决策/偏差**：
- **D-017**：QEMU 用 `TickArch`（每帧显式推进），不读硬件时基；QEMU 不产出抖动数字。
- **D-018**：图 demo 用独立 bin `echo-arm-graph`，`echo-arm` 的 Hello 矩阵保留为回归基线。
- **D-019**：QEMU 用 semihosting，H7 真机换 UART。
- **D-020**：生成物放 `dataflows/generated/`，host conformance 与 QEMU BSP 共用，避免两份漂移。
- **D-021（偏差）**：W5 未引入 `echo-arch-cortex-m`；真机时基（SysTick/DWT）与 H7 bring-up 一起做，避免为 QEMU 造一个假 Arch。

**下一步（W6）**：
1. 确认 `NUCLEO-H743ZI2` + 逻辑分析仪到货；没有硬件只做设计与编译验证，**不造假数据**；
2. `echo-arch-cortex-m`：SysTick/DWT 时基 + `Arch` 实现；
3. `echo-trace`：UART 后端；
4. H7 上复跑同一张生成图；新增 `docs/17-h7-bringup.md`，追加 `W6-001`。

---

## 2026-11-11 · W6-001 · Cortex-M DWT + trace + H7 骨架（硬件门未过）

**目标**：为 H7 bring-up 准备时基与 trace 代码路径，同时明确硬件边界。

**改动**：
- `crates/arch/echo-arch-cortex-m`：`DwtClock<CPU_HZ>`、`cycles_to_ns`、DWT 使能。
- `crates/core/echo-trace`：`Sink` trait、`write_str`/`write_bytes`、semihosting 后端。
- `bsp/echo-bsp-nucleo-h743zi`：H7 启动骨架、link.x、DWT 初始化、semihosting 启动输出。
- `docs/17-h7-bringup.md`：已实现/未实现/上板验收清单。

**验证**：
```text
cargo test                                         48 passed, 0 failed
cargo build --target thumbv7em-none-eabihf -p echo-bsp-nucleo-h743zi   OK
```

**硬件门（未过）**：
- `lsusb` 无 ST-Link；`/dev/ttyACM*` 不存在；
- 因此**没有** H7 启动日志、没有 UART 输出、没有抖动数据；
- 明确记录：**编译通过 ≠ 已在 H7 上跑通**。

**决策/偏差**：
- **D-022**：真机时基用 DWT `CYCCNT`（单次 volatile 读，无中断开销）。
- **D-023**：首次上板用 semihosting；UART 需要 RCC/引脚配置，后置。
- **D-024**：无硬件只做编译与设计验证，不产出任何"实测"数字。
- **D-025**：H7 骨架先最小化（只有 Reset 向量），完整向量表随 UART 一起补。

**下一步（W7）**：
1. `echo-sched` 抢占调度决策逻辑（静态优先级 + 同优先级 RR）；
2. `echo-arch-mock` 中断注入与切换序列记录；
3. Cortex-M PendSV/SVC 汇编先编译通过，真机验证等硬件门；
4. 新增 `docs/18-preemptive-sched.md`，追加 `W7-001`。

---

## 2026-11-11 · W7-001 · 抢占调度决策 + RR（PendSV 占位）

**目标**：实现静态优先级抢占与同优先级 RR 的决策逻辑，host 完全可验证。

**改动**：
- `crates/kernel/echo-sched/src/preemptive.rs`：`PreemptiveScheduler`（`priority_bitmap` + per-priority `task_mask` + RR cursor）。
- `crates/kernel/echo-sched/tests/preemptive.rs`：10 个行为测试。
- `crates/arch/echo-arch-cortex-m/src/context.rs`：PendSV/SVC **占位**（`bx lr`）。
- `docs/18-preemptive-sched.md`：语义、复杂度、限制、决策。

**验证**：
```text
cargo test -p echo-sched --test preemptive   10 passed
cargo test                                  58 passed, 0 failed
cargo clippy                                无警告
cargo build --target thumbv7em ...          通过（含占位汇编）
cargo xtask graph / hello                   仍 PASS
```

**决策/偏差**：
- **D-026**：`priority_bitmap + task_mask`，最高优先级与 RR 都是 O(1)。
- **D-027**：调度决策与 `Arch` 解耦，纯 host 可测。
- **D-028**：PendSV/SVC 只占位，**不假装上下文切换已完成**；真实 TCB/PSP/栈帧需硬件门。
- **D-029**：RR 由 `yield_now` 驱动，时间片 tick 留到 W8。

**下一步（W8）**：
1. `echo-time`：单调钟、延时链表、`sleep_until`；
2. `echo-bench`：P50/P99/P99.9/max 直方图 + JSON/CSV；
3. DWT/SysTick 回绕处理（真机门）；
4. 新增 `docs/19-time-and-bench.md`，追加 `W8-001`。

---

## 2026-11-11 · W8-001 · 时间队列 + bench 直方图 + DWT 回绕

**目标**：实现 host 可测的时间/测量基础设施，并补上 DWT 32 位回绕。

**改动**：
- `crates/kernel/echo-time`：`DelayQueue`、`deadline_after`/`is_due` + 9 个测试。
- `crates/test/echo-bench`：`Histogram`、`Summary`、JSON/CSV + 6 个测试。
- `crates/arch/echo-arch-cortex-m`：`wrap_period_ns`、`extend_monotonic`、`epoch_ns` + 3 个回绕测试。
- `docs/19-time-and-bench.md`：契约、百分位算法、回绕语义、限制。

**验证**：
```text
cargo test                         76 passed, 0 failed
cargo clippy                       无警告
cargo xtask graph / hello          仍 PASS
cargo build --target thumbv7em ... OK
```

**决策/偏差**：
- **D-030**：百分位用整数 `div_ceil`，不引浮点。
- **D-031**：`Histogram<N>` 固定容量，满时拒绝而非覆盖。
- **D-032**：DWT 回绕用 `epoch_ns` 累加；480 MHz 周期 8,947,848,533 ns 有断言。
- **D-033**：时间队列与 `Arch` 解耦，host 可测。
- **边界**：`echo-bench` 目前只有 host 数学测试；**没有**真机抖动数据。

**下一步（W9）**：
1. 新增 `docs/20-h7-evidence.md`：实验设计、测量步骤、报告模板、验收阈值；
2. 硬件门未过则明确写"无数据"，不写通过；
3. 硬件到位后：DWT 直方图 + 逻辑分析仪 GPIO 翻转 + 72h 稳定性；
4. W10 先做 host 可完成的 PAL 冻结 + 第二后端 conformance。

---

## 2026-11-11 · W9-001 · H7 证据实验设计（⛔ 硬件门，无数据）

**目标**：在无硬件条件下，把 H7 抖动实验设计、验收阈值与报告模板固定下来。

**改动**：
- `docs/20-h7-evidence.md`：硬件清单、A/B 对照组、指标、步骤、阈值、报告模板。

**验证**：
- ❌ 无硬件：`lsusb` 无探针、`/dev/ttyACM*` 不存在；
- ❌ 无 H7 启动日志、无抖动数据、无 deadline 统计；
- ✅ 文档明确记录"W9 不能标通过，D60 不能判过"。

**决策/偏差**：
- **D-034**：必须同时测基线 A（裸机 ISR）与 EchoOS B；否则抖动无法归因。
- **D-035**：原始 CSV + 构建指纹 + 调度表一起入库，保证可复现。
- **D-036**：硬件缺失时 W9 标 ⛔，绝不用 host/QEMU 数字冒充真机数据。

**下一步（W10）**：
1. 冻结 PAL v0.1（trait + 能力矩阵 + conformance 用例集）；
2. 实现第二个 PAL 后端（不同容量/行为），跑同一套 conformance；
3. 新增 `docs/21-pal-freeze.md`，追加 `W10-001`。

---

## 2026-11-11 · W10-001 · PAL v0.1 冻结 + 第二后端

**目标**：冻结 PAL 契约，并用第二个后端证明 conformance 只依赖契约、不依赖具体容量。

**改动**：
- `crates/api/echo-pal-api`：新增 `PAL_VERSION = (0,1,0)`。
- `crates/test/echo-conformance/src/mock_small.rs`：`SmallMockPlatform`（2 任务 / 8 trace / 4×8B 池 / 2 槽通道）。
- `crates/test/echo-conformance/tests/pal_two_backends.rs`：4 个双后端 conformance 测试。
- `docs/21-pal-freeze.md`：冻结面、变更策略、两后端容量表。

**验证**：
```text
cargo test                                         80 passed, 0 failed
cargo clippy                                       无警告
cargo xtask graph                                  PASS
```

**决策/偏差**：
- **D-037**：`PAL_VERSION` 显式常量，DoraOS 侧可做编译期/运行期协商。
- **D-038**：第二后端只改容量、不改语义，避免"复制粘贴的假验证"。
- **D-039**：冻结以 conformance 测试为准，不以文档为准。

**下一步（W11）**：
1. 同一份生成图在 host 两个后端/通道容量上对比 trace；
2. 对比 QEMU M7 的摘要结果（只有 OK/帧数/丢包，没有逐帧 trace）；
3. 新增 `docs/22-two-backends.md`，追加 `W11-001`。

---

## 2026-11-11 · W11-001 · 同一 YAML，两个后端

**目标**：证明同一份 codegen 产物在不同通道容量/运行环境下语义一致。

**改动**：
- `crates/test/echo-conformance/tests/two_backends_graph.rs`：`Mailbox<8>` vs `Mailbox<2>` 跑同一生成图 10k 帧，对比 count/sum/drops/trace digest。
- `docs/22-two-backends.md`：host 双后端 + QEMU 摘要对比矩阵。

**验证**：
```text
cargo test -p echo-conformance --test two_backends_graph   2 passed
cargo test                                                 82 passed, 0 failed
cargo xtask graph                                          PASS（QEMU M7 摘要）
cargo clippy                                               无警告
```

**决策/偏差**：
- **D-040**：trace 用 FNV digest 对比，不打印全量。
- **D-041**：窄通道选容量 1（`Mailbox<2>`），最苛刻但仍能承载每帧 1 条消息。
- **D-042**：QEMU 只报摘要，不参与逐帧对比；不把"摘要一致"夸大为"trace 一致"。
- **边界**：H7 真机没有参与（硬件门）；QEMU 无周期精度。

**下一步（W12）**：
1. 汇总 D35/D60/D90 证据；
2. 输出 Go / 条件 Go / No-Go；
3. 新增 `docs/23-d90-review.md`，追加 `W12-001`，然后结束本轮目标。

---

## 2026-11-11 · W12-001 · D90 评审：条件 Go

**目标**：汇总 W6–W12 的代码、测试、文档与硬件门，给出 D90 结论。

**改动**：
- `docs/23-d90-review.md`：证据表、D35/D60/D90 判定、Go/条件 Go/No-Go、D120 动作。

**验证**：
```text
cargo test                          82 passed, 0 failed
cargo xtask graph                   PASS（QEMU M7 摘要）
cargo xtask hello                   四目标 PASS
H7 真机启动/抖动                    ⛔ 无硬件，无数据
```

**结论**：
- **条件 Go**：host/契约/codegen/PAL 线可继续；
- **No-Go**：任何真机实时性声明、认证、扩范围（POSIX/EtherCAT/42 crate）；
- **D35 功能通过；D60 未通过；D90 部分通过**。

**决策**：
- **D-043**：D90 判条件 Go 而非 Go。
- **D-044**：D60 保留未通过，认证/客户材料不得引用。
- **D-045**：下一阶段先解决硬件，再谈扩展。

**目标收口**：W6–W12 已逐项执行并记录；硬件相关里程碑只做代码/编译/设计，未编造数据。

---

## 2026-11-11 · H750-001 · WeAct H750 BSP（编译通过，待上板）

**目标**：按用户选型 **WeAct STM32H750VBT6 + ST-Link**，提供可烧录/调试的板级支持。

**改动**：
- `bsp/echo-bsp-weact-h750vb`：启动汇编（栈/向量/`.data`/`.bss`）、FPU/VTOR、DWT 64 MHz 时基、生成图 3 节点 10k 帧、semihosting。
- `bsp/echo-bsp-weact-h750vb/link.x`：128 KB Flash / 128 KB DTCM / 512 KB AXI。
- `tools/scripts/h750-run.sh`：OpenOCD 烧录 + `arm semihosting enable`。
- `docs/24-weact-h750-bringup.md`：SWD 接线、OpenOCD/GDB、常见问题、限制。

**验证**：
```text
cargo build --target thumbv7em-none-eabihf -p echo-bsp-weact-h750vb   OK
ELF: .text 0x30b8 (~12.2 KB), .bss 0x60 (96 B)
vector: SP=0x20020000, Reset=0x08000041, HardFault 已挂
cargo test                                                          82 passed
cargo xtask graph / hello                                           PASS
```
**硬件门**：没有 H750 板子，**未上板**；没有启动日志、没有抖动数据。本文不把编译通过写成跑通。

**决策/偏差**：
- **D-046**：首启 HSI 64 MHz，不先配 PLL 480 MHz（先验证最小启动链路）。
- **D-047**：首启用 semihosting，不先做 UART（板子无板载 USB-UART）。
- **D-048**：链接脚本按 128 KB Flash 写死，误用 H743 2 MB 脚本会在链接期暴露。
- **D-049**：H750 数据必须标注板型/时钟，不能冒充 H743 @480 MHz。

**下一步**：
1. 上板首启，看到 `EchoOS H750: boot` + `3-node graph OK`；
2. 配 PLL 480 MHz，改 `CPU_HZ` 并复测 DWT；
3. UART（PA9/PA10 + USB-TTL）；
4. W7 真实 PendSV/PSP；W9 逻辑分析仪抖动实验。

---

## 2026-11-11 · PLAN-001 · QEMU-first 开发任务计划

**目标**：把“板子未到货期间如何开发”固化成可执行、可验收、可跟踪的任务列表。

**改动**：
- `docs/26-qemu-first-plan.md`：Q0–Q3 + H1–H3 任务、依赖、验收、时间估算、Gate。
- `docs/README.md`：索引加入 26。
- `README.md` / `README.zh-CN.md`：文档入口加入 26。
- `docs/10-development-plan.md`：标注后续以 26 为准，保留历史基线。

**验证**：
```text
文档链接检查：0 broken
表格列数检查：一致
```

**决策/偏差**：
- **QEMU-first**：H750 未到货，先用 QEMU M7 做真实上下文切换/调度/PAL/CI。
- **QEMU 只证功能**：不做抖动/WCET 数字；D60 仍待真机。
- **单一开源版本**：不再有私有下游；所有任务进入同一个公开仓库。
- **首启 HSI 64 MHz**：PLL 480 等 H1 再验证，不提前写“480 MHz 通过”。

**下一步（Q0）**：
1. Q0-01 linker script；
2. Q0-02 完整向量表 + VTOR；
3. Q0-03 启动代码；
4. Q0-04 SysTick；
5. Q0-05 semihosting + exit；
6. Q0-06 `cargo xtask qemu-runtime`。

---

## 2026-11-11 · Q0-001 · QEMU MPS2-AN500 runtime

**目标**：完成 Q0——QEMU Cortex-M7 的最小 runtime 骨架，为 Q1 真实上下文切换打地基。

**改动**：
- `bsp/echo-bsp-qemu-mps2`：linker script、16 个系统向量 + VTOR、`.data/.bss`、FPU、SysTick、semihosting、`SYS_EXIT`。
- `tools/scripts/qemu-runtime.sh` + `cargo xtask qemu-runtime`。
- `docs/27-qemu-runtime.md`：Q0 契约与验收。
- `docs/26-qemu-first-plan.md`：Q0-01..06 标为完成，GQ0 通过。

**验证**：
```text
cargo xtask qemu-runtime
PASS  qemu-mps2 runtime (exit=0, 44B)
日志：EchoOS MPS2: boot / systick OK

cargo test                 82 passed
cargo xtask graph          PASS
cargo xtask hello          四目标 PASS
```

**决策/偏差**：
- MPS2-AN500 代码区 `0x0000_0000`，RAM `0x2000_0000` 256 KB，栈顶 `0x2004_0000`；
- SysTick 用 25 MHz 标称分频，只验证 tick 功能，**不是实时精度证据**；
- Q0 的 PendSV/SVC 仍指向 `Default_Handler`，Q1 替换；
- QEMU 继续只证明功能，不产出抖动/WCET。

**下一步（Q1）**：
1. TCB/栈帧布局；
2. MSP/PSP + 首次任务异常帧；
3. PendSV 保存/恢复；
4. SVC 入口；
5. 3 任务抢占 + RR；
6. 10k 切换压力 + 栈 canary。

---

## 2026-11-11 · Q1-001 · QEMU 真实上下文切换

**目标**：完成 Q1——真实 PendSV/PSP 上下文切换、SVC yield、3 任务抢占/RR、10k 切换 canary、GDB 断点。

**改动**：
- `bsp/echo-bsp-qemu-mps2/src/context_main.rs`：TCB、MSP/PSP、首次任务异常帧、PendSV、SVC、SysTick、3 任务、canary。
- `tools/scripts/qemu-context.sh` + `cargo xtask context`。
- `tools/scripts/qemu-context-gdb.sh`：Q1-07 GDB 断点演练。
- `docs/28-qemu-context.md`：Q1 契约、栈帧、踩坑与限制。
- `docs/26-qemu-first-plan.md`：Q1-01..07 标为完成，GQ1 通过。

**验证**：
```text
cargo xtask context
PASS  qemu-mps2 context switch (exit=0, 63B)
EchoOS MPS2: context OK t0=4937 t1=4945 t2=101 switches=10000

./tools/scripts/qemu-context-gdb.sh
PASS  qemu-mps2 context GDB breakpoint

cargo test                 82 passed
cargo xtask graph          PASS
cargo xtask hello          四目标 PASS
```

**关键坑**：
- 首次任务 `TCB.psp` 必须指向 **R4–R11 保存区**，不是硬件异常帧；否则 `ldmia` 会把异常帧当 R4–R11。
- 切到 PSP 后不能继续执行使用 C 栈的代码；`CONTROL=2`、`msr psp`、触发 PendSV 必须在同一汇编块完成，否则局部变量会覆盖 `DUMMY_STACK`/相邻 `.bss`。

**决策/偏差**：
- task2 先以优先级 3 抢占 100 轮，再降到 1；task0/1 同优先级 RR；
- canary 每 1 次切换检查，10k 次后退出；
- QEMU 仍只证明功能，不产出切换延迟/抖动。

**下一步（Q2）**：
1. idle 任务；
2. SysTick 时间片；
3. block/unblock/sleep/yield；
4. mutex/semaphore/event；
5. 静态内存池 + 栈 canary；
6. PAL QEMU 后端 + conformance。

---

## 2026-11-11 · Q2-001 · sleep/wake/idle + 同步原语（host）

**目标**：先完成 Q2 的可 host 测试部分：调度器睡眠/唤醒、idle 约定、同步原语。

**改动**：
- `echo-sched::preemptive`：`TaskState::Sleeping`、`sleep_until`、`wake_due`、`is_sleeping`；block/unblock/set_priority 处理 Sleeping。
- `crates/kernel/echo-sync`：Mutex / Semaphore / Event（try-only，单核）。
- `docs/29-scheduler-sync.md`：Q2 host 契约与剩余项。
- `docs/26-qemu-first-plan.md`：Q2-01/03/06/07/08/09/11 标为 🟡。

**验证**：
```text
cargo test -p echo-sched --test preemptive   14 passed
cargo test -p echo-sync                      3 passed
cargo test                                   89 passed
cargo clippy                                 无警告
```

**决策/偏差**：
- idle = 优先级 0 的普通静态任务，不引入特殊分支；
- sleep 用绝对 deadline + `wake_due(now)`，与时间源解耦；
- sync 当前只有 `try_*`，阻塞等待队列在 QEMU runtime 集成时接；
- QEMU idle/sleep/PAL/内存池仍未完成，Q2 保持 🟡。

**下一步（Q2 继续）**：
1. QEMU runtime 加 idle 任务；
2. SysTick 推进虚拟时间 + `wake_due`；
3. 任务 sleep/block 的真实切换；
4. 静态池 + 栈溢出报告；
5. PAL QEMU 后端 + conformance。

---

## 2026-11-11 · Q2-002 · QEMU 调度运行时

**目标**：把 Q2 host 调度逻辑接进真实 QEMU 上下文切换，验证 idle、SysTick、block、sleep/wake。

**改动**：
- `bsp/echo-bsp-qemu-mps2/src/sched_main.rs`：4 任务（task0/1/2 + idle）、SysTick `wake_due`、sleep/block、20k 切换。
- `tools/scripts/qemu-sched.sh` + `cargo xtask sched`。
- `docs/30-qemu-scheduler.md`：运行时契约与限制。
- `docs/26-qemu-first-plan.md`：Q2-01/02/03/06 标为完成。

**验证**：
```text
cargo xtask sched
PASS  qemu-mps2 scheduler (exit=0, 87B)
EchoOS MPS2: sched OK t0=9494 t1=10260 t2=100 idle=100 wake=1 ticks=54 switches=20001

cargo xtask qemu-runtime   PASS
cargo xtask context        PASS
cargo test                 89 passed
cargo xtask graph          PASS
cargo xtask hello          四目标 PASS
```

**关键点**：
- `pendsv_switch` 只在当前状态为 `Running` 时调用 `yield_now()`；Sleeping/Blocked 不能被重新置 Ready。
- SysTick 每次 tick 都 `wake_due` 并 pend PendSV，兼作时间片。
- idle 是优先级 0 的普通任务；worker 全阻塞后自然被选中。

**边界**：
- QEMU 只证明功能，不产出抖动/延迟；
- 同步原语仍是 try-only；
- 无 PAL QEMU、无内存池、无故障回归。

**下一步（Q2 收尾/Q3）**：
1. 静态内存池 + 栈溢出报告；
2. sync 阻塞等待队列；
3. PAL QEMU 后端 + conformance；
4. Q3 故障处理 / `-icount` / GitHub Actions。

---

## 条目模板（复制用）

```markdown
## YYYY-MM-DD · ID · 一句话标题

**目标**：

**改动**：
- 

**验证**：
```text
命令与结果
```

**决策/偏差**：
- 

**下一步**：
1. 
```
