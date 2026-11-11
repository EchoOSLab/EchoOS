ENTRY(Reset_Handler)

MEMORY
{
  CODE (rx)  : ORIGIN = 0x00000000, LENGTH = 1M
  RAM  (rwx) : ORIGIN = 0x20000000, LENGTH = 256K
}

SECTIONS
{
  .vector_table : {
    KEEP(*(.vector_table))
  } > CODE

  .text : {
    *(.text.Reset_Handler)
    *(.text*)
    *(.rodata*)
    . = ALIGN(4);
  } > CODE

  .data : {
    _sdata = .;
    *(.data*)
    . = ALIGN(4);
    _edata = .;
  } > RAM AT > CODE
  _sidata = LOADADDR(.data);

  .bss : {
    _sbss = .;
    *(.bss*)
    *(COMMON)
    . = ALIGN(4);
    _ebss = .;
  } > RAM

  _stack_top = ORIGIN(RAM) + LENGTH(RAM);

  /DISCARD/ : {
    *(.ARM.exidx*)
    *(.ARM.extab*)
    *(.comment)
  }
}
