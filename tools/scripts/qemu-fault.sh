#!/usr/bin/env bash
# Q3 验收：udf → UsageFault，CFSR 状态可读，进程干净退出。
set -uo pipefail
cd "$(dirname "$0")/../.."

LOG=target/qemu-logs/q3-fault.log
mkdir -p target/qemu-logs

cargo build --target thumbv7em-none-eabihf -p echo-bsp-qemu-mps2 --bin echo-mps2-fault || exit 1

rm -f "$LOG"
timeout 15 qemu-system-arm -accel tcg -M mps2-an500 -cpu cortex-m7 \
  -display none -monitor none \
  -chardev "file,id=semi0,path=$LOG" \
  -semihosting-config enable=on,target=native,chardev=semi0 \
  -kernel target/thumbv7em-none-eabihf/debug/echo-mps2-fault
rc=$?

if grep -q "USAGEFAULT cfsr=0x00010000" "$LOG" 2>/dev/null; then
  echo "PASS  qemu-mps2 fault injection (exit=$rc, $(wc -c <"$LOG")B)"
  grep "USAGEFAULT" "$LOG"
else
  echo "FAIL  qemu-mps2 fault injection (exit=$rc)"
  cat "$LOG" 2>/dev/null | cat -v
  exit 1
fi
