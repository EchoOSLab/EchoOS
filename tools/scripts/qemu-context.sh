#!/usr/bin/env bash
# Q1 验收：QEMU MPS2-AN500 真实 PendSV/PSP 上下文切换 + 10k 次切换 canary。
set -uo pipefail
cd "$(dirname "$0")/../.."

LOG=target/qemu-logs/q1-context.log
mkdir -p target/qemu-logs

cargo build --target thumbv7em-none-eabihf -p echo-bsp-qemu-mps2 --bin echo-mps2-context || exit 1

rm -f "$LOG"
timeout 30 qemu-system-arm -accel tcg -M mps2-an500 -cpu cortex-m7 \
  -display none -monitor none \
  -chardev "file,id=semi0,path=$LOG" \
  -semihosting-config enable=on,target=native,chardev=semi0 \
  -kernel target/thumbv7em-none-eabihf/debug/echo-mps2-context
rc=$?

if grep -q "EchoOS MPS2: context OK" "$LOG" 2>/dev/null; then
  echo "PASS  qemu-mps2 context switch (exit=$rc, $(wc -c <"$LOG")B)"
  grep "context OK" "$LOG"
else
  echo "FAIL  qemu-mps2 context switch (exit=$rc)"
  echo "---- log ----"
  cat "$LOG" 2>/dev/null | cat -v
  exit 1
fi
