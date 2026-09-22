# 07 · 里程碑规划

## 7.1 规划原则

1. **每个里程碑都有可演示的产出。** 不接受"完成了 80% 的调度器"这种状态——要么能跑，要么没完成。
2. **M0–M2 是必须最先打通的最小闭环。** 在没有"QEMU 上跑起两个抢占任务"之前，不做任何服务层。
3. **测试与基准与功能同步建设，不留到最后。** L2 测试层（host 上的内核逻辑测试）从 M0 就要建立，否则 ADR-0002 的架构收益会落空。
4. **工期是相对量级，不是承诺。** 标注 S（1 周内）/ M（1–3 周）/ L（1–2 月）/ XL（2 月+），按单人全职估算。团队规模变化时按比例调整，但**里程碑顺序不变**。

## 7.2 里程碑总览

```
M0 ──► M1 ──► M2 ──► M3 ──► M4 ──► M5 ──► M6 ──► M7 ──► M8 ──► M9
骨架   启动   调度   时间   同步   内存   驱动   服务   用户态  网络
 M      L      L      M      L      L      L      XL     XL     XL+
       │      │      │      │      │      │
       │      └──────┴──────┘      │      └── 内核 1.0 候选
       │       内核核心可用          │
       └── 可交付的第一个演示         └── 可移植性验证点
                                             （M8/M9 均为可选，
                                               见各自的"重要判断"）
```

> **规模已按 L1 策略（[ADR-0014](04-design-decisions.md#adr-0014--零外部-crate-依赖l1-完全自研)）调整**：M1 由 S 上调至 L（寄存器层入场费），M6 由 L 维持但工作内容变化（自研 HAL + 自研驱动）。

---

## M0 · 骨架就位

**规模**：M（因 L1 决策从 S 上调——增加了 `svdgen` 骨架与 `echo-vcell`）｜ **前置**：无

**目标**：工程结构、工具链、CI、测试基础设施全部就位，**并验证 `svdgen` 可行**。不写任何内核逻辑。

**交付物**：
- [ ] 工具链补齐（`rustup target add` × 4、QEMU、`cargo-deny`/`geiger`/`binutils`）
- [ ] workspace 骨架：根 `Cargo.toml`（含 `default-members`）、`rust-toolchain.toml`、`.cargo/config.toml`、`rustfmt.toml`、`deny.toml`
- [ ] 从当前单 crate 迁移：删除 `src/main.rs` 与 `[package] echoos`，改为虚拟 workspace 清单
- [ ] `echo-util` 骨架 + 至少 `bitmap.rs` 与 `intrusive_list.rs` 实现（带单元测试）
- [ ] ★ `echo-vcell`：寄存器内部可变性封装（`echo-util` 与寄存器层的共同前置）
- [ ] ★ `tools/svdgen` 骨架：能解析一个 SVD 并生成一个外设（**验证 [A7](08-risks-and-open-questions.md#84-需要重点验证的假设) 的最小闭环**）
- [ ] `echo-arch-api`：`trait Arch` 定义（可先不定稿）
- [ ] `echo-arch-mock`：`MockArch` 最小实现
- [ ] `xtask`：`setup` / `build` / `test --layer host` / `lint-deps` 四个子命令
- [ ] GitHub Actions：`fmt` + `clippy` + `test(host)` + `lint-deps` + `size`
- [ ] `tools/scripts/setup.sh`

**验收标准**：
1. `cargo xtask setup` 幂等执行成功，环境检查全绿
2. `cargo test` 在 host 上通过（跑 `echo-util` 的单元测试）
3. `cargo xtask lint-deps` 能检出**两类**故意注入的违规：跨层依赖（R1）**与外部 crate 依赖（R7）**
4. **★ `svdgen` 能用目标芯片的真实 SVD 生成一个外设的寄存器代码，且该代码通过 `cargo check`** —— 这验证了 [假设 A7](08-risks-and-open-questions.md#84-需要重点验证的假设)，是整个 L1 策略的可行性闸门
5. CI 全绿

**风险**：多 crate + `default-members` 的配置容易反复试错。**先在一个最小 workspace（3 个 crate）上验证构建矩阵，再铺开。**

---

## M1 · 最小可运行内核

**规模**：**L**（因 [ADR-0014](04-design-decisions.md#adr-0014--零外部-crate-依赖l1-完全自研) 的 L1 决策从 M 上调）｜ **前置**：M0

**目标**：QEMU 上启动 EchoOS，串口输出启动日志，进入 idle。**这是"它活了"的时刻。**

**交付物（★ = L1 特有，在 L2 方案中不存在）**：
- [ ] ★ `tools/svdgen`：SVD → Rust 代码生成器（XML 解析 + 建模 + 代码生成 + 夹具测试）
- [ ] ★ `echo-stm32f4-regs`：svdgen 的生成产物（首个验证目标）
- [ ] ★ `echo-cortex-m-regs`：NVIC / SCB / SysTick / MPU / DWT（手写，规模小且稳定）
- [ ] ★ `echo-atomic`：原子抽象（M3+ 原生 CAS）
- [ ] `echo-vcell`：在 M0 骨架基础上补全（生成代码依赖它）
- [ ] `echo-arch-cortex-m`：Reset 流程、向量表、`link.x`、`memory.x` 模板
- [ ] `echo-rt`：`.data`/`.bss` 初始化、panic handler、`#[echo::main]` 宏
- [ ] `echo-trace`：串口输出后端（自研，无 `defmt`）
- [ ] `echo-bsp-qemu-cortex-m`：QEMU mps2-an385 的 BSP
- [ ] `echo-time`：只读时基（`Instant::now()`），无定时器
- [ ] 启动流程骨架（[02 §2.5.4](02-architecture.md#254-启动流程) 的步骤 1–7，任务创建留待 M2）

**验收标准**：
1. `cargo xtask run --bsp qemu-cortex-m` 在 QEMU 中输出：
   ```
   [INFO ] EchoOS v0.1.0 booting...
   [INFO ] arch: cortex-m3 @ qemu-mps2-an385
   [INFO ] clock: 25 MHz, tick source: systick
   [INFO ] kernel: initialized, entering idle
   ```
2. 进入永久 idle 循环（`wfi`），无异常
3. `cargo xtask size` 报告 Flash < 8 KB
4. 故意触发 panic 时，输出任务名、PC/LR、栈回溯

**关于自举顺序**：在 [ADR-0014](04-design-decisions.md#adr-0014--零外部-crate-依赖l1-完全自研) 的 L1 策略下，没有 `cortex-m-rt` 可以"先借用一下"——从第一天起就是自研。这消除了替换风险，但**要求 M1 一开始就投入 `svdgen` 与 `echo-cortex-m-regs`**（见 ADR-0015）。

**M1 内部建议顺序**：

```
1. svdgen 骨架 + 微型 SVD 夹具测试        ← 先让生成器能跑通
2. 生成 STM32F401 寄存器 crate（验证生成器）
3. echo-cortex-m-regs（NVIC/SCB/SysTick 手写，规模小）
4. echo-vcell + echo-atomic（被上面依赖）
5. link.x + 向量表 + Reset 流程
6. echo-trace + UART 输出 → 看到第一个字符
```

第 1–4 步是 L1 特有的前置工作，**在 L2 方案中不存在**。这是 L1 的主要前期成本。

---

## M2 · 调度器与上下文切换 ★ 关键里程碑

**规模**：L ｜ **前置**：M1

**目标**：多任务抢占式调度在 QEMU 上正确运行。**这是内核从"能启动"变成"是操作系统"的分界点。**

**交付物**：
- [ ] `echo-arch-cortex-m`：`PendSV` / `SVC` / `SysTick` 的 `naked_asm!` 实现、MSP/PSP 双栈、栈帧布局
- [ ] `echo-arch-mock`：完整的 `switch_context` 模拟（可记录调用序列）
- [ ] `echo-macros`：`#[echo::task]` —— 生成静态 TCB + 栈 + 注册到 `.echo_tasks` 段
- [ ] `echo-sched`：就绪位图 + 队列、`O(1)` 最高优先级查找、同优先级 RR
- [ ] `echo-critical`：`BASEPRI` RAII guard
- [ ] `echo-kernel`：`Kernel<A: Arch>` 门面 + 首次调度入口
- [ ] idle 任务
- [ ] **L2 测试套件**：不少于 15 个 host 上的调度语义测试
- [ ] **`echo-bench`**：上下文切换时间基准

**验收标准**：
1. **功能**：`examples/02-preemptive-scheduling` 在 QEMU 上产生确定性输出——3 个不同优先级任务按预期抢占顺序打印，重复运行结果一致
2. **语义**（host 测试，不依赖 QEMU）：
   - 高优先级就绪时立即抢占
   - 同优先级任务按 RR 轮转
   - `idle` 仅在无其它就绪任务时运行
   - 优先级变更触发正确的重新调度
3. **实时性**：上下文切换 < 2 µs（QEMU 周期数换算，作为基线）
4. **回归**：`cargo xtask bench` 输出 JSON，CI 与基线对比

**风险**：
- 上下文切换的汇编是**最易出错**的部分，栈帧布局差一个字节就是随机崩溃。**对策**：先用 `echo-arch-mock` 把调度**决策逻辑**测透，再让汇编只负责机械的寄存器搬运；同时用栈魔数（canary）验证栈帧边界。
- 首次任务启动（伪造异常返回帧）是经典难点。**对策**：单独写一个最小测试用例（单任务 + `SVC` 启动），先于多任务场景验证。

---

## M3 · 时间与阻塞

**规模**：M ｜ **前置**：M2

**目标**：任务可以睡眠、延时、被定时唤醒。内核具备完整的"阻塞/唤醒"能力。

**交付物**：
- [ ] `echo-time`：`Instant`/`Duration`、单调时基（DWT 或定时器）、计数器回绕处理
- [ ] 延时链表（按到期时刻有序）
- [ ] `sleep` / `sleep_until` / `yield_now`
- [ ] 软件定时器（单次/周期），回调在定时器服务任务上下文
- [ ] Tickless 空闲（空闲时关 tick，一次性定时器唤醒）
- [ ] `echo-sched`：`blocked` 模块（延时链表 + 事件等待链表）、`wake_reason` 竞态处理

**验收标准**：
1. `examples/06-timer-and-soft-timers`：任务按 10/100/1000 ms 周期运行，实测周期误差 < 1%
2. **超时与事件竞争测试**：构造"事件在超时时恰好到达"的用例，断言 `wake_reason` 正确且节点只被摘除一次（host 测试，用 `MockArch` 精确控制时间推进）
3. Tickless 验证：空闲 1 秒期间的中断次数 < 10（对比 tickful 的 1000 次）
4. 时基回绕测试：用 `MockArch` 模拟 32 位计数器回绕，断言 `Instant` 单调性不受影响

---

## M4 · 同步与 IPC

**规模**：L ｜ **前置**：M3

**目标**：任务间可以安全地共享数据和传递消息。内核具备实用价值。

**交付物**：
- [ ] `echo-sync`：`Mutex`（含 PIP）、`Semaphore`、`BinarySemaphore`、`EventGroup`、`RwLock`
- [ ] `echo-ipc`：`Queue<T, N>`、`Channel<T>`、`Mailbox<T>`、`Signal`
- [ ] ISR 安全 API（`try_*` / `*_from_isr`）与类型区分
- [ ] `echo-mm` 骨架：固定块内存池（`Pool<T, N>` + `PoolBox<T>`），供 IPC 传递所有权
- [ ] ISR 上下文校验：启动时断言 ISR 优先级与临界区屏蔽线的关系
- [ ] **并发测试套件**：优先级反转、死锁检测（运行时 mutex 持有者追踪）

**验收标准**：
1. **优先级反转测试**（核心验收项）：
   ```
   场景：L 持有 Mutex → H 请求（被阻塞）→ M 就绪（不应能抢占 L）
   断言：H 的阻塞时长 ≤ L 的临界区长度 + 常数开销
   对照：关闭 PIP 时，H 被 M 无限期阻塞（测试应能演示这个差异）
   ```
2. `examples/03-sync-mutex-priority-inheritance` 与 `04-ipc-queue` 在 QEMU 上通过
3. 队列语义测试：满/空边界、`try_send` 归还消息、多生产者多消费者压力测试
4. `Pool` 分配最坏时间 < 100 ns（基准）
5. **`cargo geiger`**：`echo-sync` / `echo-ipc` 的 unsafe 行数为 0（`deny(unsafe_code)` 生效）

**风险**：阻塞/唤醒的竞态是**最难测的 bug 来源**。对策：`MockArch` 提供"精确时间控制"和"中断注入点"，让单个测试能构造出所有交错顺序。

---

## M5 · 内存管理与栈保护

**规模**：L ｜ **前置**：M4

**目标**：栈溢出被精确捕获而非静默破坏内存；具备变长分配的确定性方案。

**交付物**：
- [ ] `echo-mm`：TLSF 分配器（`O(1)` 最坏情况）、可选 `#[global_allocator]`
- [ ] 任务栈：MPU 守卫区、水位线填充与查询、canary 检查
- [ ] `echo-arch-cortex-m`：MPU region 编程、`MemManage` 异常处理
- [ ] 静态 MPU 分区（[ADR-0005](04-design-decisions.md#adr-0005--单地址空间--mpu-静态分区)）
- [ ] `echo-shell` 骨架：查询任务、栈水位、内存池用量
- [ ] FPU lazy stacking 优化（若目标有 FPU）

**验收标准**：
1. **栈溢出测试**：构造递归耗尽栈的任务
   - 配置 MPU 守卫 → 任务被终止，输出"task X stack overflow at 0x..."，**系统继续运行**
   - 无 MPU 配置 → canary 检测到并报告
2. 栈水位报告准确：实际使用量报告值 ≥ 真实值
3. TLSF 分配最坏时间有界：压力测试下最坏值 < 最坏值的 3 倍（抖动有界）
4. **内存池耗尽测试**：池耗尽时 `alloc()` 返回 `None`，不 panic、不阻塞
5. Miri 通过：`cargo +nightly miri test -p echo-mm -p echo-util`

---

## M6 · 驱动框架与可移植性验证 ★ 关键里程碑

**规模**：L ｜ **前置**：M5

**目标**：**验证 `Arch` 抽象是否真的可移植**。第二个架构跑通同一套测试。

**交付物**：
- [ ] `echo-driver`：编译期设备树宏、中断绑定、`#[echo::irq]`、上半部/下半部
- [ ] `echo-hal`：**自研** HAL trait 体系（概念对齐 `embedded-hal`，见 [05 §5.6](05-subsystems.md#hal-trait-体系echo-hal)）
- [ ] 各芯片的 `echo-*-regs`：用 `svdgen` 生成（验证"换芯片只换 SVD"这一承诺）
- [ ] 驱动：UART、GPIO、定时器、SPI（全部自研）
- [ ] `echo-arch-riscv`：RV32IMAC 移植（CLINT 定时器、PLIC 中断控制器、机器模式陷出）
- [ ] `echo-bsp-qemu-riscv-virt`
- [ ] `echo-conformance`：跨平台一致性测试套件
- [ ] `echo-bsp-nucleo-f401re`：首个真机 BSP

**验收标准**：
1. **可移植性（核心里程碑）**：`echo-conformance` 套件在 Cortex-M 和 RISC-V 上**全部通过**，且内核 crate 一行未改
   ```
   这直接验证 ADR-0002。如果做不到，说明 Arch trait 抽象泄漏，
   需要回头修改——越早发现代价越小。
   ```
2. `examples/05-driver-uart` 在两个平台上行为一致
3. 一个真实外设（如 I2C 温度传感器）通过 `echo-hal` trait 接入，且驱动代码可在 host 上用 mock 测试
4. ISR 优先级校验生效：故意配置错误优先级 → 启动时 panic 并给出清晰信息
5. 真机验证：Nucleo-F401RE 上跑通 M2–M5 的全部测试

---

## M7 · 服务层

**规模**：XL ｜ **前置**：M6

**目标**：EchoOS 具备完整的系统服务能力，可支撑真实产品原型。

**交付物**：
- [ ] `echo-fs`：日志结构文件系统（掉电一致性、磨损均衡）
- [ ] `echo-ota`：A/B 双分区升级、镜像校验、回滚
- [ ] `echo-shell`：完整版（任务、内存、定时器、互斥量、文件系统诊断）
- [ ] `echo-async`：阻塞原语的 `Future` 适配 + 可选执行器
- [ ] 长时间稳定性测试（7×24 小时运行 + 内存泄漏/碎片检测）
- [ ] 文档：用户指南、移植指南、API 文档

**验收标准**：
1. **掉电一致性**：在随机时刻断电 1000 次，文件系统可恢复且无数据损坏（QEMU 可模拟复位）
2. OTA 升级失败回滚：写入损坏镜像 → 自动回滚到旧版本
3. 7×24 小时运行：栈水位、内存池用量、定时器数量全部稳定（无泄漏）
4. 各服务可独立裁剪：`minimal` 配置下不含任何服务 crate，Flash 占用回到 M2 水平

---

## M8 · 用户态与系统调用（可选/长期）

**规模**：XL ｜ **前置**：M7

**目标**：非特权任务 + `SVC` 系统调用 + 能力模型。

**交付物**：
- [ ] `echo-sys`：系统调用号定义、`SVC` 陷入、参数校验
- [ ] 非特权任务：`CONTROL` 寄存器配置、独立 PSP 栈
- [ ] `UserPtr<T>`：用户指针校验（含 TOCTOU 防护）
- [ ] 类型化句柄 + 能力模型
- [ ] 系统调用边界的形式化测试 / fuzzing

**验收标准**：
1. 非特权任务无法直接访问内核内存（MPU 触发 `MemManage`）
2. 恶意系统调用参数（越界句柄、非法指针）被拒绝且不崩溃
3. 系统调用开销 < 1 µs
4. 能力模型：任务只能访问被显式授予的资源

**重要判断**：这一阶段**风险最高、收益最不确定**。系统调用边界是内核中唯一"Rust 类型系统完全不提供保护"的地方（[05 §5.7](05-subsystems.md#57-系统调用与用户态后期m8)）。**建议推迟到 M7 完成后重新评估需求**——若目标场景（工业控制 / 汽车 ECU）不需要多租户隔离，可以完全跳过，把精力投入实时性验证与认证材料。

---

## M9 · 网络协议栈（自研，长期）

**规模**：XL+ ｜ **前置**：M7

**背景**：在 [ADR-0014](04-design-decisions.md#adr-0014--零外部-crate-依赖l1-完全自研) 的 L1 约束下，`smoltcp` 不可用，TCP/IP 需自研。**这是一个独立的多年项目**，因此从 M7 中独立出来，并**从简单的链路层起步**而非直接上 TCP/IP。

**建议的推进顺序**（每一步都是可独立交付的里程碑）：

| 阶段 | 内容 | 规模 |
| --- | --- | --- |
| N1 | 链路层抽象 + 以太网驱动 + ARP | M |
| N2 | IPv4 + ICMP（可 ping 通） | M |
| N3 | UDP + 简单的请求/响应应用 | M |
| N4 | CAN / Modbus / 串口协议族（工业场景优先级**高于** TCP/IP） | M |
| N5 | TCP 状态机（最复杂：拥塞控制、重传、窗口管理） | XL |
| N6 | DHCP / DNS / 应用层协议 | L |
| N7 | TLS（需要自研密码学 —— 极高风险） | XL |

**重要判断**：**不要为了"支持网络"而启动 N5–N7**。工业控制与汽车场景中，CAN/Modbus 的价值远高于 TCP/IP，而成本低一个数量级。**除非目标场景明确要求 IP 网络，否则建议止步于 N4。**

**若必须支持 IP 网络**，应重新评估 L1 决策——这是 [ADR-0014 复议条件](04-design-decisions.md#adr-0014--零外部-crate-依赖l1-完全自研)中列出的情形之一。

---

## 7.3 持续进行的跨里程碑工作

| 工作 | 频率 | 说明 |
| --- | --- | --- |
| 实时性基准回归 | 每个 PR | 任一指标劣化 > 10% 需说明理由 |
| unsafe 基线守卫 | 每个 PR | `cargo geiger` 只允许下降 |
| 依赖规则校验 | 每个 PR | `cargo xtask lint-deps` |
| 体积报告 | 每次发布 | Flash/RAM 占用记录与趋势 |
| 文档更新 | 每个 PR | 设计变更先改 `docs/` |
| 示例维护 | 每个里程碑 | 示例是主要的使用文档 |

## 7.4 关键路径与依赖

```
M0 ──► M1 ──► M2 ──► M3 ──► M4 ──► M5 ──► M6 ──► M7 ──► M8
              │                │      │      │
              │                │      │      └─ RISC-V 移植可与 M7 并行启动
              │                │      └─ MPU 可与 M4 并行准备
              │                └─ 内存池（M4 的子集）是 IPC 的前置
              └─ echo-bench + L2 测试套件必须与 M2 同步建立，不能后补
```

**可并行的分支**（团队 > 2 人时）：
- 内核线：M2 → M3 → M4 → M5
- 移植线：M6 的 RISC-V 移植
- BSP 线：真机 BSP（M6）+ 硬件验证
- 测试线：`echo-testkit` / `echo-bench` / `echo-conformance`（与内核线并行，服务内核线）

## 7.5 立即可执行的下一步

1. **确认 [08 Q1](08-risks-and-open-questions.md#q1--首选目标平台是哪个-高优先级)（首选平台）** —— Q2（"从零"边界）已决定为 **L1**，平台决定 M1 的第一块 SVD 用哪个芯片
2. 执行 M0 的工具链补齐（`cargo xtask setup` 的内容）
3. 用 4 个 crate 的最小 workspace（`echo-util` + `echo-vcell` + `echo-arch-api` + `echo-arch-mock`）验证构建矩阵（`default-members` + 交叉 target 是否能共存），再铺开完整结构
4. 编写 `echo-arch-api` 的 `trait Arch` 初稿——**这是所有后续工作的接口契约，值得多花时间**
5. **★ 在最小 workspace 上跑通 `svdgen`**：用目标芯片的真实 SVD，生成一个外设并让代码通过编译
6. 进入 M1

> **第 3 步不可跳过。** 多 crate + 多 target 的构建配置是嵌入式 Rust 项目最容易卡住的地方，先用最小规模验证，比在 35 个 crate 上调试便宜得多。
>
> **第 5 步是 L1 特有的前置验证，应尽早执行。** 若 SVD 生成在目标芯片上不可行，整个 L1 策略的基础就动摇了——这个结论越早得出越好。
>
> **注意 M1 的规模已因 L1 决策从 S 上调为 L。** `svdgen` 与寄存器层是 L1 的入场费，无法绕过也无法推迟——没有寄存器定义就点不亮任何一个外设。好消息是它是**一次性投入**：生成器写对之后，后续每支持一颗新芯片的成本从"数月"降到"换一个 SVD 文件"。
