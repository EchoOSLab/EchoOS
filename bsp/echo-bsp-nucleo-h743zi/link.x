ENTRY(Reset_Handler)

MEMORY
{
  FLASH (rx)  : ORIGIN = 0x08000000, LENGTH = 2M
  DTCM  (rwx) : ORIGIN = 0x20000000, LENGTH = 128K
  AXI   (rwx) : ORIGIN = 0x24000000, LENGTH = 512K
}

SECTIONS
{
  .vector_table : {
    KEEP(*(.vector_table))
  } > FLASH

  .text : {
    *(.text.Reset_Handler)
    *(.text*)
    *(.rodata*)
    . = ALIGN(4);
  } > FLASH

  .data : {
    *(.data*)
  } > AXI AT > FLASH

  .bss : {
    *(.bss*)
    *(COMMON)
  } > AXI

  /DISCARD/ : {
    *(.ARM.exidx*)
    *(.ARM.extab*)
    *(.comment)
  }
}
