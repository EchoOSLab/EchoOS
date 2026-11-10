# 21 · PAL v0.1 冻结与第二后端（W10）

**状态**：✅ PAL 契约冻结（v0.1.0）｜ **测试**：80 passed（新增 4）｜ **后端**：2 个

> 本文是 W10 的契约冻结记录。
> 上游：[14-pal-contract](14-pal-contract.md)；DoraOS 的 `dora-pal-conformance` 必须镜像同一组用例。

## 1. 冻结内容

```rust
pub const PAL_VERSION: (u8, u8, u8) = (0, 1, 0);

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

五个域：`TaskApi` / `TimeApi` / `MemoryApi` / `ChannelApi` / `ObservabilityApi`。

**冻结的语义**：

- 静态任务（`declare`），运行期不创建；
- 无动态内存（固定块池）；
- 通道有界、非阻塞；
- 能力矩阵（`Capabilities::micro`）；
- 错误类型（`SchedError` / `TaskError` / `TimeError` 各自的公开变体）。

**不冻结的**：具体容量、任务 id 数值、可选域（Transport/Storage/Device/Isolation/Loading）、后端实现细节。

## 2. 变更策略

| 变更 | 规则 |
| --- | --- |
| 新增可选能力/方法（带默认实现） | 允许，patch 版本 |
| 修改已有方法签名/语义 | **禁止**；必须提升 minor 并新增 conformance 用例 |
| 修改 `Capabilities` 字段 | 禁止；新增字段需版本升级与 conformance 更新 |
| 修改错误变体 | 禁止；新增变体需版本升级 |

**理由**：DoraOS 的 codegen 直接依赖这些签名；冻结后只做加法，避免生成代码大面积重写。

## 3. 第二后端

| 能力 | `MockPlatform` | `SmallMockPlatform` |
| --- | --- | --- |
| 静态任务 | 8 | **2** |
| trace 容量 | 64 | **8** |
| 内存池 | 16×16 B = 256 B | **4×8 B = 32 B** |
| 通道槽位 | 8（容量 7） | **2（容量 1）** |
| 能力矩阵 | Micro | Micro（相同） |

第二后端**故意采用更小容量**，用来证明 conformance 只依赖契约，不依赖某个后端的常量。

## 4. Conformance 测试（4 个）

```bash
cargo test -p echo-conformance --test pal_two_backends
```

| 测试 | 证明 |
| --- | --- |
| `pal_version_is_frozen` | `PAL_VERSION == (0,1,0)` |
| `two_backends_pass_same_conformance` | 同一泛型 `check_common<P: Platform>` 同时通过两个后端 |
| `capacities_differ_but_semantics_match` | 容量不同，语义一致 |
| `small_backend_enforces_its_limits` | 小后端正确返回容量错误/丢弃计数 |

## 5. 决策

| # | 决策 | 理由 |
| --- | --- | --- |
| D-037 | 用 `PAL_VERSION` 常量显式版本化，而非只靠 git tag | DoraOS 侧需要在编译期/运行期协商版本 |
| D-038 | 第二后端只改容量、不改语义 | 证明 conformance 的契约性；避免"第二个 mock 复制粘贴"变成假验证 |
| D-039 | 冻结以 conformance 测试为准，不以文档为准 | 文档会漂移，测试不会（CI 守门） |

## 6. 文件

| 文件 | 内容 |
| --- | --- |
| `crates/api/echo-pal-api/src/lib.rs` | `PAL_VERSION` + 5 域 trait |
| `crates/test/echo-conformance/src/mock_small.rs` | 第二后端 |
| `crates/test/echo-conformance/tests/pal_two_backends.rs` | 4 个 conformance 测试 |

## 7. 下一步（W11）

用**同一份生成图**在两个后端/环境下对比：

1. host `MockPlatform` 通道；
2. host `SmallMockPlatform` 通道；
3. QEMU M7 生成图（W5 已有摘要结果）；
4. 新增 `docs/22-two-backends.md`，追加 `W11-001`。

> QEMU 只提供摘要（OK/帧数/丢包），没有逐帧 trace；对比报告必须写明这一点。
