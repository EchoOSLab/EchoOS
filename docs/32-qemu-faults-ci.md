# 32 · QEMU 故障注入、`-icount` 回归与 CI（Q3 部分）

**状态**：🟡 Q3 进行中（2026-11-11）
**已完成**：故障注入、CFSR 状态、`-icount` 确定性、GitHub Actions 配置
**未完成**：trace ring、CI 实际首跑、alpha tag、MPU/NVIC/ISR

## 1. 故障注入

新增 `echo-mps2-fault`：

```rust
system_init();
write_boot("EchoOS MPS2: fault injection (udf)\n");
unsafe { core::arch::asm!("udf #0"); }
```

- 使能 `SHCSR.USGFAULTENA`；
- `udf #0` → `UsageFault`；
- handler 读取 CFSR/HFSR 并 semihosting 输出后退出。

验收：

```bash
cargo xtask fault
```

```text
PASS  qemu-mps2 fault injection (exit=0, 93B)
EchoOS MPS2: USAGEFAULT cfsr=0x00010000 hfsr=0x00000000
```

`CFSR=0x0001_0000` 即 `UNDEFINSTR` 位，说明故障状态寄存器路径可用。

## 2. `-icount` 确定性回归

```bash
cargo xtask icount
```

- QEMU `-icount shift=0` 跑两次 `echo-mps2-sched`；
- 两次日志必须**逐字节一致**；
- 结果：

```text
PASS  qemu-mps2 -icount deterministic (exit=0/0)
EchoOS MPS2: sched OK t0=9348 t1=10422 t2=100 idle=100 wake=1 ticks=27 switches=20000
```

**边界**：`-icount` 是确定性指令计数模型，不是周期精确，也不是抖动/WCET 证据。

## 3. GitHub Actions

新增 `.github/workflows/ci.yml`：

```text
checkout
rust toolchain + targets
apt install qemu-system-arm/qemu-system-misc/qemu-system-x86/ovmf
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo xtask hello
cargo xtask qemu-runtime
cargo xtask context
cargo xtask sched
cargo xtask fault
cargo xtask icount
cargo xtask graph
```

状态：

- ✅ workflow 已写入；
- 🟡 **还没有在 GitHub 上实际跑过**（本轮只在本地验证命令链）；
- 首次 push 后需要看 Actions 结果，再决定是否调 runner/包名。

## 4. Q3 任务状态

| ID | 任务 | 状态 |
|---|---|---|
| Q3-01 | trace ring buffer + 事件 ID | ⬜ |
| Q3-02 | HardFault/MemManage/BusFault/UsageFault 状态 | ✅ UsageFault 验证 |
| Q3-03 | 故障注入测试 | ✅ `udf` |
| Q3-04 | `-icount` 确定性回归 | ✅ |
| Q3-05 | GitHub Actions | 🟡 workflow 已加，首跑待做 |
| Q3-06 | 文档/commit lint | ⬜ |
| Q3-07 | alpha tag | ⬜ |

## 5. 限制

- 只注入了 `UsageFault`；MemManage/BusFault 只有 handler，未做真实 MPU/总线故障注入；
- 没有 trace ring，故障现场只有 CFSR/HFSR；
- CI 未实际运行，不能声称“CI 绿”；
- QEMU 不提供时序证据。

## 6. 文件

| 文件 | 内容 |
|---|---|
| `bsp/echo-bsp-qemu-mps2/src/fault_main.rs` | `udf` 故障注入 |
| `tools/scripts/qemu-fault.sh` | 故障验收 |
| `tools/scripts/qemu-icount.sh` | 确定性回归 |
| `.github/workflows/ci.yml` | GitHub Actions |
| `xtask/src/main.rs` | `fault` / `icount` 子命令 |
