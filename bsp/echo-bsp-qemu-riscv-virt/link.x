ENTRY(_start)

SECTIONS
{
  . = 0x80000000;

  .text : {
    *(.text.entry)
    *(.text*)
    *(.rodata*)
  }

  .data : {
    *(.data*)
  }

  .bss : {
    *(.bss*)
    *(COMMON)
  }

  . = ALIGN(16);
  . += 0x10000;              /* 64 KiB stack */
  _stack_top = .;
}
