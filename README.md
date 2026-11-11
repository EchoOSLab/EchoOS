# EchoOS

> English | [中文](README.zh-CN.md)

A **Rust `no_std` hard real-time operating system (Phase 1)** for embodied AI robotics.
EchoOS runs at the **cerebellum (coordination)** and **spinal cord (joint actuation)** layers,
forming a real-time boundary between a non-deterministic AI stack and physical actuators that
must remain reliable.

> **Current status (2026-11-11)**: Phase 1 · **82 tests passing** · QEMU four-target PASS ·
> PAL v0.1 frozen · **D90: conditional Go**
> **Hardware status**: the WeAct STM32H750VBT6 BSP compiles, but **the board has not arrived**;
> there is no boot log and no jitter data.

## What can be verified today

| Capability | Status |
|---|---|
| Host tests (scheduling / IPC / PAL / codegen / time / bench) | ✅ 82 passing |
| QEMU Cortex-M3/M7, RISC-V, x86_64 UEFI | ✅ `cargo xtask hello` |
| Same YAML-generated graph on QEMU M7 for 10k frames | ✅ `cargo xtask graph` |
| PAL v0.1 + two-backend conformance | ✅ frozen + 4 dual-backend tests |
| WeAct H750 BSP | ✅ compiles; ⚠️ not yet run on hardware |
| Real context switch / 480 MHz / jitter | ⛔ hardware gate |

## Quick start

```bash
cargo test                  # 82 passing
cargo xtask hello           # QEMU four targets
cargo xtask graph           # QEMU M7: generated graph, 10k frames
cargo build --target thumbv7em-none-eabihf -p echo-bsp-weact-h750vb
./tools/scripts/h750-run.sh # once the board arrives: OpenOCD + semihosting
```

## Hardware

- Selected board: **WeAct STM32H750VBT6 + ST-Link V2** (128 KB Flash / 1 MB RAM);
- First boot runs at **HSI 64 MHz**; the PLL is not configured to 480 MHz yet;
- Wiring, SWD pinout, and GDB instructions: [docs/24-weact-h750-bringup.md](docs/24-weact-h750-bringup.md);
- **No real hardware data, no "pass" claim**: D60 is still open; all real-time statements
  must wait for measured H750 data.

## Repository layout

```text
crates/arch/       Arch abstraction, MockArch, Cortex-M DWT time base
crates/core/       trace Sink / semihosting
crates/kernel/     echo-sched, echo-ipc, echo-time
crates/api/        echo-pal-api (PAL v0.1)
crates/test/       conformance, echo-bench
tools/codegen/     YAML to static schedule graph
tools/scripts/     QEMU / H750 run scripts
bsp/               QEMU M3/M7/RV32/x86_64, Nucleo-H743, WeAct-H750
docs/              design documents, ADRs, dev log, bring-up, D90 review
```

## Documentation

Start with [docs/README.md](docs/README.md) (Chinese). Key entry points:

- [02 Requirements baseline](docs/02-embodied-ai-robot-profile.md)
- [10 Development plan and progress](docs/10-development-plan.md)
- [12 Development log](docs/12-dev-log.md)
- [23 D90 review](docs/23-d90-review.md)
- [24 H750 bring-up and debugging](docs/24-weact-h750-bringup.md)
- [25 Repository model: single open-source version](docs/25-repo-model.md)
- [26 QEMU-first development task plan](docs/26-qemu-first-plan.md)
- [27 QEMU MPS2-AN500 runtime (Q0)](docs/27-qemu-runtime.md)
- [28 QEMU context switching (Q1)](docs/28-qemu-context.md)
- [29 Scheduler sleep/wake and sync primitives (Q2 partial)](docs/29-scheduler-sync.md)
- [30 QEMU scheduler runtime: idle, tick, block, sleep (Q2)](docs/30-qemu-scheduler.md)
- [31 Static memory pool (Q2)](docs/31-memory-pool.md)
- [32 QEMU faults, icount, and CI (Q3)](docs/32-qemu-faults-ci.md)

## Scope and honesty boundaries

- Phase 1 only: the real-time control kernel; **no** POSIX substrate, EtherCAT, or a
  42-crate expansion;
- L1: zero external Rust crate dependencies;
- The preemptive **context switch is still a PendSV/SVC placeholder**; the real switch
  waits for the H750 board;
- D90 is **conditional Go**, not "done";
- Host/QEMU results are functional evidence, not hardware timing evidence.

## License

Dual-licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
