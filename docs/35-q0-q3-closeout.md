# 35 · Q0–Q3 收口

**状态**：✅ Q0–Q3 核心完成（2026-11-11）
**Tag**：`v0.1.0-alpha.1`（最终 commit 见 tag）
**CI**：GitHub Actions 已绿（run 9/10，runner 固定 `ubuntu-24.04`）

## 1. Gate 证据

| Gate | 证据 | 结果 |
|---|---|---|
| GQ0 | `cargo xtask qemu-runtime` | ✅ |
| GQ1 | `cargo xtask context`（10k 切换） | ✅ |
| GQ2 | `cargo xtask sched` + `pal` | ✅ |
| GQ3 | `cargo xtask fault` + `icount` + CI | ✅ |

## 2. 关键数字

```text
cargo test                 102 passed
cargo xtask sched          20k switches, mutex=1, irq=1
cargo xtask pal            PAL QEMU: OK
cargo xtask stack          STACK OVERFLOW task=0
cargo xtask fault          USAGEFAULT cfsr=0x00010000
cargo xtask icount         两次日志逐字节一致
cargo xtask docs           37 files, 0 broken / 0 bad tables
cargo xtask commits        all Conventional Commits
```

## 3. 已交付

- QEMU MPS2-AN500 runtime（linker/向量/SysTick/semihosting）；
- 真实 PendSV/PSP 上下文切换、SVC、SysTick 时间片；
- idle / block / sleep / wake / mutex 等待队列；
- 静态内存池、栈 canary 报告、trace ring；
- PAL QEMU 后端 + 共享 conformance；
- 故障注入、CFSR、`-icount` 回归、GitHub Actions；
- 文档 link/table lint、commit lint、alpha tag。

## 4. Q2 可选 backlog（不阻塞 Q0–Q3 收口）

| ID | 任务 |
|---|---|
| Q2-04 | 优先级继承 / 天花板 |
| Q2-05 | deadline/overrun |
| Q2-07 | mutex 优先级继承（当前 handoff，无 PI） |
| Q2-08 | 信号量等待队列的 QEMU 集成 |
| Q2-12 | MPU 区域配置 |
| Q2-14 | ISR 后半段 / deferred work |

这些属于 **P1/P2 增强**，不改变 Q0–Q3 的 gate 结论。

## 5. 下一步

1. 代码进入 **H 阶段**：H750 首启、PLL 480、UART、真实上下文切换；
2. Q2 backlog 按实际需求择机清；
3. 任何真机数据必须标注板型/时钟/构建指纹，不用 QEMU 数字替代。
