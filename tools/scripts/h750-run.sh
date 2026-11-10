#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

ELF="target/thumbv7em-none-eabihf/debug/echo-h750"

cargo build --target thumbv7em-none-eabihf -p echo-bsp-weact-h750vb

if ! command -v openocd >/dev/null 2>&1; then
  echo "缺少 openocd，请先安装：sudo apt install openocd" >&2
  exit 1
fi

echo "烧录并运行 $ELF"
echo "semihosting 输出会显示在本终端；Ctrl-C 退出。"
echo

exec openocd \
  -f interface/stlink.cfg \
  -f target/stm32h7x.cfg \
  -c "adapter speed 2000" \
  -c "arm semihosting enable" \
  -c "program $ELF verify" \
  -c "reset run"
