# Contributing to EchoOS

Thanks for your interest. EchoOS is currently in **Phase 1** (the real-time control
kernel). Please keep changes focused on that scope.

## Before you start

- Read [docs/02-embodied-ai-robot-profile.md](docs/02-embodied-ai-robot-profile.md)
  and [docs/10-development-plan.md](docs/10-development-plan.md).
- Check the hardware gate: the real-time claims in this repository are **not**
  validated on hardware yet. Do not present host/QEMU results as hardware timing.
- Use stable Rust and the targets listed in `rust-toolchain.toml`.

## Build and test

```bash
cargo test
cargo xtask hello
cargo xtask graph
cargo build --target thumbv7em-none-eabihf -p echo-bsp-weact-h750vb
```

All tests must pass, and `cargo fmt --all -- --check` / `cargo clippy` should be clean.

## Commit messages

Commit messages are in **English**, using Conventional Commits:

```text
feat(sched): add preemptive scheduling and round-robin
fix(ipc): handle a full mailbox without dropping the head
docs(adr): add ADR-00xx for ...
test(codegen): cover the generated-vs-handwritten drift case
```

Guidelines:

- Subject in the imperative mood, at most ~72 characters; no trailing period.
- Body explains **why** and what changed, wrapped at 72 columns.
- Reference the relevant milestone or document (for example `W7`, `ADR-0020`,
  `docs/24-weact-h750-bringup.md`) in the body when useful.
- Do not add fake hardware numbers. If a change is compile-only, say so.

## Documentation requirement

Every feature or design change must include:

1. a contract/design note under `docs/`;
2. an entry in [docs/12-dev-log.md](docs/12-dev-log.md);
3. updates to [docs/10-development-plan.md](docs/10-development-plan.md) and
   [docs/README.md](docs/README.md) where relevant.

Chinese design documents are the current baseline; English summaries and README
improvements are welcome.

## Pull requests

- Keep each PR scoped to one feature or fix.
- Include the verification commands you ran and their results.
- Mark hardware-dependent work clearly as **compile-only** until measured on a board.
- Do not commit generated build artifacts under `target/`.

## License

By contributing, you agree that your contributions are licensed under the
project's dual MIT / Apache-2.0 license.
