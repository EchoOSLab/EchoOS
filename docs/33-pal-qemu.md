# 33 · PAL QEMU 后端与共享 conformance（Q2-16/17）

**状态**：✅ Q2-16/17 完成（2026-11-11）｜ **验收**：`cargo xtask pal` PASS

> 目标：让 PAL 不只在 Mock 上通过，而是在 QEMU 的真实 Cortex-M7 runtime 上也跑同一套契约检查。

## 1. 共享契约

新增 `echo-conformance::contract::check_common`：

```rust
pub fn check_common<P: Platform>(platform: &P) -> Result<(), &'static str>;
```

检查项：

1. `Capabilities` 为 Micro（static tasks、无 dynamic memory、channels/trace、无 POSIX）；
2. 初始任务数为 0，`declare` 后为 1；
3. `now_ns` 不回退；
4. channel 容量 > 0；send/recv 值一致；空队列返回 None；
5. 内存池可分配；`pool_used > 0`；free 幂等；
6. trace 事件计数增加。

host 侧：

```bash
cargo test -p echo-conformance --test pal_two_backends
# shared_contract_passes_both_backends
# MockPlatform + SmallMockPlatform 都通过
```

## 2. QEMU Platform 实现

`bsp/echo-bsp-qemu-mps2/src/pal_main.rs`：

| 域 | 实现 |
|---|---|
| Task | 静态计数，最多 4；`current=0`；`yield_now` 空操作 |
| Time | `TICKS × 1ms`（SysTick） |
| Memory | `BlockPool<16, 8>` + `RefCell` |
| Channel | `Mailbox<8>`（容量 7） |
| Trace | 16 条事件，超出计 dropped |
| Capabilities | `Capabilities::micro()` |

启动流程：

```text
reset → SysTick 1kHz → 等 2 个 tick
  → QemuPlatform::new()
  → check_common(&platform)
  → "PAL QEMU: OK" / "PAL QEMU: FAIL <reason>"
  → SYS_EXIT
```

## 3. 验收

```bash
cargo xtask pal
```

```text
PASS  qemu-mps2 PAL conformance (exit=0, 14B)
PAL QEMU: OK
```

## 4. 限制

- QEMU PAL 只做契约检查，不运行完整多任务应用；
- `TaskApi` 是静态计数，不与调度器任务表绑定；
- `MemoryApi` 的 free 幂等通过忽略错误实现；
- 时间来自虚拟 SysTick，不是墙钟；
- 没有 MPU/ISR/阻塞等待集成。

## 5. 文件

| 文件 | 内容 |
|---|---|
| `crates/test/echo-conformance/src/contract.rs` | 共享契约检查 |
| `crates/test/echo-conformance/tests/pal_two_backends.rs` | host 双后端 + 共享契约 |
| `bsp/echo-bsp-qemu-mps2/src/pal_main.rs` | QEMU Platform |
| `tools/scripts/qemu-pal.sh` | QEMU 验收 |
| `xtask/src/main.rs` | `pal` 子命令 |

## 6. 下一步

1. 同步原语阻塞等待队列（Q2-07/08/09）；
2. 栈溢出报告（Q2-11）；
3. Q3 trace ring / CI 首跑 / alpha tag。
