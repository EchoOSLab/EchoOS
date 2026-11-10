# 16 · QEMU Cortex-M7 集成（3 节点图，功能验证）

**状态**：已实现（W5，2026-11-11）｜ **验收**：`cargo xtask graph` PASS｜ **host 测试**：40 个全绿

> 本文记录 W5 的 QEMU 集成契约。
> **QEMU 只证明功能与语义，不产出抖动数字**——抖动证据必须来自 H7 真机（W6/W9）。

## 1. 目标与结果

在 QEMU `mps2-an500`（Cortex-M7）上：

- 加载 **codegen 生成的静态调度表**（`dataflows/generated/three_node.rs`）；
- 跑 sensor → control → actuator 3 节点图 10,000 帧；
- 通过 semihosting 输出 `EchoOS: 3-node graph OK` 并干净退出。

```bash
cargo xtask graph
```

```text
PASS  qemu-m7 3-node graph (exit=0, 25B)
```

对照：`cargo xtask hello` 的四目标冒烟仍 PASS（回归未破坏）。

## 2. 实现结构

```text
bsp/echo-bsp-qemu-cortex-m/src/graph.rs
  ├── Reset_Handler（向量表 + SP + bl echo_main）
  ├── TickArch：每帧显式推进 1000 ns（QEMU 专用）
  ├── include!("../../../dataflows/generated/three_node.rs")
  ├── 3 个任务：sensor / control / actuator（两个 Mailbox<8>）
  ├── CyclicExecutive::new(&arch, task_defs([...]), FRAME_NS)
  ├── for 0..10_000 { run_frame(); arch.advance(FRAME_NS); }
  └── semihosting_write0("EchoOS: 3-node graph OK") + SYS_EXIT
```

**同一个生成物**同时被 host conformance 与 QEMU BSP 使用：

- `crates/test/echo-conformance/tests/generated_graph.rs`：生成图 vs 手写图 1000 帧 trace 一致；
- `bsp/echo-bsp-qemu-cortex-m/src/graph.rs`：生成图在 Cortex-M7 上跑 10k 帧。

这就是 D35 的"同一份 YAML，两个运行环境"的功能部分。

## 3. `TickArch` 的语义与边界

```rust
struct TickArch { now_ns: Cell<u64> }
impl Arch for TickArch { fn now_ns(&self) -> u64 { ... } }
```

- 每帧由调用者 `advance(FRAME_NS)`，**不读硬件时钟**；
- 目的是让 QEMU 上的调度 trace 完全可复现；
- **不能**用它报告抖动、WCET、deadline miss 的真实值；
- H7 真机将换成 SysTick/DWT 时基（W6）。

## 4. 测试与验收

| 项 | 结果 |
| --- | --- |
| `cargo xtask graph` | PASS，exit=0，输出 25 B |
| 10,000 帧 | 0 丢包；`actuator.count == 10_000`；`sum == n(n-1)` |
| `cargo xtask hello` | 四目标仍 PASS |
| `cargo test` | 40 passed，0 failed |
| `cargo fmt --check` | OK |
| `cargo clippy` | 无警告 |

## 5. 明确不证明的事

- ❌ QEMU 的周期/抖动精度：不模拟 cache、流水线、总线争用；
- ❌ 1 kHz 的真实抖动：`TickArch` 是逻辑推进；
- ❌ 抢占、优先级反转、MPU 保护：属 W7/W8；
- ❌ UART 输出：QEMU 用 semihosting，真机才用 UART；
- ❌ H7 板级启动：W6。

## 6. 决策

| # | 决策 | 理由 |
| --- | --- | --- |
| D-017 | QEMU 用 `TickArch`（每帧推进），不用硬件时基 | QEMU 无周期精度；先证明调度/IPC/生成图语义 |
| D-018 | 图 demo 用独立 bin `echo-arm-graph`，不改 `echo-arm` | `Hello EchoOS` 矩阵保留为回归基线 |
| D-019 | QEMU 用 semihosting 输出，H7 换 UART | semihosting 只适合 QEMU；真机必须真实外设 |
| D-020 | 生成物放 `dataflows/generated/`，test 与 BSP 共用 | 一份 YAML 一份产物，避免两份生成物漂移 |
| D-021（偏差） | W5 未引入 `echo-arch-cortex-m` crate；`TickArch` 暂放 BSP | 真机时基（SysTick/DWT）与 H7 一起做，避免为 QEMU 造一个假 Arch |

## 7. 文件

| 文件 | 内容 |
| --- | --- |
| `bsp/echo-bsp-qemu-cortex-m/src/graph.rs` | QEMU M7 图 demo |
| `bsp/echo-bsp-qemu-cortex-m/Cargo.toml` | 新增 `echo-arm-graph` bin + 三个依赖 |
| `tools/scripts/qemu-graph.sh` | 构建 + QEMU + 断言 |
| `xtask/src/main.rs` | `cargo xtask graph` |
| `dataflows/generated/three_node.rs` | 共享生成物 |

## 8. 下一步（W6）

1. **硬件前置**：确认 `NUCLEO-H743ZI2` + 逻辑分析仪已到货；没有硬件则先做设计/编译验证，不假装有数据；
2. `echo-arch-cortex-m`：SysTick/DWT 时基、`Arch` 实现、启动/向量表收口；
3. `echo-trace`：UART 后端（替代 semihosting）；
4. H7 上复跑同一张生成图，输出 `EchoOS: 3-node graph OK`；
5. 新增 `docs/17-h7-bringup.md`，开发日志追加 `W6-001`。
