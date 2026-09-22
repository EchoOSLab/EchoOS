# EchoOS 设计文档

EchoOS 是一个**从零用 Rust 实现**、面向嵌入式场景的**硬实时操作系统（RTOS）**，采用 Cargo workspace 多 crate 布局。

本目录是项目的设计基线（design baseline）。代码实现应与本文档保持一致；当设计发生变化时，**先改文档再改代码**。

## 文档索引

| 文档 | 内容 | 读者 |
| --- | --- | --- |
| [01-vision-and-scope.md](01-vision-and-scope.md) | 定位、设计目标、非目标、竞品对标与差异化、**依赖边界（L1）** | 所有人 |
| [02-architecture.md](02-architecture.md) | 分层架构、内核形态、特权模型、核心机制 | 内核开发者 |
| [03-crate-layout.md](03-crate-layout.md) | crate 清单、依赖图、目录树、依赖规则 | 所有人 |
| [04-design-decisions.md](04-design-decisions.md) | 关键技术决策记录（ADR） | 内核开发者 |
| [05-subsystems.md](05-subsystems.md) | 调度/内存/同步/IPC/时间/驱动 子系统设计 | 子系统负责人 |
| [06-toolchain-and-testing.md](06-toolchain-and-testing.md) | 工具链、构建配置、测试与实时性验证 | 所有人 |
| [07-roadmap.md](07-roadmap.md) | 里程碑 M0–M9、交付物、验收标准 | 所有人 |
| [08-risks-and-open-questions.md](08-risks-and-open-questions.md) | 风险登记、待确认问题、下一步 | 决策者 |

## 一页纸摘要

**要做什么**：一个单地址空间、特权级隔离、编译期静态配置的**组件化宏内核** RTOS。提供优先级抢占调度、确定性内存、同步原语、IPC、驱动框架，以及全部自研的中间件层。**零外部 crate 依赖**，API 面向 Rust 原生（非 POSIX 兼容）。

**核心取舍**：

- **零外部 crate 依赖（L1 完全自研）** —— `[dependencies]` 中外部 crate 恒为 0，所有进入镜像的代码都是自研的。寄存器定义通过**自研 `svdgen` 从 CMSIS-SVD 生成**（不是手写）。代价是工期延长约一个数量级、第三方 `embedded-hal` 驱动不可直接用、首版无 TCP/IP。见 [ADR-0014](04-design-decisions.md#adr-0014--零外部-crate-依赖l1-完全自研) / [ADR-0015](04-design-decisions.md#adr-0015--寄存器定义通过自研-svdgen-代码生成)。
- **宏内核而非微内核** —— MCU 上微内核的 MPU 重编程与 IPC 开销过大。据 Cortex-M4 上的对比研究，Tock 的中断延迟约 15.6 µs，而 RTIC 约 4 µs，差距主要来自 MPU 配置与任务抢占。EchoOS 选择"单地址空间 + MPU 静态分区"，把 MPU 当作纵深防御而非唯一隔离手段。
- **线程模型而非 async 执行器** —— 硬实时要求有界的最坏响应时间，协作式调度无法提供。保留 `echo-async` 作为**应用侧可选适配层**，不作为内核调度基础。
- **内核泛型化 over `Arch` trait** —— 零开销单态化，且让内核逻辑能在 host 上通过 `echo-arch-mock` 做单元测试。这是本方案里**质量收益最大的一个决策**。
- **只依赖 stable Rust** —— `naked_asm!` 自 Rust 1.88 起稳定，上下文切换无需 nightly。当前工具链为 1.98.1 / edition 2024。

**当前环境缺口**（必须优先补齐，见 [06](06-toolchain-and-testing.md)）：

- ❌ 未安装交叉编译 target（`thumbv7m-none-eabi`、`thumbv7em-none-eabihf`、`riscv32imac-unknown-none-elf`）
- ❌ 未安装 QEMU（`qemu-system-arm` / `qemu-system-misc`）
- ❌ 未安装 `probe-rs`（真机烧录调试）
- ✅ 已有 `rust-lld` / `rust-objcopy`（无需额外 binutils）

**已决定的关键问题**（2026-09-22）：

- ✅ **"从零开发"的边界 = L1 完全自研**，零外部 crate 依赖 → [ADR-0014](04-design-decisions.md#adr-0014--零外部-crate-依赖l1-完全自研)
- ✅ 寄存器定义通过自研 SVD 代码生成，不手写 → [ADR-0015](04-design-decisions.md#adr-0015--寄存器定义通过自研-svdgen-代码生成)

**当前阻塞项**：**首选目标平台尚未确定**（[08 Q1](08-risks-and-open-questions.md#q1--首选目标平台是哪个-高优先级)）——它决定 M1 的第一块 SVD 用哪个芯片。

**建议的第一步**：见 [08 §8.5](08-risks-and-open-questions.md#85-下一步行动清单)。最关键的是**在最小 workspace 上先验证 `svdgen` 可行性**——L1 策略的成败取决于此，越早暴露风险越好。
