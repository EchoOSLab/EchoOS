# 19 · 时间域与 bench 直方图（W8）

**状态**：✅ host 逻辑完成（76 tests）；⚠️ 真机时序仍待硬件门

> 本文是 `echo-time`、`echo-bench` 与 DWT 回绕处理的实现契约。
> 相关：[11-arch-and-sched](11-arch-and-sched.md)、[17-h7-bringup](17-h7-bringup.md)。

## 1. `echo-time`

### 1.1 API

```rust
pub struct DelayQueue<const N: usize>;

impl<const N: usize> DelayQueue<N> {
    pub const fn new() -> Self;
    pub fn insert(&mut self, deadline_ns: u64, id: u16) -> Result<(), TimeError>;
    pub fn pop_due(&mut self, now_ns: u64) -> Option<u16>;
    pub fn next_deadline(&self) -> Option<u64>;
    pub fn remove(&mut self, id: u16) -> bool;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn clear(&mut self);
}

pub fn deadline_after(now_ns: u64, delta_ns: u64) -> u64;
pub fn is_due(now_ns: u64, deadline_ns: u64) -> bool;
```

### 1.2 语义

- 固定容量、**按 deadline 有序**、零分配；
- 相同 deadline 按插入顺序稳定排序；
- `pop_due` 只在 `now >= deadline` 时返回，边界包含；
- `deadline_after` 饱和加法，避免溢出/回绕；
- 与 `Arch` 解耦：时钟由 Mock/DWT 提供，队列只做排序与到期判断。

### 1.3 测试（9 个）

`cargo test -p echo-time`：插入排序、同 deadline 稳定、未到期、重复 id、容量、remove、饱和、边界、clear。

## 2. DWT 回绕（W8 补）

32 位 `CYCCNT` 在 480 MHz 下约 **8.9 s** 回绕。v0 直接 `cycles_to_ns` 会倒退；W8 引入 epoch：

```rust
pub fn wrap_period_ns(cpu_hz: u32) -> u64;
pub fn extend_monotonic(last_cycles: u32, epoch_ns: u64, cycles: u32, cpu_hz: u32)
    -> (u32, u64);
```

- 若 `cycles < last_cycles` → 判定一次回绕，`epoch_ns += wrap_period_ns`；
- `now_ns = epoch_ns + cycles_to_ns(cycles)`，跨回绕保持单调；
- 480 MHz 下 `wrap_period_ns = 8_947_848_533 ns`（有测试断言）。

**边界**：如果两次读取之间跨过**多个**回绕周期（>8.9 s 不调用），无法恢复；调度器应按帧调用 `now_ns`，不会触发。

## 3. `echo-bench`

### 3.1 API

```rust
pub struct Histogram<const N: usize>;

impl<const N: usize> Histogram<N> {
    pub const fn new() -> Self;
    pub fn record(&mut self, value: u64) -> bool; // 满返回 false
    pub fn summary(&mut self) -> Option<Summary>;
    pub fn write_json<W: Write>(&mut self, out: &mut W) -> fmt::Result;
    pub fn write_csv<W: Write>(&mut self, out: &mut W) -> fmt::Result;
}

pub struct Summary {
    pub samples: usize,
    pub min: u64,
    pub p50: u64,
    pub p99: u64,
    pub p999: u64,
    pub max: u64,
    pub mean: u64,
}
```

### 3.2 百分位算法（整数，无浮点）

```
index = ceil(percent × len / 100) - 1   // 再 clamp 到 [0, len-1]
```

- P50/P99 用百分位；P99.9 用 `permille(999)`；
- 无浮点、无分配，适合 `no_std` 与 MCU；
- JSON：`{"samples":...,"min":...,"p50":...,"p99":...,"p999":...,"max":...,"mean":...}`
- CSV：表头 + 一行，便于 CI 回归对比。

### 3.3 测试（6 个）

`cargo test -p echo-bench`：1..=1000 的摘要（p50=500、p99=990、p999=999、mean=500）、空、满、单样本、JSON 形状、CSV 形状。

## 4. 明确没有的东西

- ❌ **真机抖动数据**：没有 H7 板卡/示波器，`echo-bench` 目前只有 host 数学测试；
- ❌ Tickless 空闲、SysTick 时基、`sleep_until` 与调度器阻塞的集成；
- ❌ 每帧调度开销的实测；
- ❌ JSON 之外的报告生成（PDF/图表）。

**W9 不会用 host 数字冒充 H7 数据。**

## 5. 决策

| # | 决策 | 理由 |
| --- | --- | --- |
| D-030 | 百分位用整数 `div_ceil`，不引浮点 | no_std/MCU 友好；边界可精确断言 |
| D-031 | `Histogram<N>` 固定容量，满时拒绝而非覆盖 | 测量语义明确；不静默丢样本 |
| D-032 | DWT 回绕用 `epoch_ns` 累加 | 跨回绕单调；避免每次读都做宽除法补偿 |
| D-033 | 时间队列与 `Arch` 解耦 | host 可测；时钟来源可替换 |

## 6. 文件

| 文件 | 内容 |
| --- | --- |
| `crates/kernel/echo-time/src/lib.rs` | `DelayQueue` + deadline 辅助 |
| `crates/kernel/echo-time/tests/delay_queue.rs` | 9 个测试 |
| `crates/test/echo-bench/src/lib.rs` | `Histogram` + JSON/CSV |
| `crates/test/echo-bench/tests/histogram.rs` | 6 个测试 |
| `crates/arch/echo-arch-cortex-m/src/lib.rs` | DWT 回绕 epoch |
| `crates/arch/echo-arch-cortex-m/tests/clock_math.rs` | 8 个测试（含回绕） |

## 7. 下一步（W9）

W9 是 **H7 真机证据**，当前硬件门未过：

1. 生成 `docs/20-h7-evidence.md`：实验设计、测量步骤、报告模板、验收阈值；
2. 明确标记"无数据"，不写通过；
3. 硬件到位后：DWT 直方图 + 逻辑分析仪 GPIO 翻转 + 72h 稳定性；
4. W10 先做 host 可完成的部分（PAL 冻结 + 第二后端 conformance）。
