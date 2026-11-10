# 17 · H7 bring-up：代码路径就绪，硬件门未过

**状态**：⚠️ **部分完成**（W6，2026-11-11）｜ **代码路径**：编译通过｜ **硬件门**：❌ 未通过（无 ST-Link / 板卡）

> **诚实边界**：本仓库当前没有 H7 板卡与调试探针（`lsusb` 无 ST-Link，`/dev/ttyACM*` 不存在），
> 因此 W6 **没有**真机启动日志、没有 UART 输出、没有抖动数据。本文只记录代码路径、编译验证与上板验收清单。
> **绝不把编译通过写成"已在 H7 上跑通"。**

## 1. 已完成（代码路径）

| 组件 | 内容 | 验证 |
| --- | --- | --- |
| `crates/arch/echo-arch-cortex-m` | DWT `CYCCNT` 时基、`DwtClock<CPU_HZ>`、`cycles_to_ns` | host 5 个数学测试 |
| `crates/core/echo-trace` | `Sink` trait、`write_str`/`write_bytes`、semihosting 后端 | host 3 个字节流测试 |
| `bsp/echo-bsp-nucleo-h743zi` | H7 启动骨架、DWT 初始化、semihosting 首次启动输出 | `cargo build --target thumbv7em-none-eabihf` 通过 |

```bash
cargo test                     # 48 passed
cargo build --target thumbv7em-none-eabihf -p echo-bsp-nucleo-h743zi
```

## 2. 未完成（硬件门）

| 项 | 状态 |
| --- | --- |
| H7 上电启动 | ❌ 未验证 |
| semihosting 首次启动日志 | ❌ 未验证 |
| UART 输出（USART3 / VCP） | ❌ 未实现 |
| RCC 时钟树（480 MHz、Cache、TCM 配置） | ❌ 未实现 |
| `.data`/`.bss` 初始化 | ❌ 未实现（骨架暂无静态数据） |
| 完整向量表 | ❌ 只有 Reset 两项 |
| 真机抖动 / deadline | ❌ 未测量 |

## 3. 上板验收清单（拿到硬件后按顺序执行）

**硬件**：NUCLEO-H743ZI2、USB 线、8 通道逻辑分析仪（≥24 MHz）、杜邦线。

**工具**：

```bash
cargo install probe-rs-tools --locked
rustup target add thumbv7em-none-eabihf
```

**步骤**：

1. `probe-rs list` 能列出 ST-Link；
2. 烧录并运行：
   ```bash
   cargo build --target thumbv7em-none-eabihf -p echo-bsp-nucleo-h743zi
   probe-rs run --chip STM32H743ZITx target/thumbv7em-none-eabihf/debug/echo-h7
   ```
3. 期望输出：`EchoOS H7: boot (DWT clock enabled)`；
4. 失败排查：DWT 未使能 → 检查 `DEMCR.TRCENA` / `DWT.CTRL.CYCCNTENA`；semihosting 无输出 → 检查 probe-rs 的 semihosting 选项；
5. 通过后进入 UART 后端与完整启动流程（RCC/时钟/向量表）。

**通过判据**：串口/调试器看到启动行，且 `DwtClock::now_ns()` 在两次读取间单调递增。

## 4. 决策

| # | 决策 | 理由 |
| --- | --- | --- |
| D-022 | 真机时基用 DWT `CYCCNT`，不用 SysTick 中断计数 | 无中断开销、单次 volatile 读；回绕在 W8 处理 |
| D-023 | 首次上板用 **semihosting** 输出，UART 后置 | 只需调试器即可验证启动；UART 需要 RCC/引脚配置 |
| D-024 | 没有硬件就只做编译与设计验证，不产出任何"实测"数字 | 抖动/延迟数据必须来自真机；否则 W7 的验收会被污染 |
| D-025 | H7 骨架先最小化（只有 Reset 向量） | 先验证"能启动"；完整向量表/异常处理在 UART 之后补 |

## 5. 文件

| 文件 | 内容 |
| --- | --- |
| `crates/arch/echo-arch-cortex-m/src/lib.rs` | DWT 时基 + `cycles_to_ns` |
| `crates/arch/echo-arch-cortex-m/tests/clock_math.rs` | 5 个数学测试 |
| `crates/core/echo-trace/src/lib.rs` | `Sink` + semihosting |
| `crates/core/echo-trace/tests/write.rs` | 3 个字节流测试 |
| `bsp/echo-bsp-nucleo-h743zi/` | H7 骨架 + linker script |

## 6. 下一步（W7）

W7 是**抢占调度与上下文切换**，可以完全在 host + MockArch 上验证，不依赖 H7：

1. `echo-sched`：静态优先级抢占 + 同优先级 RR 的决策逻辑；
2. `echo-arch-mock`：可注入中断、记录切换序列；
3. Cortex-M 的 PendSV/SVC 汇编先写、编译通过，真机验证等硬件门；
4. 新增 `docs/18-preemptive-sched.md`，开发日志追加 `W7-001`。
