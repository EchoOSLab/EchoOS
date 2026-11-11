#!/usr/bin/env bash
# Q1-07：用 GDB 在 PendSV 切换点停下，验证调试链路。
set -uo pipefail
cd "$(dirname "$0")/../.."

ELF=target/thumbv7em-none-eabihf/debug/echo-mps2-context
LOG=target/qemu-logs/q1-context-gdb.log
mkdir -p target/qemu-logs

cargo build --target thumbv7em-none-eabihf -p echo-bsp-qemu-mps2 --bin echo-mps2-context || exit 1

rm -f "$LOG"
timeout 20 qemu-system-arm -accel tcg -M mps2-an500 -cpu cortex-m7 \
  -display none -monitor none -s -S \
  -chardev "file,id=semi0,path=$LOG.qemu" \
  -semihosting-config enable=on,target=native,chardev=semi0 \
  -kernel "$ELF" &
QPID=$!
trap 'kill "$QPID" 2>/dev/null || true' EXIT
sleep 1

gdb-multiarch -q -batch \
  -ex "set confirm off" \
  -ex "target remote :1234" \
  -ex "break pendsv_switch" \
  -ex "continue" \
  -ex "info registers pc" \
  -ex "detach" \
  "$ELF" >"$LOG" 2>&1
rc=$?

if grep -q "pendsv_switch" "$LOG" 2>/dev/null && grep -q "pc" "$LOG" 2>/dev/null; then
  echo "PASS  qemu-mps2 context GDB breakpoint (exit=$rc, $(wc -c <"$LOG")B)"
else
  echo "FAIL  qemu-mps2 context GDB breakpoint (exit=$rc)"
  cat "$LOG" 2>/dev/null
  exit 1
fi
