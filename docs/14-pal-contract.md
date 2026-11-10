# 14 · PAL 契约（Micro 5 域，v0）

**状态**：已实现（W3，2026-11-10）｜ **新增测试**：7 个｜ **累计**：31 个 host 测试全绿

> 本文是 `echo-pal-api` 与 `echo-conformance::mock` 的实现契约。
> 背景：DoraOS 需要把"底座能力"抽象成稳定接口；PAL 由**最受限的底座**（EchoOS Micro）定义，
> Linux/Zephyr 只做适配。设计动机见 DoraOS 架构方案 v0.2（外部对话产物）。

## 1. 抽象的是语义，不是 OS 原语

| 不抽象 | 抽象 |
| --- | --- |
| `pthread_create` / 闭包 `spawn` | `TaskApi::declare(StaticTask)`（编译期声明） |
| `malloc` / `mmap` | `MemoryApi`（固定块静态池） |
| `socket` / `epoll` | `ChannelApi`（有界、非阻塞） |
| `printf` / `epoll_wait` | `ObservabilityApi::trace_event` |
| 运行期探测能力 | `Capabilities`（编译期常量） |

这条原则直接来自对 DoraOS PAL 草案的评审：`spawn(cfg, impl FnOnce())`、`bounded(cap)`、`alloc(size)`
都会把 POSIX/动态假设带进 Micro，导致 RTOS 侧无法实现静态保证。v0 全部改成静态语义。

## 2. 五个能力域

```rust
pub trait Platform {
    type Task: TaskApi;
    type Time: TimeApi;
    type Memory: MemoryApi;
    type Channel: ChannelApi;
    type Trace: ObservabilityApi;

    fn capabilities(&self) -> Capabilities;
    fn task(&self) -> &Self::Task;
    fn time(&self) -> &Self::Time;
    fn memory(&self) -> &Self::Memory;
    fn channel(&self) -> &Self::Channel;
    fn trace(&self) -> &Self::Trace;
}
```

| 域 | 方法 | Micro 语义 |
| --- | --- | --- |
| **Task** | `current` / `task_count` / `yield_now` / `declare(StaticTask)` | 任务在启动阶段声明，运行期不创建；容量编译期确定 |
| **Time** | `now_ns` | 单调纳秒；推进由硬件/调用者负责 |
| **Memory** | `pool_capacity` / `pool_used` / `try_alloc(len)` / `free(block)` | 固定块静态池；热路径零分配 |
| **Channel** | `capacity` / `try_send` / `try_recv` | 有界、非阻塞、FIFO |
| **Observability** | `trace_event(id,value)` / `event_count` / `dropped_events` | 固定容量缓冲；满则计数丢弃 |

### 能力矩阵

| 能力 | Micro v0 | 说明 |
| --- | --- | --- |
| `static_tasks` | ✅ | 编译期静态任务 |
| `dynamic_tasks` | ❌ | 运行期不创建任务 |
| `blocking_sleep` | ❌ | 控制域槽位内不允许阻塞 |
| `dynamic_memory` | ❌ | 只有静态池 |
| `channels` | ✅ | 单通道 v0 |
| `trace` | ✅ | 固定容量事件缓冲 |
| `posix_partition` | ❌ | [ADR-0020](05-design-decisions.md#adr-0020--posix-兼容分区)；90 天内恒为 false |

与 DoraOS PAL 10 域的对应：**已实现 5 域**（Task/Time/Memory/Channel/Observability）；
`Transport` / `Storage` / `Device` / `Isolation` / `Loading` 在 v0 标为不支持，后续按消费者加入。

## 3. Mock 后端（`echo-conformance::mock`）

| 常量 | 值 | 含义 |
| --- | --- | --- |
| `MAX_TASKS` | 8 | 静态任务表容量 |
| `TRACE_CAPACITY` | 64 | trace 事件数 |
| `POOL_BLOCK_SIZE` | 16 B | 单块大小 |
| `POOL_BLOCKS` | 16 | 池块数（总 256 B） |
| `CHANNEL_CAPACITY` | 8 槽 | 可用容量 7 |

`MockPlatform` 用 `type Task = Self` 等把五个域实现在同一个结构上，因此 conformance 测试
可以用 `P: Platform` 泛型**只依赖契约**，不依赖具体 mock 类型。

## 4. Conformance 测试（7 个）

```bash
cargo test
```

| 测试 | 证明 |
| --- | --- |
| `mock_platform_reports_micro_capabilities` | 能力矩阵正确（无动态能力、无 POSIX） |
| `generic_contract_is_satisfied` | 通过 `P: Platform` 泛型跑完整五域流程 |
| `task_table_is_bounded` | 静态任务表满时返回 `CapacityExceeded` |
| `channel_is_bounded_and_fifo` | 通道有界、FIFO、满时归还值 |
| `trace_records_and_reports_drops` | trace 记录与丢弃计数 |
| `memory_pool_exhausts_and_reuses` | 池耗尽返回 `None`，释放后可复用 |
| `time_only_advances_explicitly` | 时间只由显式推进改变 |

## 5. 决策

| # | 决策 | 理由 |
| --- | --- | --- |
| D-009 | 用 `declare(StaticTask)` 取代 `spawn(closure)` | Micro 任务编译期确定；运行期 `spawn` 会破坏静态保证 |
| D-010 | v0 每个 `Platform` 只有一个 channel | 先固定语义；多通道/多端点留给 M4 的 Channel 域扩展 |
| D-011 | 内存域用 16 B × 16 固定块池 | 零分配、O(1)、可测；typed pool 在 M4 引入 |
| D-012 | PAL 契约由 EchoOS 定义，DoraOS 的 `dora-pal-conformance` 必须镜像同一组用例 | 契约以测试为准，不以文档为准 |

## 6. 明确限制

- ❌ 没有 `spawn` / 动态任务 / 动态加载；
- ❌ 没有阻塞收发、超时、背压；
- ❌ 没有多通道、设备域、传输域、存储域；
- ❌ 没有 POSIX 语义（ADR-0020 另立）；
- ❌ `Platform` 不是 `dyn` 安全的（关联类型 + 静态分发），这是刻意取舍。

## 7. 文件

| 文件 | 内容 |
| --- | --- |
| `crates/api/echo-pal-api/src/lib.rs` | 5 域 trait + `Capabilities` + `StaticTask` |
| `crates/test/echo-conformance/src/mock.rs` | `MockPlatform` 五域实现 |
| `crates/test/echo-conformance/tests/pal.rs` | 7 个 conformance 测试 |

## 8. 下一步（W4）

1. `tools/codegen`：YAML 子集 → 静态图代码，任务声明走 `TaskApi::declare`；
2. 生成物与手写 3 节点图 trace 一致；
3. 把 `dora-codegen` 的目标 API 冻结为 `echo-pal-api` + `echo-sched` 的公开面；
4. 新增 `docs/15-codegen.md`，开发日志追加 `W4-001`。
