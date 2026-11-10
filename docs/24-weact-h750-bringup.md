# 24 · WeAct STM32H750VBT6 上板与调试

**状态**：✅ BSP 编译通过（`echo-h750`，约 13 KB）｜ ⚠️ 待上板验证（无硬件）
**目标板**：WeAct MiniSTM32H7xx · STM32H750VBT6 · 128 KB Flash / 1 MB RAM · Cortex-M7

> **先读边界**：H750 不是 H743。Flash 只有 **128 KB**，本 BSP 首启跑 **HSI 64 MHz**，
> 不配置 PLL 480 MHz。W9 的抖动数据必须写明"H750 @64 MHz"，不能冒充 H743 @480 MHz。

## 1. 硬件与接线

### 1.1 板子接口

- 供电：USB-C，3.3–5.5 V；
- 调试：板上 **P3 四针 SWD 排针**；
- 用户键：K1（PC13）；复位键 NRST；BOOT0 键；
- 串口：**没有板载 USB-UART**；PA9/PA10 是 USART1，需要外接 USB-TTL（下一步再配）。

### 1.2 SWD 接线（P3）

| P3 引脚 | 信号 | 接到 ST-Link V2 |
| --- | --- | --- |
| 1 | 3V3 | **不接**（板子 USB-C 已供电） |
| 2 | SWDIO | SWDIO |
| 3 | SWCLK | SWCLK |
| 4 | GND | GND |

- 只接 **GND / SWDIO / SWCLK** 三根；不要用 ST-Link 的 3.3V 给板子供电。
- BOOT0 键**不要按住**（内部 Flash 启动）。
- 上电顺序：先接 USB-C 给板子供电，再插 ST-Link；共地即可。

> 接线依据：WeAct `STM32H7xx SchDoc V12.pdf` 的 P3（1=3V3，2=SWDIO，3=SWCLK，4=GND）。

## 2. 工具安装

```bash
sudo apt update
sudo apt install -y openocd gdb-multiarch
```

可选（用 probe-rs 烧录/调试）：

```bash
cargo install probe-rs-tools --locked
```

## 3. 构建与烧录

```bash
cd ~/Dev/RustProjects/EchoOS
cargo build --target thumbv7em-none-eabihf -p echo-bsp-weact-h750vb
```

产物：`target/thumbv7em-none-eabihf/debug/echo-h750`。

一键烧录 + semihosting 输出：

```bash
./tools/scripts/h750-run.sh
```

脚本做的事：

```text
openocd -f interface/stlink.cfg -f target/stm32h7x.cfg \
  -c "adapter speed 2000" \
  -c "arm semihosting enable" \
  -c "program target/.../echo-h750 verify" \
  -c "reset run"
```

OpenOCD 会保持运行，**semihosting 输出显示在当前终端**；Ctrl-C 退出。

### 先验证连接

```bash
lsusb | grep -i st-link
openocd -f interface/stlink.cfg -f target/stm32h7x.cfg -c "init; shutdown"
```

前者应看到 ST-Link；后者应打印 `Info : stm32h7x.cpu: hardware has 6 breakpoints...` 且无 error。

## 4. 预期输出

```text
EchoOS H750: boot (HSI 64MHz, 128KB flash)
EchoOS H750: 3-node graph OK
```

- 第一行：DWT 已使能、VTOR/FPU 已初始化；
- 第二行：`dataflows/generated/three_node.rs` 的 3 节点图跑完 10,000 帧、0 丢包。

若看到 `schedule FAIL` / `3-node graph FAIL`：先查链接脚本和 `.bss` 清零；
若看到 `HARDFAULT`：查 VTOR、FPU、栈顶和向量表。

## 5. GDB 调试

终端 1（OpenOCD 服务器 + semihosting）：

```bash
openocd -f interface/stlink.cfg -f target/stm32h7x.cfg \
  -c "arm semihosting enable"
```

终端 2（GDB）：

```bash
gdb-multiarch target/thumbv7em-none-eabihf/debug/echo-h750 \
  -ex "target extended-remote :3333" \
  -ex "monitor reset halt" \
  -ex "load" \
  -ex "break echo_main" \
  -ex "continue"
```

常用命令：

```text
break run_graph
break HardFault_Handler
monitor reset halt
info registers
x/4xw 0x08000000      # 向量表
p/x $sp
```

> 如果 `load` 报 flash 错误，先 `monitor reset halt`，再 `monitor stm32h7x mass_erase 0`，
> 然后重新 `load`。H750 只有 128 KB，不要把 H743 的 2 MB 链接脚本拿来用。

## 6. probe-rs（可选）

```bash
probe-rs list
probe-rs run --chip STM32H750VBTx \
  target/thumbv7em-none-eabihf/debug/echo-h750
```

注意：

- probe-rs 对 **semihosting** 的显示支持因版本而异；看不到输出时用 OpenOCD；
- 若 probe-rs 不认 H750，用 `probe-rs chip list | grep -i H750` 确认芯片名；
- 烧录成功 ≠ 启动成功，最终以串口/semihosting 的启动行为准。

## 7. 常见问题

| 现象 | 原因/处理 |
| --- | --- |
| `lsusb` 无 ST-Link | USB 线、驱动、udev；换线/换口 |
| `Error: init mode failed` | SWDIO/SWCLK 接反、目标未供电、BOOT0 被按住；降到 `adapter speed 1000` |
| `Error: flash write failed` | Flash 保护/没擦除；`monitor stm32h7x mass_erase 0` 后重试 |
| 没有 semihosting 输出 | 忘记 `arm semihosting enable`；或程序跑飞/进 HardFault |
| 一上电就 `HARDFAULT` | 栈顶/向量表/FPU/VTOR；检查 `0x08000000` 处的 SP 和 Reset 向量 |
| 程序跑一次就停 | `echo_main` 末尾是 `wfi` 循环，属正常；不是死机 |

## 8. 当前限制（必须知道）

| 项 | 状态 |
| --- | --- |
| 时钟 | **HSI 64 MHz**；PLL 480 MHz 未配置 |
| Flash | **128 KB**；当前镜像约 13 KB；未使用外部 8 MB QSPI |
| 串口 | 未实现；板子无板载 USB-UART，需外接 USB-TTL 到 PA9/PA10 |
| 上下文切换 | PendSV/SVC 仍是占位（W7） |
| 抖动数据 | ❌ 无；即使上板，也需要逻辑分析仪 + 明确时钟 |
| 与 H743 的关系 | H750 是**廉价 bring-up/功能验证板**，不是 H743 的 2 MB/480 MHz 验收替代 |

## 9. 下一步

1. **上板首启**：看到两行输出，确认栈/向量/FPU/DWT；
2. **PLL 480 MHz**：配置 HSE 25 MHz → PLL1 → 480 MHz，`CPU_HZ` 改 480 MHz，重新验证 DWT；
3. **UART**：PA9/PA10（USART1）+ USB-TTL，接 `echo-trace` 的 UART sink；
4. **W7**：实现 PendSV/PSP 真实上下文切换；
5. **W9**：逻辑分析仪测 GPIO，报告必须写清 `H750 @?MHz`、构建指纹、样本数。

## 10. 决策

| # | 决策 | 理由 |
| --- | --- | --- |
| D-046 | 首启用 HSI 64 MHz，不先配 PLL | 先验证最小启动链路；PLL 错误会伪装成 HardFault |
| D-047 | 首启用 semihosting，不先做 UART | 板子无板载 USB-UART；semihosting 只需 ST-Link |
| D-048 | 链接脚本按 128 KB Flash 写死 | 防止误用 H743 的 2 MB 脚本，链接期就暴露超限 |
| D-049 | H750 数据必须标注板型/时钟 | 不能把 64 MHz H750 数据写成 480 MHz H743 结论 |

## 11. 文件

| 文件 | 内容 |
| --- | --- |
| `bsp/echo-bsp-weact-h750vb/src/main.rs` | 启动、向量表、FPU/VTOR、DWT、3 节点图 |
| `bsp/echo-bsp-weact-h750vb/link.x` | 128 KB Flash / 128 KB DTCM / 512 KB AXI |
| `tools/scripts/h750-run.sh` | 构建 + OpenOCD 烧录 + semihosting |
| `docs/24-weact-h750-bringup.md` | 本文 |
