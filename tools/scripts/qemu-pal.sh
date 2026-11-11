#!/usr/bin/env bash
# Q2-17 验收：同一份 contract::check_common 在 QEMU PAL 后端通过。
set -uo pipefail
cd "$(dirname "$0")/../.."

LOG=target/qemu-logs/q2-pal.log
mkdir -p target/qemu-logs

cargo build --target thumbv7em-none-eabihf -p echo-bsp-qemu-mps2 --bin echo-mps2-pal || exit 1

rm -f "$LOG"
timeout 15 qemu-system-arm -accel tcg -M mps2-an500 -cpu cortex-m7 \
  -display none -monitor none \
  -chardev "file,id=semi0,path=$LOG" \
  -semihosting-config enable=on,target=native,chardev=semi0 \
  -kernel target/thumbv7em-none-eabihf/debug/echo-mps2-pal
rc=$?

if grep -q "PAL QEMU: OK" "$LOG" 2>/dev/null; then
  echo "PASS  qemu-mps2 PAL conformance (exit=$rc, $(wc -c <"$LOG")B)"
else
  echo "FAIL  qemu-mps2 PAL conformance (exit=$rc)"
  cat "$LOG" 2>/dev/null | cat -v
  exit 1
fi
