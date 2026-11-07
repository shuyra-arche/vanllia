OUTPUT_ARCH(riscv)
ENTRY(_start)
PHDRS { image PT_LOAD FLAGS(5); }
SECTIONS {
  . = 0x10000;
  __image_start = .;
  .text : { KEEP(*(.text.entry)) *(.text .text.*) } :image
  .rodata : { *(.rodata .rodata.*) *(.srodata .srodata.*) } :image
  . = ALIGN(4096);
  __image_end = .;
  /DISCARD/ : { *(.eh_frame*) *(.comment*) }
}
