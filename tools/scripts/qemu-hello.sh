#!/usr/bin/env bash
# 一条命令跑完 x86 / ARM / RISC-V 的 QEMU Hello 冒烟测试。
set -uo pipefail
cd "$(dirname "$0")/../.."

fail=0

check() {
  local name="$1"; shift
  local out
  out=$("$@" 2>&1 || true)
  if grep -q "Hello EchoOS" <<<"$out"; then
    echo "PASS  $name"
  else
    echo "FAIL  $name"
    echo "---- output (tail) ----"
    echo "$out" | tail -20
    fail=1
  fi
}

echo "== build =="
cargo build --target thumbv7m-none-eabi           -p echo-bsp-qemu-cortex-m
cargo build --target thumbv7em-none-eabihf        -p echo-bsp-qemu-cortex-m
cargo build --target riscv32imac-unknown-none-elf -p echo-bsp-qemu-riscv-virt
cargo build --target x86_64-unknown-uefi          -p echo-bsp-qemu-x86

echo "== run =="
check "arm-cortex-m3 (mps2-an385)" timeout 10 \
  qemu-system-arm -M mps2-an385 -cpu cortex-m3 -nographic -semihosting \
  -kernel target/thumbv7m-none-eabi/debug/echo-arm

check "arm-cortex-m7 (mps2-an500)" timeout 10 \
  qemu-system-arm -M mps2-an500 -cpu cortex-m7 -nographic -semihosting \
  -kernel target/thumbv7em-none-eabihf/debug/echo-arm

check "riscv32 (virt)" timeout 10 \
  qemu-system-riscv32 -M virt -nographic -bios none \
  -kernel target/riscv32imac-unknown-none-elf/debug/echo-riscv

# x86_64: UEFI 应用 + OVMF；串口直写 COM1
OVMF="${OVMF:-/usr/share/ovmf/OVMF.fd}"
[ -f "$OVMF" ] || OVMF=/usr/share/OVMF/OVMF.fd
rm -rf target/esp
mkdir -p target/esp/EFI/BOOT
cp target/x86_64-unknown-uefi/debug/echo-x86.efi target/esp/EFI/BOOT/BOOTX64.EFI

check "x86_64 (UEFI + OVMF)" timeout 40 \
  qemu-system-x86_64 -machine q35 -m 256 -bios "$OVMF" \
  -drive format=raw,file=fat:rw:target/esp \
  -display none -serial stdio -no-reboot -net none

exit $fail
