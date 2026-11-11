#!/usr/bin/env bash
# Q3 回归：-icount 下两次运行的 sched trace 必须完全一致（确定性，不是时序）。
set -uo pipefail
cd "$(dirname "$0")/../.."

mkdir -p target/qemu-logs
cargo build --target thumbv7em-none-eabihf -p echo-bsp-qemu-mps2 --bin echo-mps2-sched || exit 1

run_once() {
  local log="$1"
  rm -f "$log"
  timeout 30 qemu-system-arm -accel tcg -M mps2-an500 -cpu cortex-m7 -icount shift=0 \
    -display none -monitor none \
    -chardev "file,id=semi0,path=$log" \
    -semihosting-config enable=on,target=native,chardev=semi0 \
    -kernel target/thumbv7em-none-eabihf/debug/echo-mps2-sched
}

run_once target/qemu-logs/icount-1.log
rc1=$?
run_once target/qemu-logs/icount-2.log
rc2=$?

if grep -q "sched OK" target/qemu-logs/icount-1.log 2>/dev/null \
  && cmp -s target/qemu-logs/icount-1.log target/qemu-logs/icount-2.log; then
  echo "PASS  qemu-mps2 -icount deterministic (exit=$rc1/$rc2)"
  grep "sched OK" target/qemu-logs/icount-1.log
else
  echo "FAIL  qemu-mps2 -icount deterministic (exit=$rc1/$rc2)"
  echo "---- run1 ----"; cat target/qemu-logs/icount-1.log 2>/dev/null | cat -v
  echo "---- run2 ----"; cat target/qemu-logs/icount-2.log 2>/dev/null | cat -v
  exit 1
fi
