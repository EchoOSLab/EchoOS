#!/usr/bin/env bash
# 一条命令跑完 x86 / ARM / RISC-V 的 QEMU Hello 冒烟测试。
#
# 关键设计：串口输出直接写文件（-serial file:... / -chardev file），
# 不依赖 QEMU 的 stdio/终端捕获，避免不同终端环境下输出为空。
# 三个 guest 都会自己干净退出（ARM semihosting SYS_EXIT / RISC-V sifive_test /
# x86 isa-debug-exit），timeout 只作为兜底。
set -uo pipefail
cd "$(dirname "$0")/../.."

fail=0
LOGDIR=target/qemu-logs
mkdir -p "$LOGDIR"

for bin in cargo qemu-system-arm qemu-system-riscv32 qemu-system-x86_64; do
  if ! command -v "$bin" >/dev/null 2>&1; then
    echo "MISSING: $bin (PATH=$PATH)"
    exit 2
  fi
done

check_log() {
  local name="$1" log="$2"; shift 2
  rm -f "$log" "$log.err"
  "$@" >/dev/null 2>"$log.err"
  local rc=$?
  local out bytes
  out=$(cat "$log" 2>/dev/null; cat "$log.err" 2>/dev/null)
  bytes=$(wc -c <"$log" 2>/dev/null || echo 0)
  if grep -q "Hello EchoOS" <<<"$out"; then
    echo "PASS  $name (exit=$rc, log=${bytes}B)"
  else
    echo "FAIL  $name (exit=$rc, log=${bytes}B)"
    echo "---- log ----"
    head -20 "$log" 2>/dev/null | cat -v
    echo "---- stderr ----"
    head -10 "$log.err" 2>/dev/null
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
# ARM：semihosting console 绑到 file chardev
check_log "arm-cortex-m3 (mps2-an385)" "$LOGDIR/arm-m3.log" timeout 15 \
  qemu-system-arm -accel tcg -M mps2-an385 -cpu cortex-m3 \
  -display none -monitor none \
  -chardev "file,id=semi0,path=$LOGDIR/arm-m3.log" \
  -semihosting-config enable=on,target=native,chardev=semi0 \
  -kernel target/thumbv7m-none-eabi/debug/echo-arm

check_log "arm-cortex-m7 (mps2-an500)" "$LOGDIR/arm-m7.log" timeout 15 \
  qemu-system-arm -accel tcg -M mps2-an500 -cpu cortex-m7 \
  -display none -monitor none \
  -chardev "file,id=semi0,path=$LOGDIR/arm-m7.log" \
  -semihosting-config enable=on,target=native,chardev=semi0 \
  -kernel target/thumbv7em-none-eabihf/debug/echo-arm

# RISC-V：UART0 串口直接写文件
check_log "riscv32 (virt)" "$LOGDIR/riscv32.log" timeout 15 \
  qemu-system-riscv32 -accel tcg -M virt -display none -monitor none \
  -serial "file:$LOGDIR/riscv32.log" -bios none \
  -kernel target/riscv32imac-unknown-none-elf/debug/echo-riscv

# x86_64：UEFI + OVMF；COM1 写文件，isa-debug-exit 退出
OVMF="${OVMF:-/usr/share/ovmf/OVMF.fd}"
[ -f "$OVMF" ] || OVMF=/usr/share/OVMF/OVMF.fd
if [ ! -f "$OVMF" ]; then
  echo "MISSING: OVMF firmware (set OVMF=/path/to/OVMF.fd)"
  exit 2
fi
rm -rf target/esp
mkdir -p target/esp/EFI/BOOT
cp target/x86_64-unknown-uefi/debug/echo-x86.efi target/esp/EFI/BOOT/BOOTX64.EFI

check_log "x86_64 (UEFI + OVMF)" "$LOGDIR/x86.log" timeout 120 \
  qemu-system-x86_64 -accel tcg -machine q35 -m 256 -bios "$OVMF" \
  -drive format=raw,file=fat:rw:target/esp \
  -device isa-debug-exit,iobase=0x501,iosize=0x04 \
  -display none -monitor none -serial "file:$LOGDIR/x86.log" \
  -no-reboot -net none

exit $fail
