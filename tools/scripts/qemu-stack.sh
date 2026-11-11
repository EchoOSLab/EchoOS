#!/usr/bin/env bash
# Q2-11 验收：栈 canary 被破坏时报告任务索引并退出。
set -uo pipefail
cd "$(dirname "$0")/../.."

LOG=target/qemu-logs/q2-stack.log
TARGET=target/qemu-stack
mkdir -p target/qemu-logs

CARGO_TARGET_DIR="$TARGET" cargo build --target thumbv7em-none-eabihf \
  -p echo-bsp-qemu-mps2 --bin echo-mps2-sched --features stack-fault || exit 1

rm -f "$LOG"
timeout 20 qemu-system-arm -accel tcg -M mps2-an500 -cpu cortex-m7 \
  -display none -monitor none \
  -chardev "file,id=semi0,path=$LOG" \
  -semihosting-config enable=on,target=native,chardev=semi0 \
  -kernel "$TARGET/thumbv7em-none-eabihf/debug/echo-mps2-sched"
rc=$?

if grep -q "STACK OVERFLOW task=0" "$LOG" 2>/dev/null; then
  echo "PASS  qemu-mps2 stack overflow report (exit=$rc, $(wc -c <"$LOG")B)"
  grep "STACK OVERFLOW" "$LOG"
else
  echo "FAIL  qemu-mps2 stack overflow report (exit=$rc)"
  cat "$LOG" 2>/dev/null | cat -v
  exit 1
fi
