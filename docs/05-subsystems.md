# 05 · 核心子系统设计

## 5.1 调度器（`echo-sched`）

### 算法

**固定优先级抢占式 + 同优先级时间片轮转（可选）+ 优先级继承**。

### 数据结构

```
就绪状态
┌──────────────────────────────────────────────────────────┐
│  ready_bitmap: u64          ← 位 i = 1 表示优先级 i 有就绪任务 │
│                                                          │
│  ready_queues: [IntrusiveList<Tcb>; MAX_PRIORITIES]      │
│      [0]  ──► idle 任务                                  │
│      [1]  ──► TCB_B ◄──► TCB_D          (同优先级 RR 轮转) │
│      [2]  ──► (空)                                       │
│      ...                                                 │
│      [31] ──► TCB_A  (最高优先级)                         │
└──────────────────────────────────────────────────────────┘

查找最高优先级就绪任务：
    let prio = 63 - ready_bitmap.leading_zeros();   // 单条 CLZ 指令
    let tcb  = ready_queues[prio].pop_front();
```

**复杂度**：`O(1)` 查找 + `O(1)` 出队。无循环、无搜索——这是硬实时的基本要求。

### 子系统组成

| 模块 | 职责 |
| --- | --- |
| `ready` | 就绪队列与位图，`O(1)` 最高优先级查找 |
| `blocked` | 阻塞链表（按唤醒时间排序的延时链表 + 事件等待链表） |
| `context` | 上下文切换的决策逻辑（谁该运行），**不含汇编** |
| `priority` | 优先级变更（PIP 提升/恢复）、优先级合法性校验 |
| `idle` | 空闲任务（最低优先级，进入 `wfi`），可选低功耗钩子 |
| `stats` | 每任务的运行时间、切换次数统计（可选，用于调优） |

### 阻塞与唤醒

```
任务调用阻塞 API（如 mutex.lock()）
    │
    ├─ 立即能满足？ ──► 直接返回（快路径，无调度）
    │
    └─ 不能 ──► 从就绪队列移除
                ├─ 若是定时等待 → 插入 delay_list（按超时时刻有序）
                ├─ 插入该对象的 wait_list
                ├─ 设置 TCB.state = Blocked
                └─ 触发调度（PendSV）
```

**超时与唤醒竞争**：任务可能同时被"事件"和"超时"唤醒。用 `TCB.wake_reason` 标记原因，并在两个链表中各放一个节点（或在同一节点上用标志位）。必须保证**只有一个路径能成功摘除节点**——这是竞态高发区，需重点测试。

### 时间片轮转

仅在**同优先级**任务间轮转，由 tick 中断驱动。可通过 `const ROUND_ROBIN: bool` 编译期关闭（关闭后完全无 tick 驱动的调度决策，延迟更优）。

### Tickless 空闲

```
就绪队列只剩 idle 任务时：
    ├─ 计算下一个定时器到期时刻 Δ
    ├─ 若 Δ > 阈值 → 关闭周期性 tick，设置一次性硬件定时器在 Δ 后触发
    ├─ 进入 wfi（等待中断）
    └─ 定时器中断唤醒 → 重新计算

效果：空闲时中断频率从 1 kHz 降到接近 0，功耗显著下降。
```

**注意**：tickless 下 `now()` 的读取必须基于**单调硬件计数器**（如 `DWT CYCCNT` 或定时器计数），而不是"tick 累加值"，否则休眠期间时间会停滞。

---

## 5.2 内存管理（`echo-mm`）

### 内存布局

```
Flash                                     RAM
┌──────────────────────┐                 ┌──────────────────────┐ 0x2000_0000
│ .vector_table        │                 │ .data                │
│ .text (代码)          │  ◄── 只读执行   ├──────────────────────┤
│ .rodata (常量)        │                 │ .bss                 │
├──────────────────────┤                 ├──────────────────────┤
│ .echo_tasks (任务描述)│  ◄── 静态注册   │ 内核堆（可选）         │
├──────────────────────┤                 ├──────────────────────┤ _task_stacks_start
│ .echo_devices        │                 │ 任务 A 栈 [静态数组]   │
├──────────────────────┤                 │   ├─ 守卫区 (NoAccess)│
│ .echo_isr (中断表)    │                 │ 任务 B 栈             │
└──────────────────────┘                 │   ├─ 守卫区           │
                                          │ 任务 C 栈             │
                                          │   ├─ 守卫区           │
                                          ├──────────────────────┤
                                          │ idle 栈               │
                                          └──────────────────────┘ _stack_end
```

由 `echo-rt` 的 `link.x` 定义 `.echo_*` 段，用于**编译期注册**：`#[echo::task]` 宏把 TCB 和栈放入这些段，启动流程遍历段内容自动创建任务——**无需运行时注册调用**。

### 分配器

| 分配器 | 最坏时间 | 碎片 | 用途 |
| --- | --- | --- | --- |
| **固定块池（`pool.rs`）** | `O(1)` | **无** | 默认推荐：消息、缓冲区、固定大小对象 |
| **TLSF（`tlsf.rs`）** | `O(1)` | 有界 | 变长分配：文件系统缓存、协议栈 |
| `global_alloc`（可选） | 取决于后端 | — | 需要 `alloc` crate 的场景 |

**固定块内存池 API**：

```rust
static MSG_POOL: Pool<Message, 16> = Pool::new();   // 编译期声明，无运行时初始化

let msg: PoolBox<Message> = MSG_POOL.alloc()?;      // O(1)，可能失败（不阻塞）
// ... 使用 ...
// Drop 时自动归还，或显式 MSG_POOL.free(msg)
```

**关键性质**：
- 分配**永不阻塞**，立即返回 `Option`。实时系统中"等待内存"是不可接受的。
- 归还即释放：**无碎片，无合并逻辑，时间恒定**。
- `PoolBox<T>` 在池被 drop 时归还——所有权通过类型系统管理，不会有泄漏或重复释放。

### 栈管理

```rust
#[echo::task(stack = 2048, priority = 3)]
fn sensor_task() -> ! { /* ... */ }
```

生成：
```rust
#[repr(align(32))]                       // MPU region 对齐全
static SENSOR_TASK_STACK: Stack<2048> = Stack::new();
static SENSOR_TASK_TCB: Tcb = Tcb::new(/* ... */);
```

**栈溢出检测（三重）**：

1. **MPU 守卫区**（首选）：栈底放置 32 字节 `NoAccess` region。溢出立即触发 `MemManage` 异常 → 精确捕获，无延迟。
2. **水位线（watermark）**：栈填充魔数 `0xDEAD_BEEF`，运行时扫描未被覆盖的范围 → 可查询"高水位"，用于**调优栈大小**。
3. **Canary**（降级）：栈底放魔数，上下文切换时检查 → 无 MPU 时的兜底。

---

## 5.3 同步原语（`echo-sync`）

### 原语矩阵

| 原语 | 语义 | ISR 可调用 | 优先级继承 | 典型用途 |
| --- | --- | --- | --- | --- |
| `Mutex<T>` | 互斥 + 保护数据 | ❌ | ✅ | 共享资源保护 |
| `Semaphore` | 计数信号量 | 仅 `give` | 可选 | 资源计数、ISR→任务同步 |
| `BinarySemaphore` | 二值信号量 | 仅 `give` | — | 事件通知 |
| `EventGroup` | 多事件位等待/置位 | 仅 `set` | — | 多条件等待 |
| `RwLock<T>` | 读写锁 | ❌ | 可选 | 读多写少（配置表） |
| `Condvar` | 条件变量 | ❌ | — | 配合 Mutex |

### 类型区分 ISR 与任务上下文

**核心设计**：用**不同的类型**表达 ISR 安全，让误用在编译期失败。

```rust
impl<T> Mutex<T> {
    pub fn lock(&self) -> MutexGuard<'_, T> { /* 可阻塞，仅任务上下文 */ }
    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> { /* 非阻塞，ISR 可用 */ }
}

impl Semaphore {
    pub fn acquire(&self) { /* 可阻塞，仅任务上下文 */ }
    pub fn try_acquire(&self) -> bool { /* ISR 可用 */ }
    pub fn release(&self) { /* 非阻塞，ISR 可用 */ }
}
```

进一步：用 `echo-sys` 的标记类型 `TaskContext` / `IsrContext` 作为 API 的隐式参数（通过 trait bound 或零大小标记参数），把"ISR 里调用了阻塞 API"变成编译错误，而非运行时断言。

```rust
// 概念示意
pub fn lock(&self, _ctx: &mut TaskContext) -> MutexGuard<'_, T>;
pub fn try_lock(&self, _ctx: &mut IsrContext) -> Option<MutexGuard<'_, T>>;
```

> 实现细节待定：Rust 没有隐式上下文参数，需要宏或 `#[track_caller]` 辅助。**这是一个待研究项**，若成本过高则退化为运行时断言 + 文档约定。

### 优先级继承的实现

```
高优先级任务 H 请求被低优先级任务 L 持有的 Mutex M：
  1. 检测到 M.owner = L 且 L.priority < H.priority
  2. L.priority = H.priority          ← 临时提升
  3. L 从原就绪队列移到新优先级队列
  4. H 进入 M.wait_list 阻塞
  5. 重新调度 → L 立即运行（不再被中优先级任务 M 抢占）
  6. L 释放 M 时：
     - 从 M.wait_list 唤醒 H
     - 恢复 L 的原优先级
     - 若 L 还被其它任务继承 → 取继承者中的最高优先级
```

**难点**：
- **传递性继承**：L 持有 M1，又阻塞在 M2 上（M2 被更低优先级的 K 持有）→ 继承链需要传播。首版可限制为**非传递**（简化），文档明确说明限制。
- **优先级恢复的正确性**：必须维护"继承栈"或重新计算 `max(基础优先级, 所有等待者的优先级)`。

### 默认无堆下的 IPC 所有权传递

ADR-0008 决定了默认无 `Box`。消息传递需要转移所有权：

```rust
// PoolBox<T> 可安全跨任务转移所有权，池在归还时回收
let msg = MSG_POOL.alloc().ok_or(Error::NoMemory)?;
msg.fill_from(&sensor_reading);

QUEUE.send(msg).await;      // 所有权转移给接收者
```

这提供了 `Box` 级别的所有权语义，但**分配时间有界、无碎片**。

---

## 5.4 时间管理（`echo-time`）

### 时基

```rust
pub struct Instant(u64);       // 微秒级，单调递增
pub struct Duration(u64);

impl Instant {
    pub fn now() -> Self;                        // 读硬件计数器
    pub fn elapsed(&self) -> Duration;
    pub fn checked_add(self, d: Duration) -> Option<Instant>;  // 显式处理溢出
}
```

- **单调性来源**：`DWT CYCCNT`（Cortex-M3+，与 CPU 同频，精度最高）或硬件定时器计数（M0+ 无 DWT）。
- `u64` 微秒 → 溢出周期约 58 万年，实际不会溢出，但仍提供 `checked_*` API。
- **必须处理计数器回绕**（32 位计数器约 42.9 秒 @100 MHz 回绕一次）→ 由 `echo-arch-*` 内部维护高位扩展。

### 软件定时器

```rust
static TIMER: StaticTimer = StaticTimer::new();

TIMER.start(Duration::from_ms(100), TimerMode::Periodic, || {
    // 回调在「定时器服务任务」上下文执行，不在 ISR 中
});
```

**关键设计**：回调在**专用任务**中执行，不在 ISR 中。原因：

1. ISR 中执行用户回调会破坏中断延迟保证。
2. 回调中可能调用阻塞 API（如发送到队列）——这在 ISR 中非法。

代价：定时器回调的触发时刻受该任务优先级影响，有少量调度延迟（通常 < 10 µs）。对精度敏感的场景提供 ISR 直接触发的"快速定时器"变体。

### 延时链表

按到期时刻**有序**插入的链表（或时间轮）。到期检查在 tick / 定时器中断中执行：

```
O(n) 插入（线性扫描）适用于少量定时器（< 32 个）
若定时器数量大 → 时间轮（O(1) 插入），代价是精度受槽宽限制
首版用有序链表，按需优化
```

---

## 5.5 IPC（`echo-ipc`）

| 机制 | 语义 | 数据传递 | 适用 |
| --- | --- | --- | --- |
| `Queue<T, N>` | 有界 FIFO，多生产者多消费者 | 值拷贝或 `PoolBox` 所有权转移 | 通用消息 |
| `Channel<T>` | 单生产者单消费者，零拷贝 | 引用/所有权 | 高吞吐数据流 |
| `Mailbox<T>` | 单槽，覆盖旧值 | 值 | 最新状态共享 |
| `EventGroup` | 位集合等待 | 无 | 多事件同步 |
| `Signal` | 轻量直接通知（类似 FreeRTOS Task Notification） | 无 | ISR→任务快速唤醒 |

**API 形态**（以 Queue 为例）：

```rust
impl<T, const N: usize> Queue<T, N> {
    pub const fn new() -> Self;                    // 编译期构造，无堆

    // 任务上下文：可阻塞
    pub fn send(&self, item: T, timeout: Option<Duration>) -> Result<(), Error>;
    pub fn recv(&self, timeout: Option<Duration>) -> Result<T, Error>;

    // ISR 上下文：非阻塞
    pub fn try_send(&self, item: T) -> Result<(), Error>;
    pub fn try_recv(&self) -> Result<T, Error>;

    pub fn capacity(&self) -> usize;
    pub fn len(&self) -> usize;
}
```

**SendError 语义**：队列满时 `try_send` 返回 `Err(Full(item))`——**归还消息**，调用者决定丢弃还是重试。绝不静默丢弃。

---

## 5.6 驱动模型与设备树（`echo-driver`）

### 编译期设备树

不用运行时 DTB（那是 Zephyr/Linux 的方案，需要解析器和额外 RAM）。改用**宏 + `static`**：

```rust
// BSP 中声明设备
echo::devices! {
    uart0: Uart<Usart2> @ (0x4000_4400, irq = 38, tx = PA2, rx = PA3),
    spi1:  Spi<Spi1>    @ (0x4001_3000, irq = 35),
    i2c1:  I2c<I2c1>    @ (0x4000_5400, irq = 31),
}
```

### 中断绑定

```rust
#[echo::irq(USART2, priority = 6)]
fn uart2_isr(dev: &'static Uart<Usart2>) {
    // 上半部：只做最小工作
    dev.ack_interrupt();
    dev.rx_drain_to_buffer();        // 搬运到环形缓冲
    if dev.has_complete_frame() {
        dev.rx_task.wake();          // 唤醒任务处理
    }
}
```

**上半部/下半部模型**：

```
ISR（上半部）                      任务（下半部）
├─ 清除中断标志        延迟 < 1 µs   ├─ 解析协议
├─ 搬运到环形缓冲                    ├─ 业务逻辑
└─ 唤醒处理任务                      └─ 可阻塞、可耗时
```

**中断优先级必须与内核临界区屏蔽线协同**（见 ADR-0006）。`echo-driver` 在启动时断言：访问内核 API 的 ISR 优先级低于屏蔽线，违反则 `panic`（早期失败，而非偶发竞态）。

### HAL trait 体系（`echo-hal`）

在 [ADR-0014](04-design-decisions.md#adr-0014--零外部-crate-依赖l1-完全自研) 的 L1 策略下，`embedded-hal` 不可用，**必须自研 HAL trait**。设计原则：

1. **概念对齐 `embedded-hal` 1.0**。不追求 API 相同，但 trait 划分（`I2c` / `Spi` / `Digital` / `Delay` / `Serial`）、错误语义、读写分离的思路保持一致。这样将来若有人写移植层，心智负担最小。
2. **区分阻塞与异步**。`embedded-hal` 的阻塞 trait 在 EchoOS 中会**挂起当前任务**而非忙等——这需要 trait 的签名允许内核介入。
3. **原子操作 trait 化**，便于在 host 上用 `echo-arch-mock` 测试驱动逻辑。

```rust
// echo-hal 的 trait 草案（自研，概念对齐 embedded-hal）
pub trait I2c {
    type Error;
    /// 阻塞式读：内部会挂起当前任务直到完成，不忙等
    fn read(&mut self, addr: u8, buf: &mut [u8]) -> Result<(), Self::Error>;
    fn write(&mut self, addr: u8, buf: &[u8]) -> Result<(), Self::Error>;
    fn write_read(&mut self, addr: u8, wr: &[u8], rd: &mut [u8]) -> Result<(), Self::Error>;
}

pub trait Spi {
    type Error;
    fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), Self::Error>;
}

pub trait DigitalPin {
    fn set_high(&mut self);
    fn set_low(&mut self);
    fn is_high(&self) -> bool;
}
```

**代价**：第三方 `embedded-hal` 驱动无法直接使用，需要写适配层。这是 L1 的**长期成本**（见 [01 §1.7](01-vision-and-scope.md#17-依赖边界l1-完全自研)），不是一次性成本。

**缓解**：仓库内提供一个可选的 `echo-hal-compat`（**仅当将来放宽 L1 时**），把 `embedded-hal` 的 trait 适配到 `echo-hal`。当前不实现，但 trait 设计为其留出空间。

---

## 5.7 系统调用与用户态（后期，M8+）

首版任务全部运行在**非特权模式**，通过 `SVC` 陷入内核。这是 Tock/Hubris 都采用的模型。

```rust
// 用户侧
echo_sys::queue_send(&QUEUE, msg)?;      // 生成 SVC 指令

// 内核侧（SVC 处理函数）
fn syscall_handler(num: SyscallNumber, args: [u32; 4]) -> u32 {
    match num {
        SyscallNumber::QueueSend => {
            let q = validate_handle::<Queue<_>>(args[0])?;   // ★ 句柄校验
            let ptr = UserPtr::<T>::new(args[1])?;            // ★ 用户指针校验
            let item = ptr.copy_from_user()?;                 // ★ 边界检查
            q.try_send(item).map(|_| 0).map_err(Into::into)
        }
        // ...
    }
}
```

**关键安全点**：系统调用边界是**唯一**不可信输入来源。Rust 的类型系统在跨特权边界时**不再提供任何保证**——内核必须显式校验：

- **句柄校验**：用户传入的是内核对象索引，不是指针。用类型化句柄 `Handle<Queue>` 而非裸 `u32`，并在内核侧校验句柄有效性。
- **用户指针校验**：`UserPtr<T>` 类型在构造时校验地址落在用户的合法内存区域内，且**在每次解引用时重新校验**（TOCTOU 防护）。
- **能力模型**：任务只能访问被显式授予的资源（类似 Hubris/Tock 的能力传递）。

**这部分风险极高，建议放在 M8 及以后，且优先考虑只做特权模式版本（所有任务特权），把非特权隔离作为后续可选增强。**
