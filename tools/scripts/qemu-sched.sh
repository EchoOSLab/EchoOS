#!/usr/bin/env bash
# Q2 验收：QEMU idle + SysTick + block/sleep/wake。
set -uo pipefail
cd "$(dirname "$0")/../.."

LOG=target/qemu-logs/q2-sched.log
mkdir -p target/qemu-logs

cargo build --target thumbv7em-none-eabihf -p echo-bsp-qemu-mps2 --bin echo-mps2-sched || exit 1

rm -f "$LOG"
timeout 30 qemu-system-arm -accel tcg -M mps2-an500 -cpu cortex-m7 \
  -display none -monitor none \
  -chardev "file,id=semi0,path=$LOG" \
  -semihosting-config enable=on,target=native,chardev=semi0 \
  -kernel target/thumbv7em-none-eabihf/debug/echo-mps2-sched
rc=$?

if grep -q "EchoOS MPS2: sched OK" "$LOG" 2>/dev/null; then
  echo "PASS  qemu-mps2 scheduler (exit=$rc, $(wc -c <"$LOG")B)"
  grep "sched OK" "$LOG"
else
  echo "FAIL  qemu-mps2 scheduler (exit=$rc)"
  cat "$LOG" 2>/dev/null | cat -v
  exit 1
fi
