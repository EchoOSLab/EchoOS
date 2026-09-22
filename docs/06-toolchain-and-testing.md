# 06 · 工具链、构建与测试

## 6.1 当前环境状态

| 项 | 状态 | 说明 |
| --- | --- | --- |
| `rustc` / `cargo` | ✅ 1.98.1 (2026-09-01) stable | 满足最低要求 1.88（`naked_asm!` 稳定版本） |
| edition 2024 | ✅ | `resolver = "3"` 可用 |
| `rustup` | ✅ | 可安装交叉 target |
| `rust-lld` / `rust-objcopy` | ✅ 已随工具链提供 | **无需额外安装 binutils** |
| `thumbv7m-none-eabi` 等 target | ❌ **未安装** | 必须补 |
| QEMU (`qemu-system-arm` / `-misc`) | ❌ **未安装** | 必须补 |
| `probe-rs` | ❌ **未安装** | 真机阶段补 |
| nightly 工具链 | 未安装（符合 ADR-0010，不需要） | — |

### 一键补齐

```bash
# 1. 交叉编译 target
rustup target add thumbv7m-none-eabi          # Cortex-M3 (QEMU mps2-an385)
rustup target add thumbv7em-none-eabihf       # Cortex-M4F/M7F (STM32F4, 带硬浮点)
rustup target add thumbv6m-none-eabi          # Cortex-M0+ (低端，可选)
rustup target add riscv32imac-unknown-none-elf # RISC-V RV32IMAC (QEMU virt, ESP32-C3)

# 2. QEMU（Debian/Ubuntu）
sudo apt install qemu-system-arm qemu-system-misc gdb-multiarch

# 3. 真机调试工具（M3 之后再装）
cargo install probe-rs-tools --locked

# 4. 质量工具
cargo install cargo-deny cargo-geiger cargo-binutils
rustup component add llvm-tools-preview        # cargo-size / cargo-objdump 需要

# 5. 验证
rustup target list --installed
qemu-system-arm --version
```

`tools/scripts/setup.sh` 应把上述内容固化，并由 `cargo xtask setup` 调用（幂等，可重复执行）。

## 6.2 工具链固定

`rust-toolchain.toml`：

```toml
[toolchain]
channel    = "1.98.1"
components = ["rustfmt", "clippy", "llvm-tools"]
targets    = [
    "thumbv7m-none-eabi",
    "thumbv7em-none-eabihf",
    "riscv32imac-unknown-none-elf",
]
profile    = "minimal"
```

**固定具体版本号而非 `stable`**：OS 项目的构建必须可复现。升级工具链是一个有意识的、经过测试的动作，不是每天自动发生的事。

## 6.3 workspace 与多 target 的冲突

这是**多 crate + 嵌入式最常见的坑**，必须在 M0 就解决：

**问题**：`cargo build`（不带 `--target`）会尝试为 **host x86_64** 编译所有 workspace 成员。而 `echo-arch-cortex-m` 含内联汇编、`echo-bsp-*` 依赖特定架构的链接脚本——在 host 上**必然编译失败**。

**反直觉的是**：`.cargo/config.toml` 里设置 `[build] target = "..."` 能让 `cargo build` 工作，但会让 `cargo test`（需要 host）也去用交叉 target，导致**内核单元测试根本跑不了**——恰好摧毁了 ADR-0002 的最大收益。

**解决方案（三层）**：

### 第 1 层：内核逻辑保持 host 可编译

**这是最重要的一条，且是架构约束而非构建技巧。**

`echo-sched` / `echo-sync` / `echo-ipc` / `echo-mm` / `echo-time` / `echo-util` **只依赖 `echo-arch-api`（trait 定义）**，不含任何 inline asm 或架构特定代码 → 它们**天然可以在 x86_64 host 上编译并通过 `cargo test` 运行**，配合 `echo-arch-mock` 提供 `Arch` 实现。

```bash
# 这些能在 host 上跑，因为它们不知道自己是 ARM 还是 x86
cargo test -p echo-util -p echo-sched -p echo-sync -p echo-ipc -p echo-mm
```

### 第 2 层：`default-members` 排除架构相关 crate

```toml
# 根 Cargo.toml
[workspace]
default-members = [
    "xtask",
    "crates/core/*",
    "crates/kernel/*",     # 只依赖 arch-api，host 可编译
    "crates/api/*",
]
# arch/ services/ bsp/ examples/ 不在默认成员中 → 裸 `cargo build`/`cargo test` 不会碰它们
```

成员列表与默认成员列表不同，这样：

- `cargo test` → 只跑 host 可编译的部分 ✅
- `cargo xtask build --bsp ...` → 显式带 `--target` 构建特定 BSP 及其依赖 ✅

### 第 3 层：`xtask` 统一入口

`xtask` 负责拼接正确的 `--target` / `--features` / `RUSTFLAGS`，并调用 `cargo`。**开发者不需要记住这些**。

```bash
cargo xtask build --bsp qemu-cortex-m --example 02-preemptive-scheduling
```

## 6.4 `.cargo/config.toml`

```toml
# 注意：不设置 [build] target —— 保持 host 为默认，让 cargo test 可用

[target.thumbv7m-none-eabi]
rustflags = [
    "-C", "link-arg=-Tlink.x",
    "-C", "linker=rust-lld",
]
runner = "qemu-system-arm -cpu cortex-m3 -machine mps2-an385 -nographic -semihosting-config enable=on,target=native -kernel"

[target.thumbv7em-none-eabihf]
rustflags = ["-C", "link-arg=-Tlink.x", "-C", "linker=rust-lld"]
runner = "probe-rs run --chip STM32F401RETx"

[target.riscv32imac-unknown-none-elf]
rustflags = ["-C", "link-arg=-Tlink.x", "-C", "linker=rust-lld"]
runner = "qemu-system-riscv32 -machine virt -nographic -bios none -kernel"

[alias]
xtask = "run --package xtask --"
```

### 链接脚本的可见性

`link.x` 由 `echo-rt` 提供。使用它的 BSP 需要在 `build.rs` 中：

```rust
// bsp/echo-bsp-qemu-cortex-m/build.rs
fn main() {
    // 1. 让链接器能找到 echo-rt 提供的 link.x
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::write(out.join("memory.x"), include_bytes!("memory.x")).unwrap();
    println!("cargo:rustc-link-search={}", out.display());

    // 2. 关键：改动链接脚本或 memory.x 时必须触发重新链接
    println!("cargo:rerun-if-changed=memory.x");
    println!("cargo:rerun-if-changed=build.rs");
}
```

**`rerun-if-changed` 漏写是嵌入式项目的经典陷阱**：改了 `memory.x`（比如调整内存布局）但 cargo 认为没变化 → 用旧的链接脚本链接 → 诡异的运行时错误。

## 6.5 寄存器代码生成流水线（L1 特有）

[ADR-0015](04-design-decisions.md#adr-0015--寄存器定义通过自研-svdgen-代码生成) 的落地流程。

### 数据流

```
tools/vendor-svd/STM32F401.svd        厂商发布（公开标准，非软件依赖）
              │
              ▼  cargo run -p svdgen -- --input ... --output ...
   crates/regs/echo-stm32f4-regs/src/  ★ 生成产物，检查进版本库
              │
              ▼
        echo-bsp-nucleo-f401re  ──►  驱动 / 应用
```

### 生成产物形态

```rust
// crates/regs/echo-stm32f4-regs/src/usart.rs  （由 svdgen 生成，请勿手工编辑）
#[repr(C)]
pub struct UsartRegisterBlock {
    pub sr:   Reg<Sr,   RW>,     // 状态寄存器
    pub dr:   Reg<Dr,   RW>,     // 数据寄存器
    pub brr:  Reg<Brr,  RW>,     // 波特率寄存器
    pub cr1:  Reg<Cr1,  RW>,     // 控制寄存器 1
    // ...
}

// echo-vcell 提供的寄存器封装：偏移 + 位域访问器
impl Sr {
    /// Bit 5 — Read data register not empty
    pub fn rxne(&self) -> bool { self.bits & (1 << 5) != 0 }
    pub fn set_rxne(&mut self) { self.bits |= 1 << 5 }
}

// 中断号枚举（从 SVD 的 <interrupt> 节点生成）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Interrupt {
    Usart2 = 38,
    // ...
}
```

### 为什么生成产物要 check in 版本库

| 理由 | 说明 |
| --- | --- |
| **可审计** | 任何人都能读到实际运行的代码，无需先跑一遍生成器 |
| **可修正** | 厂商 SVD 确实有错（位域名、偏移、`derivedFrom` 继承错误），需手工打补丁 |
| **构建简单** | 不需要在构建时跑代码生成（避免 build.rs 里跑 cargo 的鸡生蛋问题） |
| **diff 可见** | SVD 升级带来的变化是一次可 review 的提交 |

**代价**：存在"生成产物与 SVD 不同步"的风险 → 由 CI 守卫（见下）。

### CI 守卫：生成产物一致性

```bash
# CI 中执行：重新生成一遍，检查是否产生 diff
cargo run -p svdgen -- --input tools/vendor-svd/STM32F401.svd \
                       --output crates/regs/echo-stm32f4-regs/src
git diff --exit-code crates/regs/ \
  || { echo "❌ 寄存器 crate 与 SVD 不同步。请运行 tools/scripts/gen-regs.sh"; exit 1; }
```

**例外处理**：若需要手工修正生成产物（SVD 有错），修正必须写进 `svdgen` 的**补丁表**（`patches/` 目录下的声明式修正），而不是直接改生成文件 —— 否则下次重新生成会被冲掉。

### `svdgen` 自身的测试

生成器是**信任根**：它错了，所有芯片的寄存器定义都错。因此必须有独立的测试：

| 测试类型 | 内容 |
| --- | --- |
| 解析测试 | 用手写的微型 SVD 夹具覆盖：寄存器数组、`derivedFrom` 继承、重复位域、保留位、无位域的寄存器 |
| 校验测试 | 地址重叠、对齐违规、命名冲突应被检出并报错 |
| 快照测试 | 对完整 SVD 生成的结果做快照比对，防止无意的生成逻辑变更 |
| 回环测试 | 生成的代码必须能编译（`cargo check -p echo-<chip>-regs`） |

### 命令

```bash
cargo xtask gen-regs --chip stm32f401      # 重新生成指定芯片的寄存器 crate
cargo xtask gen-regs --chip all            # 全部重新生成
cargo xtask gen-regs --check               # CI 用：只校验一致性，不写文件
```

## 6.6 测试金字塔

| 层级 | 环境 | 测什么 | 工具 | 何时跑 |
| --- | --- | --- | --- | --- |
| **L1 · 单元测试** | x86 host (`std`) | `echo-util` 数据结构、纯逻辑函数 | `cargo test` | 每次提交 |
| **L2 · 内核逻辑测试** | x86 host | 调度器、同步、IPC、内存池的**语义正确性** | `cargo test` + `echo-arch-mock` | 每次提交 |
| **L3 · QEMU 集成** | `qemu-system-arm` / `-riscv32` | 真实上下文切换、中断、驱动、端到端 | `echo-testkit` + semihosting | 每次提交 |
| **L4 · 硬件在环** | STM32F4 真机 | 真实外设、时序、功耗、长时间稳定性 | `probe-rs` + 自定义 runner | 每日/发布前 |
| **L5 · 实时性基准** | QEMU（趋势）+ 真机（绝对值） | 中断延迟、切换时间、临界区窗口 | `echo-bench` + DWT/GPIO | 每次 PR（趋势回归） |

### L2 是其价值最大的一层

`echo-arch-mock` 实现 `Arch`，提供：

```rust
pub struct MockArch;

// 可控制的"硬件"
static TIME: AtomicU64 = AtomicU64::new(0);
static INTERRUPT_STATE: AtomicU32 = AtomicU32::new(0);

impl Arch for MockArch {
    fn now_ticks() -> u64 { TIME.load(Ordering::Relaxed) }

    fn switch_context(_from: *mut Ctx, _to: *const Ctx) {
        // 不真的切栈！只记录调用，让测试可以在单线程里验证调度决策
        SWITCH_LOG.lock().push(current_task_id());
    }

    fn disable_interrupts() -> State { /* 模拟 */ }
    // ...
}

// 现在可以在 x86 上测试调度器的全部逻辑：
#[test]
fn higher_priority_task_preempts() {
    let mut kernel = Kernel::<MockArch>::new();
    let lo = kernel.spawn(Task::new(priority = 1, ...));
    let hi = kernel.spawn(Task::new(priority = 5, ...));
    kernel.tick();
    assert_eq!(kernel.current(), hi);
}

#[test]
fn priority_inheritance_bounds_inversion() {
    // 复现经典优先级反转场景，断言阻塞时长 ≤ 临界区长度
}
```

**可测的内容**：优先级选择、同优先级轮转、阻塞/唤醒竞争、超时处理、PIP 提升与恢复、队列满/空行为、内存池耗尽。

**不可测的内容**（必须上 QEMU/真机）：真实的栈切换、寄存器保存正确性、中断延迟、MPU 行为。

### L3 的测试框架

`echo-testkit` 提供基于 semihosting 的断言，QEMU 输出被 host 捕获并解析：

```rust
// 测试代码（跑在 QEMU 内）
echo_testkit::test!(scheduler_preemption, {
    let log = Arc::new(...);
    spawn_task(priority = 1, || log.push("lo"));
    spawn_task(priority = 5, || log.push("hi"));
    sleep_ms(10);
    assert_eq!(log, ["hi", "lo"]);           // 失败 → semihosting 输出 FAILED + 原因
});

// host 侧
#[test]
fn qemu_integration() {
    let out = run_qemu("echo-bsp-qemu-cortex-m");
    assert!(out.contains("PASS: 12"), "{}", out);
}
```

在 L1 策略下，`defmt-test` / `probe-rs test` 等生态工具不可用（它们依赖 `defmt` crate），因此 `echo-testkit` **必须自研**。这反而简单：semihosting 的接口很窄，实现约数百行。

## 6.7 静态检查清单（CI）

```bash
cargo fmt --all -- --check                     # 格式
cargo clippy --all-targets -- -D warnings      # lint（零容忍）
cargo xtask lint-deps                          # ★ 架构依赖规则（R1/R2/R5/R7）
cargo deny check                               # 守卫（黑名单 + 重复依赖）
cargo geiger --output-format Json              # unsafe 统计，与基线对比，只允许下降
cargo udeps                                    # 未使用的依赖（需 nightly，可选）
```

**注意 `lint-deps` 现在是最重要的一项**：它同时守卫架构分层（R1/R2）和零外部依赖（R7）。在 L1 策略下，一次"顺手加个 crate"就能让整个策略失效，而这条规则是唯一的自动防线。

**`cargo geiger` 的用法**：把当前 unsafe 行数作为**基线**提交，CI 检查"不得增加"。这把 G2（内存安全）从口号变成可执行的约束。

**`cargo deny` 在 L1 下的角色变了**：没有第三方依赖，就没有许可证审计与安全公告跟踪的需求。`deny.toml` 退化为**守卫配置**：

```toml
# deny.toml —— 主要作用是守卫，而非合规
[bans]
# ★ 关键：禁止任何非 path 依赖进入镜像 crate
deny = [
    { name = "cortex-m" },
    { name = "embedded-hal" },
    { name = "defmt" },
    { name = "heapless" },
    { name = "smoltcp" },
    { name = "portable-atomic" },
    # ... 已知"诱惑源"清单
]
```

真正的强制由 `cargo xtask lint-deps`（规则 R7）承担 —— 它检查的是**通用规则**（source 必须为 `null`），而不是一张黑名单。`deny.toml` 的 `bans` 只是给出一条更友好的错误信息。

> 这体现了 L1 的一个隐性收益：**合规工作量归零**。没有 SBOM 要维护，没有许可证冲突要分析，没有 CVE 要跟踪。

## 6.8 Miri

`echo-util` / `echo-mm` 的 `unsafe` 代码**必须**通过 Miri：

```bash
cargo +nightly miri test -p echo-util -p echo-mm   # Miri 需要 nightly（仅测试用，不影响产物）
```

> 注意：ADR-0010 说的是**产物只依赖 stable**，Miri 作为开发期工具使用 nightly 是允许的——两者不矛盾。CI 中的 Miri job 与产物构建 job 分离。

**Miri 的限制**：
- 不支持 inline asm（所以 `echo-arch-*` 无法用 Miri 测）
- 不支持部分原子操作与内存映射 I/O
- 因此 Miri 的覆盖面**恰好**是 `echo-util` / `echo-mm` 这些"纯 Rust + unsafe 指针操作"的 crate —— 而这正是最需要它的地方。

## 6.9 实时性度量

### 中断延迟

```
方法 A（真机，推荐）：
  用一个定时器周期性触发中断
  在 ISR 第一行翻转 GPIO（或读 DWT->CYCCNT）
  用示波器/逻辑分析仪测"触发信号 → GPIO 翻转"的时延
  → 采集数万次，统计 P50/P99/max

方法 B（QEMU）：
  读 DWT->CYCCNT 计算周期数
  → QEMU 的时序是模拟的，绝对值无意义，但可做版本间趋势对比
```

### 上下文切换时间

```
在 PendSV 首尾读 DWT->CYCCNT，差值即为切换耗时
需要扣除 PendSV 自身开销 → 用两个任务互相唤醒的方式测量
```

### 内核临界区窗口

```
关键：这是实时性的隐性杀手，且容易随时间劣化。

方法：
  1. 编译期：xtask 分析 echo-critical 的所有调用点，输出每个临界区的行数/代码大小
  2. 运行时：在 CriticalSection::enter/exit 中记录时间戳，维护"历史最长窗口"，
     通过 shell 命令或自研 trace 输出

CI 守卫：最长临界区窗口不得比基线增加超过 10%
```

### 建议的验收阈值

| 指标 | Cortex-M4 @ 100 MHz | 说明 |
| --- | --- | --- |
| 中断延迟（P99） | < 1 µs | 含硬件异常进入 + 压栈 + ISR 入口 |
| 上下文切换 | < 2 µs | 含 PendSV 入口 + 保存 + 恢复 + 退出 |
| 内核临界区最长窗口 | < 500 ns | 超出需在 PR 中说明理由 |
| `Mutex::lock` 快路径（无竞争） | < 200 ns | 不触发调度 |
| `Pool::alloc` 最坏 | < 100 ns | 固定块分配，必须是常数时间 |

**这些数字是设计目标，不是承诺。** 在 M2 建立基准后会根据实测校准。

### 基准回归

```bash
cargo xtask bench --bsp nucleo-f401re --json > bench.json
# CI 对比上一版本，任一指标劣化 > 10% → 告警
```

`echo-bench` 的输出格式应稳定（JSON + 版本号），便于跨版本对比与画趋势图。

## 6.10 构建产物大小报告

```bash
cargo xtask size --bsp nucleo-f401re
```

输出（`cargo size` / `cargo bloat` 封装）：

```
Section         Size      Description
.text          6216 B    Code
.rodata        1024 B    Read-only data
.data           144 B    Initialized data (RAM)
.bss           3920 B    Zero-initialized (RAM)
------------------------------------------
Flash total    7240 B    / 512 KB  (1.4%)
RAM   total    4064 B    /  96 KB  (4.1%)

Top 10 largest symbols:
  1240 B  echo_sched::scheduler::pick_next
   892 B  core::fmt::float           ← 可疑：no_std 下不该有浮点格式化
   ...
```

**用途**：G3（静态可裁剪）的验收依据。每次发布记录，劣化即审查。`core::fmt` 的意外引入是嵌入式常见的体积膨胀源，需要专门盯。

## 6.11 调试支持

| 手段 | 用途 |
| --- | --- |
| `echo-trace`（自研） | 日志。首版为 UART 文本；结构化日志（符号表 + host 解码）推迟 |
| `echo-shell` | 运行时查询：任务状态、栈水位、定时器、互斥量持有者 |
| `probe-rs` + VSCode/`gdb-multiarch` | 断点、变量、栈回溯 |
| `panic` handler | panic 时输出任务名、PC、LR、栈回溯，写 Flash 黑匣子后复位 |
| DWT 性能计数器 | 周期级测量（指令数、异常数、CPI） |
| MemManage handler | MPU 违规时报告出错任务、违规地址、可见的 region 配置 |

**panic handler 是 RTOS 的关键可观测性设施**。设计目标：现场可诊断性——设备在现场死机后，能通过日志/黑匣子还原原因，而非只能复现。
