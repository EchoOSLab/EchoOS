# 27 · QEMU MPS2-AN500 Runtime（Q0）

**状态**：✅ Q0 完成（2026-11-11）｜ **验收**：`cargo xtask qemu-runtime` PASS

> 本文是 Q0 的最小 runtime 契约。
> 目标：在没有 H750 的情况下，先让 QEMU Cortex-M7 具备“完整启动 + 向量 + SysTick + 可断言退出”的基础。

## 1. 范围

Q0 只做 runtime 骨架，不做上下文切换：

- ✅ linker script；
- ✅ 16 个系统向量 + VTOR；
- ✅ `.data` 拷贝 / `.bss` 清零 / FPU；
- ✅ SysTick 1 kHz（QEMU 虚拟时钟）；
- ✅ semihosting 输出 + `SYS_EXIT`；
- ❌ PendSV/SVC 真实切换（Q1）；
- ❌ 调度运行时（Q2）；
- ❌ MPU/Cache/TCM（Q3/H1）。

## 2. 内存布局

`bsp/echo-bsp-qemu-mps2/link.x`：

| 区域 | 地址 | 大小 | 用途 |
|---|---|---|---|
| CODE | `0x0000_0000` | 1 MB | 向量表 + `.text` + `.rodata` |
| RAM | `0x2000_0000` | 256 KB | `.data` / `.bss` / 栈 |
| 栈顶 | `0x2004_0000` | — | `_stack_top` |

- 向量表固定在 `0x0000_0000`；
- `VTOR = 0x0000_0000`；
- `.data` 的加载地址由 `_sidata` 指向 CODE 段。

## 3. 启动时序

```text
QEMU -kernel
  │
  ▼
Reset_Handler
  ├── SP = _stack_top
  ├── copy .data
  ├── zero .bss
  └── bl runtime_main
        ├── VTOR = 0
        ├── FPU enable
        ├── semihosting "boot"
        ├── SysTick 1 kHz
        ├── cpsie i
        └── 等 10 个 tick
              ├── "systick OK"
              └── SYS_EXIT
```

## 4. 向量表

| # | 向量 | 处理 |
|---|---|---|
| 0 | SP | `_stack_top` |
| 1 | Reset | `Reset_Handler` |
| 2 | NMI | `Default_Handler` |
| 3 | HardFault | `HardFault_Handler` |
| 4 | MemManage | `MemManage_Handler` |
| 5 | BusFault | `BusFault_Handler` |
| 6 | UsageFault | `UsageFault_Handler` |
| 7–10 | reserved | `Default_Handler` |
| 11 | SVC | `Default_Handler`（Q1 替换） |
| 12 | DebugMon | `Default_Handler` |
| 13 | reserved | `Default_Handler` |
| 14 | PendSV | `Default_Handler`（Q1 替换） |
| 15 | SysTick | `SysTick_Handler` |

## 5. SysTick

```rust
SYST_RVR = CPU_HZ / TICK_HZ - 1
SYST_CVR = 0
SYST_CSR = ENABLE | TICKINT | CLKSOURCE
```

- `CPU_HZ = 25_000_000` 只是 QEMU 的标称分频；
- `TICK_HZ = 1_000`；
- 等 10 个 tick 后打印 `systick OK`；
- **这不是 1 ms 实时精度证据**，只是功能验证。

## 6. 验收

```bash
cargo xtask qemu-runtime
```

期望：

```text
PASS  qemu-mps2 runtime (exit=0, 44B)
```

日志内容：

```text
EchoOS MPS2: boot
EchoOS MPS2: systick OK
```

底层命令见 `tools/scripts/qemu-runtime.sh`。

## 7. 失败路径

| 现象 | 检查 |
|---|---|
| 无输出 | semihosting 配置、`-kernel` ELF、向量表 |
| `HARDFAULT` | SP/向量/链接脚本 |
| 一直等不到 tick | SysTick 寄存器、`cpsie i`、向量 15 |
| 链接错误 `_stack_top` | linker script 是否参与 |
| 退出码非 0 | `SYS_EXIT` 参数、QEMU semihosting |

## 8. 文件

| 文件 | 内容 |
|---|---|
| `bsp/echo-bsp-qemu-mps2/src/main.rs` | 启动、向量、SysTick、semihosting |
| `bsp/echo-bsp-qemu-mps2/link.x` | CODE/RAM/栈/段布局 |
| `bsp/echo-bsp-qemu-mps2/build.rs` | 链接脚本参数 |
| `tools/scripts/qemu-runtime.sh` | Q0 验收脚本 |
| `xtask/src/main.rs` | `qemu-runtime` 子命令 |

## 9. 下一步（Q1）

1. TCB/栈帧布局；
2. MSP/PSP + 首次任务异常帧；
3. PendSV 保存/恢复；
4. SVC 入口；
5. 3 任务抢占 + RR；
6. 10k 切换压力 + canary。

Q1 完成后，QEMU 才具备“真实上下文切换”，而不是只有调度决策逻辑。
