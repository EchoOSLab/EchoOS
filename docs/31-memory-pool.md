# 31 · 静态固定块内存池（Q2-10）

**状态**：✅ Q2-10 host 完成（2026-11-11）｜ **测试**：5

> QEMU/PAL 集成尚未接；先保证池的语义、错误路径和 host 测试。

## 1. API

```rust
pub struct BlockPool<const BLOCK_SIZE: usize, const BLOCKS: usize>;
pub struct PoolBlock { pub index: u16 }
pub enum MemError { OutOfMemory, InvalidBlock, DoubleFree }
```

| 方法 | 语义 |
|---|---|
| `new()` | 编译期固定块；BLOCKS ≤ 32 |
| `try_alloc() -> Result<PoolBlock, MemError>` | O(BLOCKS) 找空闲位；满返回 `OutOfMemory` |
| `free(block)` | 非法索引 → `InvalidBlock`；重复释放 → `DoubleFree` |
| `is_allocated(block)` | 位图查询 |
| `block(block)` / `block_mut(block)` | 仅已分配块可访问 |
| `used_blocks()` / `free_blocks()` | 统计 |
| `capacity_bytes()` / `used_bytes()` | 容量 |

## 2. 语义

- 无堆、无 `unsafe`、无外部依赖；
- 块大小与数量编译期确定；
- `free` 必须显式传入 `PoolBlock`；
- 非法索引/重复释放都返回错误，不静默；
- 单核使用；多核需要外层锁。

## 3. 测试

```bash
cargo test -p echo-mem --test pool
```

| 测试 | 证明 |
|---|---|
| `allocates_until_full` | 分配、满、容量统计 |
| `free_and_reuse` | 释放后可复用 |
| `double_free_is_rejected` | 重复释放错误 |
| `invalid_block_is_rejected` | 越界索引错误 |
| `block_data_roundtrip` | 块内数据读写、释放后不可访问 |

## 4. 限制

- 还没有接入 QEMU runtime / PAL `MemoryApi`；
- 没有线程安全包装；
- 没有碎片统计（固定块本身无外部碎片）；
- 没有 MPU 保护。

## 5. 下一步

1. 在 QEMU runtime 里用 `BlockPool` 做静态分配/释放测试；
2. 实现 PAL QEMU `MemoryApi`；
3. 接 Q3 的 MPU/栈保护。

## 6. 文件

| 文件 | 内容 |
|---|---|
| `crates/kernel/echo-mem/src/lib.rs` | `BlockPool` |
| `crates/kernel/echo-mem/tests/pool.rs` | 5 个测试 |
