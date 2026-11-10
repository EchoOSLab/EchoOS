# 12 · 开发日志（滚动更新）

> **规则**：每个改变代码或设计的提交，必须在本文件追加一条记录。
> 每条记录至少包含：目标、改动、验证、决策/偏差、下一步。
> 功能契约写在对应的专题文档（如 [11](11-arch-and-sched.md)），本文件只记录"发生了什么"。

## 索引

| 日期 | ID | 类型 | 摘要 | commit |
| --- | --- | --- | --- | --- |
| 2026-11-10 | DOC-001 | 设计 | ADR-0020 POSIX 兼容分区 + 交叉引用同步 | `1f8888f` |
| 2026-11-10 | M0-001 | 工程 | workspace + 四目标 QEMU `Hello EchoOS` | `2644974`…`b902f11` |
| 2026-11-10 | W1-001 | 功能 | `trait Arch` + `MockArch` + `CyclicExecutive` | `a981fec` |
| 2026-11-10 | W1-002 | 文档 | W1 日志与计划进度跟踪 | 本条目 |
| 2026-11-10 | W2-001 | 功能 | SPSC Mailbox + 3 节点图（10⁶ 帧 0 丢包） | `65e839d` |

---

## 2026-11-10 · DOC-001 · ADR-0020

**目标**：给 POSIX 兼容性一个明确边界，避免"支持 POSIX"变成范围爆炸。

**改动**：
- `docs/05-design-decisions.md`：新增 ADR-0020（PSE51 子集、时间分区、picolibc 受控例外、16 条验收用例）。
- `docs/01`、`docs/04`、`docs/08`、`docs/09`、`docs/README.md`：同步非目标、crate 树、R8 规则、M12、风险 T25–T28。

**决策**：
- POSIX 只作为 M12 的可选服务域；硬实时域保持无 POSIX、无堆。
- 90 天内不写一行 POSIX 实现。

**下一步**：回到 EchoOS 主线，先打通 host 链路。

---

## 2026-11-10 · M0-001 · QEMU 四目标启动

**目标**：在没有任何内核的情况下，先证明"工具链 + QEMU + 工程结构"可用。

**改动**：
- 虚拟 workspace；`default-members` 只含 host 可编译 crate。
- `bsp/echo-bsp-qemu-cortex-m`（M3/M7，semihosting）
- `bsp/echo-bsp-qemu-riscv-virt`（RV32，UART0）
- `bsp/echo-bsp-qemu-x86`（x86_64 UEFI + OVMF，ConOut/COM1）
- `xtask` + `tools/scripts/qemu-hello.sh`（矩阵冒烟）

**验证**：
```text
cargo xtask hello
PASS arm-cortex-m3 (exit=0)   PASS arm-cortex-m7 (exit=0)
PASS riscv32 (exit=0)        PASS x86_64 (exit=33, 正常)
```
四目标总耗时 ~3 s（改为 guest 自行退出后，从 70 s 降下来）。

**决策/偏差**：
- 串口输出改为**文件后端**，绕开终端 stdio 捕获问题（`b902f11`）。
- 三个 guest 自行退出：ARM semihosting `SYS_EXIT`、RISC-V `sifive_test`、x86 `isa-debug-exit`。
- x86 图形窗口额外用 UEFI `ConOut` 输出，避免只写 COM1 时窗口里看不到。

**下一步**：W1 的 host 垂直切片。

---

## 2026-11-10 · W1-001 · Arch + Mock + 循环执行体

**目标**：建立"能在 host 上确定性测试"的最小内核切片，为 W2 的 3 节点图打地基。

**改动**：
- `crates/arch/echo-arch-api`：`trait Arch { fn now_ns(&self) -> u64; }`
- `crates/arch/echo-arch-mock`：`MockArch`（`new/with_time/advance/set`，饱和推进）
- `crates/kernel/echo-sched`：
  - `TaskDef` / `SlotStat` / `FrameStats` / `SchedError`
  - `CyclicExecutive::new` 校验 `frame_ns > 0`、`Σ slot_ns ≤ frame_ns`
  - `run_frame` 每帧读一次时钟，槽位偏移由预算推导
- `docs/11-arch-and-sched.md`：实现契约与限制

**验证**：
```text
cargo test
echo-arch-mock: 5 passed
echo-sched (tests/cyclic.rs): 8 passed
合计 13 passed, 0 failed
cargo fmt --check: OK
cargo xtask hello: 四目标仍 PASS（回归）
```

**决策/偏差**：
- **D-001**：`echo-util` 延后——bitmap 等就绪队列（M2），ring 等 IPC/trace；先写工具库没有消费者。
- **D-002**：`run_frame` 只读一次时钟；时钟推进由调用者负责（真机硬件 / host mock），保证两端语义一致。
- **D-003**：`slot_ns` 是时间预算，不是实测执行时间；WCET 与超限检测放 W8。
- **D-004**：测试放 `tests/` 集成测试，只测公开 API；`no_std` 库本身零测试 harness 依赖。

**偏差**：W1 原计划先写 `echo-util`；实际按"消费者驱动"调整为先做 Arch/Mock/Sched。计划文档已同步。

**下一步（W2）**：
1. `echo-ipc`：`Mailbox<T, const N: usize>`（`try_send`/`try_recv`，满/空语义）。
2. 手写 3 节点图：sensor → control → actuator。
3. 10⁶ 帧、0 丢包、trace 可复现。
4. 把结果写回本日志与 `docs/11`。

---

## 2026-11-10 · W1-002 · 文档跟踪机制

**目标**：让"每个功能都有对应文档、每次提交都有记录"成为可执行的规则，而不是口号。

**改动**：
- 新增本文件 `docs/12-dev-log.md`（索引 + 条目模板 + 滚动记录）。
- `docs/README.md`：索引加入 10/11/12。
- `docs/10-development-plan.md`：W1 实际执行顺序、进度表、7 天清单勾选状态。

**验证**：文档间相对链接与锚点检查通过（0 broken）。

**下一步**：W2 的 `Mailbox` 与 3 节点图；完成后追加 W2-001。

---

## 2026-11-10 · W2-001 · SPSC Mailbox + 3 节点图

**目标**：把 sensor → control → actuator 串成静态图，证明 10⁶ 帧、0 丢包、顺序正确。

**改动**：
- `crates/kernel/echo-ipc/src/lib.rs`：`Mailbox<const N>`（u32 载荷、`try_send`/`try_recv`、`AtomicU32` 环形、零 `unsafe`）+ 9 个单测。
- `crates/kernel/echo-ipc/tests/spsc.rs`：双线程 SPSC 100k 条 FIFO 测试。
- `crates/test/echo-conformance/tests/three_node.rs`：3 节点图 10⁶ 帧。
- `docs/13-ipc-and-three-node.md`：契约、内存序、测试与限制。

**验证**：
```text
cargo test
echo-ipc 单测:            9 passed
echo-ipc spsc:            1 passed（双线程 100k，FIFO）
echo-conformance:         1 passed（10^6 帧，0 丢包，sum 正确）
累计 24 passed, 0 failed
cargo fmt --check:        OK
cargo clippy:             无警告
cargo xtask hello:        四目标回归 PASS
```

**决策/偏差**：
- **D-005**：v0 载荷固定 `u32`。`AtomicU64` 在 thumbv7m/riscv32 上没有 64 位原子操作，泛型载荷留到 M4 的池化消息。
- **D-006**：保留一个空槽区分满/空，容量 = `N-1`；不引入额外 count 原子。
- **D-007**：全原子实现 + `forbid(unsafe_code)`，守住 R3 的 `echo-ipc` unsafe = 0，可直接放 `static`。
- **D-008**：3 节点图放 `crates/test/echo-conformance`，跨 crate 验收集中管理。
- **偏差**：W2 原计划还包含 `CyclicExecutive`，它已在 W1 完成；W2 实际只做 IPC + 图。

**下一步（W3）**：
1. PAL 5 域（Task/Time/Memory/Channel/Observability）trait 与 mock 后端；
2. `echo-conformance` 增加 PAL 用例骨架；
3. 与 DoraOS 侧对齐 `dora-pal-conformance` 的用例命名。

---

## 条目模板（复制用）

```markdown
## YYYY-MM-DD · ID · 一句话标题

**目标**：

**改动**：
- 

**验证**：
```text
命令与结果
```

**决策/偏差**：
- 

**下一步**：
1. 
```
