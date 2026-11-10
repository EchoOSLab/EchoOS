# EchoOS

面向具身智能机器人的 **Rust `no_std` 硬实时操作系统（阶段一）**：运行在机器人的**小脑层（协调）与脊髓层（关节执行）**，在非确定性的 AI 栈与必须可靠的执行器之间建立实时边界。

> **当前状态（2026-11-11）**：阶段一 · **82 tests passed** · QEMU 四目标 PASS · PAL v0.1 已冻结 · **D90 条件 Go**
> **真机状态**：WeAct STM32H750VBT6 BSP 编译通过，**板卡未到货，无启动日志、无抖动数据**。

## 现在能验证什么

| 能力 | 状态 |
|---|---|
| host 测试（调度 / IPC / PAL / codegen / time / bench） | ✅ 82 passed |
| QEMU Cortex-M3/M7、RISC-V、x86_64 UEFI | ✅ `cargo xtask hello` 四目标 PASS |
| 同一份 YAML 生成图在 QEMU M7 跑 10k 帧 | ✅ `cargo xtask graph` |
| PAL v0.1 + 双后端 conformance | ✅ 冻结 + 4 个双后端测试 |
| WeAct H750 BSP | ✅ 编译通过；⚠️ 未上板 |
| 真机上下文切换 / 480 MHz / 抖动 | ⛔ 未完成（硬件门） |

## 快速开始

```bash
cargo test                  # 82 passed
cargo xtask hello           # QEMU 四目标
cargo xtask graph           # QEMU M7：生成图 10k 帧
cargo build --target thumbv7em-none-eabihf -p echo-bsp-weact-h750vb
./tools/scripts/h750-run.sh # 板子到货后：OpenOCD 烧录 + semihosting
```

## 硬件

- 已选型：**WeAct STM32H750VBT6 + ST-Link V2**（128 KB Flash / 1 MB RAM）；
- 首启：**HSI 64 MHz**，不配置 PLL 480 MHz；
- 上板步骤、SWD 接线、GDB 调试见 [docs/24-weact-h750-bringup.md](docs/24-weact-h750-bringup.md)；
- **没有真机数据就不写"通过"**：D60 仍未通过，任何实时性声明都必须等 H750 实测。

## 目录

```text
crates/arch/       Arch 抽象、MockArch、Cortex-M DWT 时基
crates/core/       trace Sink / semihosting
crates/kernel/     echo-sched、echo-ipc、echo-time
crates/api/        echo-pal-api（PAL v0.1）
crates/test/       conformance、echo-bench
tools/codegen/     YAML → 静态调度图
tools/scripts/     QEMU / H750 运行脚本
bsp/               QEMU M3/M7/RV32/x86_64、Nucleo-H743、WeAct-H750
docs/              设计文档、ADR、开发日志、上板与 D90 评审
```

## 文档

从 [docs/README.md](docs/README.md) 开始；关键入口：

- [02 需求基线](docs/02-embodied-ai-robot-profile.md)
- [10 开发计划与进度](docs/10-development-plan.md)
- [12 开发日志](docs/12-dev-log.md)
- [23 D90 评审](docs/23-d90-review.md)
- [24 H750 上板与调试](docs/24-weact-h750-bringup.md)

## 边界

- 只做阶段一：实时控制内核；**不做** POSIX 底座、EtherCAT、42 个 crate 的扩张；
- L1 零外部 Rust crate 依赖；
- 当前实现的"抢占上下文切换"仍是 PendSV/SVC 占位，W7 真实切换待 H750 上板；
- D90 结论是 **条件 Go**，不是完成。
