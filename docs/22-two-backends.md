# 22 · 同一份 YAML，两个后端（W11）

**状态**：✅ host 对比完成｜ **测试**：82 passed（新增 2）｜ **QEMU**：摘要级 PASS

> 本文记录 W11 的对比结果。
> 输入：`dataflows/three_node.yaml` → `dataflows/generated/three_node.rs`（同一份生成物）。

## 1. 对比矩阵

| 环境 | 通道 | 帧数 | 结果 | 逐帧 trace |
| --- | --- | --- | --- | --- |
| host · `Mailbox<8>`（容量 7） | 宽 | 10,000 | 0 丢包；count=10,000；sum 正确 | ✅ 完整 |
| host · `Mailbox<2>`（容量 1） | 窄 | 10,000 | 同上；与宽通道 **digest 完全一致** | ✅ 完整 |
| QEMU M7（`mps2-an500`） | 生成图 | 10,000 | `EchoOS: 3-node graph OK`，exit=0 | ❌ 只有摘要 |

**host 两个后端**都使用同一份 `dataflows/generated/three_node.rs`，只有 `Mailbox` 的 const 容量不同。

## 2. 验收测试

```bash
cargo test -p echo-conformance --test two_backends_graph
```

| 测试 | 证明 |
| --- | --- |
| `same_generated_graph_on_two_channel_capacities` | 宽/窄通道的 count、sum、drops、trace digest 完全一致 |
| `generated_constants_are_shared` | `FRAME_NS` / `NODE_COUNT` / `NODE_NAMES` 与 YAML 一致 |

digest 覆盖：帧起始时间、三个槽位起始时间；每帧 10,000 次 FNV 累积。

## 3. QEMU 的边界

W5 的 `cargo xtask graph` 输出：

```text
PASS  qemu-m7 3-node graph (exit=0, 25B)
```

它证明：

- 同一份生成图在 Cortex-M7 裸机上能跑；
- 10,000 帧、0 丢包、sum 条件成立。

它**不证明**：

- 逐帧 trace 与 host 完全一致（QEMU 只打印摘要）；
- 抖动/延迟（QEMU 无周期精度，`TickArch` 是逻辑推进）；
- 通道容量变化下的真机行为（QEMU 固定 `Mailbox<8>`）。

## 4. 结论

1. **YAML → 静态图 → host/裸机** 的链路已经打通；
2. 通道容量（7 vs 1）不改变调度与消息语义——只要每帧消息数不超过容量；
3. 真机时序证据仍然缺失（W9 硬件门），本对比**不产生**抖动数字；
4. DoraOS 可以基于 `dataflows/generated/` + PAL v0.1 继续接 Micro 运行时。

## 5. 决策

| # | 决策 | 理由 |
| --- | --- | --- |
| D-040 | 用 trace digest 而不是打印全量 trace 做对比 | 无分配、可在一行断言；覆盖帧/槽位时间 |
| D-041 | 窄通道选容量 1（`Mailbox<2>`） | 最苛刻但仍能承载每帧 1 条消息，能暴露容量假设 |
| D-042 | QEMU 只报摘要，不参与逐帧对比 | 不把"摘要一致"夸大成"trace 一致" |

## 6. 文件

| 文件 | 内容 |
| --- | --- |
| `dataflows/generated/three_node.rs` | 共享生成物 |
| `crates/test/echo-conformance/tests/two_backends_graph.rs` | 宽/窄通道对比 + 常量检查 |
| `tools/scripts/qemu-graph.sh` | QEMU 摘要验收 |

## 7. 下一步（W12）

D90 评审：

1. 汇总 D35 / D60 / D90 的证据状态；
2. 明确 **Go / 条件 Go / No-Go**；
3. 硬件门（W6/W9）保留，不包装成通过；
4. 新增 `docs/23-d90-review.md`，追加 `W12-001`。
