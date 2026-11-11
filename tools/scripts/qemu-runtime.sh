#!/usr/bin/env bash
# Q0 验收：QEMU MPS2-AN500 runtime 启动 + SysTick + semihosting 退出。
set -uo pipefail
cd "$(dirname "$0")/../.."

LOG=target/qemu-logs/q0-runtime.log
mkdir -p target/qemu-logs

cargo build --target thumbv7em-none-eabihf -p echo-bsp-qemu-mps2 || exit 1

rm -f "$LOG"
timeout 15 qemu-system-arm -accel tcg -M mps2-an500 -cpu cortex-m7 \
  -display none -monitor none \
  -chardev "file,id=semi0,path=$LOG" \
  -semihosting-config enable=on,target=native,chardev=semi0 \
  -kernel target/thumbv7em-none-eabihf/debug/echo-mps2-runtime
rc=$?

if grep -q "EchoOS MPS2: boot" "$LOG" 2>/dev/null \
  && grep -q "EchoOS MPS2: systick OK" "$LOG" 2>/dev/null; then
  echo "PASS  qemu-mps2 runtime (exit=$rc, $(wc -c <"$LOG")B)"
else
  echo "FAIL  qemu-mps2 runtime (exit=$rc)"
  echo "---- log ----"
  cat "$LOG" 2>/dev/null | cat -v
  exit 1
fi
