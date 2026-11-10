# 23 · D90 评审（W12）：条件 Go，真机证据仍缺

**状态**：✅ 评审完成（2026-11-11）｜ **结论**：**条件 Go**（host/契约线）＋ **No-Go**（任何真机实时性声明）

> 本文是 W6–W12 的收口。
> 原则：**没有真机数据，就不做实时性承诺**；硬件门保留在账上。

## 1. 证据汇总

| 证据 | 状态 | 位置 |
| --- | --- | --- |
| host 测试 | ✅ **82 passed, 0 failed** | `cargo test` |
| QEMU M7 生成图 | ✅ 10,000 帧、0 丢包、exit=0 | `cargo xtask graph` |
| QEMU Hello 四目标 | ✅ ARM M3/M7、RV32、x86_64 UEFI | `cargo xtask hello` |
| codegen 防漂移 | ✅ 生成物与 YAML 一致 | `tools/codegen` 测试 |
| PAL v0.1 冻结 | ✅ `PAL_VERSION=(0,1,0)`，两后端 conformance | [21](21-pal-freeze.md) |
| 同一 YAML 双后端 | ✅ 宽/窄通道 count/sum/drops/digest 一致 | [22](22-two-backends.md) |
| H7 启动 | ⚠️ 仅编译通过；**无启动日志** | [17](17-h7-bringup.md) |
| H7 上下文切换 | ⚠️ PendSV/SVC 占位 | [18](18-preemptive-sched.md) |
| H7 抖动 / deadline | ⛔ **无数据**（无板卡/探针/示波器） | [20](20-h7-evidence.md) |

## 2. 门槛判定

| 门槛 | 判据 | 结果 |
| --- | --- | --- |
| **D35 · 链路** | 同一 YAML 在 mock 与 QEMU 上跑通 | ✅ **功能通过**（QEMU 10k 帧摘要；host 双后端 trace 一致） |
| **D60 · 真机数据** | H7 上 P99 抖动 + 复现脚本 | ⛔ **未通过**（无硬件） |
| **D90 · 证据包** | 可复现 + PAL 冻结 + 第二后端 | ⚠️ **部分通过**：host/契约侧完成；真机证据缺失 |

## 3. Go / No-Go

### ✅ 条件 Go（继续）

适用于：

- 继续 host 侧内核/契约工作（时间、IPC、调度、PAL、codegen）；
- 继续 DoraOS `pico`/Micro 集成、conformance、工具链；
- 在**明确标注"无真机数据"**的前提下对外讨论原型。

### ⛔ No-Go（当前禁止）

- 任何"硬实时/低抖动/可认证"的对外承诺；
- 把 QEMU/host 数字写成 H7 实测；
- 在拿到真机数据前启动功能安全认证；
- 扩范围到 POSIX/EtherCAT/42 crate（ADR-0019/0020 红线）。

## 4. 下一步 90 天（D120）

| 优先级 | 动作 | 完成判据 |
| --- | --- | --- |
| P0 | 采购/接入 NUCLEO-H743ZI2 + ST-Link + 逻辑分析仪 | `probe-rs list` 能看到探针 |
| P0 | H7 启动 + semihosting/UART 日志 | 看到 `EchoOS H7: boot` |
| P1 | PendSV/PSP 真实上下文切换 | 3 任务抢占顺序与 host 一致 |
| P1 | 执行 [20](20-h7-evidence.md) 的 A/B 抖动实验 | 原始 CSV + 报告 + 复现脚本 |
| P2 | 将 DWT 直方图接入 `echo-bench` | JSON/CSV 由真机产生 |

**D120 失败处置**：若仍无硬件，不继续等；把 EchoOS 定位收敛为"契约 + 证据工具 + host 参考实现"，Micro 底座改用 Zephyr 做第二实现，避免项目停在"差一块板子"。

## 5. W6–W12 交付清单

| 周 | 交付 | 状态 |
| --- | --- | --- |
| W6 | DWT 时基 + trace Sink + H7 骨架 | ✅ 编译；⚠️ 硬件门 |
| W7 | 抢占决策 + RR + PendSV 占位 | ✅ host；⚠️ 真机切换 |
| W8 | 时间队列 + bench 直方图 + DWT 回绕 | ✅ host |
| W9 | H7 证据实验设计 | ⛔ 硬件门，无数据 |
| W10 | PAL v0.1 冻结 + 第二后端 | ✅ |
| W11 | 同一 YAML 双后端对比 | ✅ host + QEMU 摘要 |
| W12 | D90 评审 | ✅ 本文 |

## 6. 决策

| # | 决策 | 理由 |
| --- | --- | --- |
| D-043 | D90 判"条件 Go"而非"Go" | 真机证据缺失；不能把 host 结果升格为实时性结论 |
| D-044 | D60 保留为未通过 | 没有数据就是没有数据；认证/客户材料不能引用 |
| D-045 | 下一阶段先解决硬件，再谈扩展 | 避免在无测量的情况下继续加代码 |

## 7. 一句话

> **链路已经打通，契约已经冻结，证据链还差最后一块：真机。**
> 在拿到 H7 数据之前，EchoOS 的正确状态是"条件 Go"，不是"完成"。
