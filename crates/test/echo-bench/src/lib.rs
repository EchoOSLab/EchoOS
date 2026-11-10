#![no_std]
#![forbid(unsafe_code)]

//! 实时性直方图（W8）。
//!
//! 设计契约见 `docs/19-time-and-bench.md`。
//!
//! - 固定容量样本、零分配；
//! - 报告 P50 / P99 / P99.9 / max / mean（整数，不依赖浮点）；
//! - 输出 JSON / CSV，便于 CI 回归。

use core::fmt::{self, Write};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Summary {
    pub samples: usize,
    pub min: u64,
    pub p50: u64,
    pub p99: u64,
    pub p999: u64,
    pub max: u64,
    pub mean: u64,
}

pub struct Histogram<const N: usize> {
    samples: [u64; N],
    len: usize,
}

impl<const N: usize> Histogram<N> {
    pub const fn new() -> Self {
        Self {
            samples: [0; N],
            len: 0,
        }
    }

    /// 记录一个样本；满时返回 `false`。
    pub fn record(&mut self, value: u64) -> bool {
        if self.len >= N {
            return false;
        }
        self.samples[self.len] = value;
        self.len += 1;
        true
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn is_full(&self) -> bool {
        self.len >= N
    }

    /// 排序并计算摘要；无样本时返回 `None`。
    pub fn summary(&mut self) -> Option<Summary> {
        if self.len == 0 {
            return None;
        }

        self.samples[..self.len].sort_unstable();
        let sorted = &self.samples[..self.len];

        let mut sum: u128 = 0;
        for &value in sorted {
            sum += u128::from(value);
        }

        Some(Summary {
            samples: self.len,
            min: sorted[0],
            p50: percentile(sorted, 50),
            p99: percentile(sorted, 99),
            p999: permille(sorted, 999),
            max: sorted[self.len - 1],
            mean: (sum / self.len as u128) as u64,
        })
    }

    pub fn write_json<W: Write>(&mut self, out: &mut W) -> fmt::Result {
        let summary = self.summary().ok_or(fmt::Error)?;
        write!(
            out,
            "{{\"samples\":{},\"min\":{},\"p50\":{},\"p99\":{},\"p999\":{},\"max\":{},\"mean\":{}}}",
            summary.samples,
            summary.min,
            summary.p50,
            summary.p99,
            summary.p999,
            summary.max,
            summary.mean
        )
    }

    pub fn write_csv<W: Write>(&mut self, out: &mut W) -> fmt::Result {
        let summary = self.summary().ok_or(fmt::Error)?;
        writeln!(out, "samples,min,p50,p99,p999,max,mean")?;
        writeln!(
            out,
            "{},{},{},{},{},{},{}",
            summary.samples,
            summary.min,
            summary.p50,
            summary.p99,
            summary.p999,
            summary.max,
            summary.mean
        )
    }
}

impl<const N: usize> Default for Histogram<N> {
    fn default() -> Self {
        Self::new()
    }
}

fn percentile(sorted: &[u64], percent: u8) -> u64 {
    let len = sorted.len();
    let index = (usize::from(percent) * len)
        .div_ceil(100)
        .saturating_sub(1)
        .min(len - 1);
    sorted[index]
}

fn permille(sorted: &[u64], per_mille: u16) -> u64 {
    let len = sorted.len();
    let index = (usize::from(per_mille) * len)
        .div_ceil(1000)
        .saturating_sub(1)
        .min(len - 1);
    sorted[index]
}
