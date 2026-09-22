# EchoOS 设计文档

EchoOS 是一个**从零用 Rust 实现**、面向**具身智能机器人**的**硬实时操作系统（RTOS）**，采用 Cargo workspace 多 crate 布局。它运行在机器人的**小脑层（协调）与脊髓层（关节执行）**，在非确定性的 AI 栈与必须可靠的物理执行器之间建立一道不可绕过的实时边界。

本目录是项目的设计基线（design baseline）。代码实现应与本文档保持一致；当设计发生变化时，**先改文档再改代码**。

## 文档索引

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

## 一页纸摘要

**要做什么**：一个单地址空间、特权级隔离、编译期静态配置的**组件化宏内核** RTOS。在通用实时内核之上，增加**机器人三层防护**（`bridge → safety → motion`）与**零抖动的循环执行体**。**零外部 crate 依赖**，API 面向 Rust 原生（非 POSIX 兼容）。

**核心取舍**：

- **★ AI 命令是"建议"，不是"命令"** —— 安全层是不可绕过的仲裁者，且永远有一条当前安全轨迹在跑。AI 失联只让它平滑减速，不会让它失控。**这是 EchoOS 最清晰的差异化点**，现有 RTOS 都不提供（`ros2_control` 把该逻辑放在无实时保证、无隔离的 Linux 应用层）。见 [ADR-0017](05-design-decisions.md#adr-0017--ai-命令是建议安全层是不可绕过的仲裁者)。
- **★ 两种调度模式并存** —— 控制域用**静态循环执行体**（零抖动，编译期确定的调度表），服务域用**优先级抢占线程**。纯抢占式的 tick 抖动（1 ms）对多速率控制完全不可接受。见 [ADR-0016](05-design-decisions.md#adr-0016--循环执行体作为一等调度模式)。
- **★ 抖动优先于延迟** —— 稳定地晚 50 µs 无害（控制器已包含该常数延迟），偶尔晚 50 µs 会激起振荡。目标改为 `抖动 P99−P50 < 200 ns`，且 `echo-bench` 必须报告分布而非单值。
- **Cortex-M7 + D-Cache + TCM 是必答题** —— 480 MHz 上一次 D-Cache Miss 的代价可能超过整个上下文切换时间。硬实时资产放 TCM，MPU 显式声明缓存属性，DMA 缓冲区默认非缓存。见 [ADR-0018](05-design-decisions.md#adr-0018--内存分层tcm-放实时代码与数据缓存属性显式声明)。

- **零外部 crate 依赖（L1 完全自研）** —— `[dependencies]` 中外部 crate 恒为 0，所有进入镜像的代码都是自研的。寄存器定义通过**自研 `svdgen` 从 CMSIS-SVD 生成**（不是手写）。代价是工期延长约一个数量级、第三方 `embedded-hal` 驱动不可直接用、首版无 TCP/IP。见 [ADR-0014](05-design-decisions.md#adr-0014--零外部-crate-依赖l1-完全自研) / [ADR-0015](05-design-decisions.md#adr-0015--寄存器定义通过自研-svdgen-代码生成)。
- **宏内核而非微内核** —— MCU 上微内核的 MPU 重编程与 IPC 开销过大。据 Cortex-M4 上的对比研究，Tock 的中断延迟约 15.6 µs，而 RTIC 约 4 µs，差距主要来自 MPU 配置与任务抢占。EchoOS 选择"单地址空间 + MPU 静态分区"，把 MPU 当作纵深防御而非唯一隔离手段。
- **线程模型而非 async 执行器** —— 硬实时要求有界的最坏响应时间，协作式调度无法提供。保留 `echo-async` 作为**应用侧可选适配层**，不作为内核调度基础。
- **内核泛型化 over `Arch` trait** —— 零开销单态化，且让内核逻辑能在 host 上通过 `echo-arch-mock` 做单元测试。这是本方案里**质量收益最大的一个决策**。
- **只依赖 stable Rust** —— `naked_asm!` 自 Rust 1.88 起稳定，上下文切换无需 nightly。当前工具链为 1.98.1 / edition 2024。

**当前环境缺口**（必须优先补齐，见 [06](07-toolchain-and-testing.md)）：

- ❌ 未安装交叉编译 target（`thumbv7m-none-eabi`、`thumbv7em-none-eabihf`、`riscv32imac-unknown-none-elf`）
- ❌ 未安装 QEMU（`qemu-system-arm` / `qemu-system-misc`）
- ❌ 未安装 `probe-rs`（真机烧录调试）
- ✅ 已有 `rust-lld` / `rust-objcopy`（无需额外 binutils）

**已决定的关键问题**：

| 日期 | 问题 | 决定 |
| --- | --- | --- |
| 2026-09-22 | 目标场景 | **具身智能机器人** → [02](02-embodied-ai-robot-profile.md) |
| 2026-09-22 | "从零开发"的边界 | **L1 完全自研**，零外部 crate 依赖 → [ADR-0014](05-design-decisions.md#adr-0014--零外部-crate-依赖l1-完全自研) |
| 2026-09-22 | 寄存器定义方式 | 自研 SVD 代码生成，**不手写** → [ADR-0015](05-design-decisions.md#adr-0015--寄存器定义通过自研-svdgen-代码生成) |

**⚠️ 当前阻塞项（两个最高优先级）**：

1. **[Q8 · 现场总线选型](09-risks-and-open-questions.md#q8--现场总线选型--最高优先级)** —— EtherCAT（< 100 ns 同步，但自研需 2–4 年，**与 L1 直接冲突**）vs CAN-FD（~50 µs，自研 3–6 月，符合 L1）。**这决定项目最难的一块是否在范围内，也决定 L1 的适用范围是否需要调整。**
2. **[Q9 · 硬件平台](09-risks-and-open-questions.md#q9--硬件平台--最高优先级)** —— 决定 M1 的第一块 SVD 用哪个芯片，以及是否立即引入 D-Cache/TCM 复杂度。

**建议的第一步**：见 [09 §9.5](09-risks-and-open-questions.md#95-下一步行动清单)。两个关键动作：

- **在最小 workspace 上先验证 `svdgen` 可行性** —— L1 策略的成败取决于此
- **尽早做 V1 垂直切片**（[08 §8.2](08-roadmap.md#-垂直切片本路线图最重要的方法论调整)）—— 用示波器测出真实的抖动分布，而不是在纸面上讨论预算
