//! 上下文切换入口占位（W7）。
//!
//! 真实实现需要 TCB 栈帧布局与 MSP/PSP 双栈；W7 只保证符号可链接、
//! 汇编可通过编译。真机上下文切换在硬件门通过后与 PSP 初始化一起补。
//!
//! 见 `docs/18-preemptive-sched.md` 的"未完成"一节。

core::arch::global_asm!(
    r#"
    .syntax unified
    .thumb

    .section .text.PendSV_Handler, "ax"
    .global PendSV_Handler
    .thumb_func
PendSV_Handler:
    bx lr

    .section .text.SVC_Handler, "ax"
    .global SVC_Handler
    .thumb_func
SVC_Handler:
    bx lr
"#
);
