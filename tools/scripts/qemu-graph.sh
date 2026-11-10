#!/usr/bin/env bash
# QEMU Cortex-M7 上跑 3 节点图（功能验证，不产出抖动数字）。
set -uo pipefail
cd "$(dirname "$0")/../.."

LOG=target/qemu-logs/graph-m7.log
mkdir -p target/qemu-logs

cargo build --target thumbv7em-none-eabihf -p echo-bsp-qemu-cortex-m --bin echo-arm-graph || exit 1

rm -f "$LOG"
qemu-system-arm -accel tcg -M mps2-an500 -cpu cortex-m7 \
  -display none -monitor none \
  -chardev "file,id=semi0,path=$LOG" \
  -semihosting-config enable=on,target=native,chardev=semi0 \
  -kernel target/thumbv7em-none-eabihf/debug/echo-arm-graph
rc=$?

if grep -q "EchoOS: 3-node graph OK" "$LOG" 2>/dev/null; then
  echo "PASS  qemu-m7 3-node graph (exit=$rc, $(wc -c <"$LOG")B)"
else
  echo "FAIL  qemu-m7 3-node graph (exit=$rc)"
  cat "$LOG" 2>/dev/null
  exit 1
fi
