#!/usr/bin/env bash
# 一条命令跑完 x86 / ARM / RISC-V 的 QEMU Hello 冒烟测试。
# 三个 guest 都会自己干净退出（不会靠 timeout 杀进程），因此输出可稳定捕获。
set -uo pipefail
cd "$(dirname "$0")/../.."

fail=0

for bin in cargo qemu-system-arm qemu-system-riscv32 qemu-system-x86_64; do
  if ! command -v "$bin" >/dev/null 2>&1; then
    echo "MISSING: $bin (PATH=$PATH)"
    exit 2
  fi
done

check() {
  local name="$1"; shift
  local out rc
  out=$("$@" 2>&1)
  rc=$?
  if grep -q "Hello EchoOS" <<<"$out"; then
    echo "PASS  $name (exit=$rc)"
  else
    echo "FAIL  $name (exit=$rc, bytes=${#out})"
    echo "---- output ----"
    printf '%s\n' "$out" | head -20
    echo "---- command ----"
    printf '  %q' "$@"; echo
    fail=1
  fi
}

echo "== build =="
cargo build --target thumbv7m-none-eabi           -p echo-bsp-qemu-cortex-m || exit 1
cargo build --target thumbv7em-none-eabihf        -p echo-bsp-qemu-cortex-m || exit 1
cargo build --target riscv32imac-unknown-none-elf -p echo-bsp-qemu-riscv-virt || exit 1
cargo build --target x86_64-unknown-uefi          -p echo-bsp-qemu-x86 || exit 1

echo "== run =="
check "arm-cortex-m3 (mps2-an385)" timeout 15 \
  qemu-system-arm -M mps2-an385 -cpu cortex-m3 -nographic -semihosting \
  -kernel target/thumbv7m-none-eabi/debug/echo-arm

check "arm-cortex-m7 (mps2-an500)" timeout 15 \
  qemu-system-arm -M mps2-an500 -cpu cortex-m7 -nographic -semihosting \
  -kernel target/thumbv7em-none-eabihf/debug/echo-arm

check "riscv32 (virt)" timeout 15 \
  qemu-system-riscv32 -M virt -nographic -bios none \
  -kernel target/riscv32imac-unknown-none-elf/debug/echo-riscv

# x86_64: UEFI 应用 + OVMF；串口直写 COM1，QEMU isa-debug-exit 退出
OVMF="${OVMF:-/usr/share/ovmf/OVMF.fd}"
[ -f "$OVMF" ] || OVMF=/usr/share/OVMF/OVMF.fd
if [ ! -f "$OVMF" ]; then
  echo "MISSING: OVMF firmware (set OVMF=/path/to/OVMF.fd)"
  exit 2
fi
rm -rf target/esp
mkdir -p target/esp/EFI/BOOT
cp target/x86_64-unknown-uefi/debug/echo-x86.efi target/esp/EFI/BOOT/BOOTX64.EFI

check "x86_64 (UEFI + OVMF)" timeout 40 \
  qemu-system-x86_64 -machine q35 -m 256 -bios "$OVMF" \
  -drive format=raw,file=fat:rw:target/esp \
  -device isa-debug-exit,iobase=0x501,iosize=0x04 \
  -display none -serial stdio -no-reboot -net none

exit $fail
