#![no_std]

//! 跨 crate 的一致性测试宿主。
//!
//! - `mock`：PAL 的 mock 后端（host 测试用）；
//! - `mock_small`：第二个 PAL 后端（小容量，验证 conformance 不依赖容量）；
//! - `tests/`：3 节点图、PAL 契约等验收用例。

pub mod mock;
pub mod mock_small;
