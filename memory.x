MEMORY {
    /*
     * The RP2350 has either external or internal flash.
     *
     * 2 MiB is a safe default here, although a Pico 2 has 4 MiB.
     */
    FLASH : ORIGIN = 0x10000000, LENGTH = 2048K
    /*
     * RAM consists of 8 banks, SRAM0-SRAM7, with a striped mapping.
     * This is usually good for performance, as it distributes load on
     * those banks evenly.
     */
    RAM : ORIGIN = 0x20000000, LENGTH = 256K

    /*
     * Mirror of the GB memory map, filled by the bus mirror DMA. Every byte the GB accesses in
     * 0xA000 to 0xFDFF lands at the same offset in here. It has to be a 64K aligned window, so
     * the GB address can be used directly as the lower half of the DMA write pointer. Whatever
     * address gets recorded, the DMA can never write outside of this window.
     */
    GB_BUS_MIRROR : ORIGIN = 0x20000000 + 256K, LENGTH = 64K

    GB_ROM_MEMORY : ORIGIN = 0x20000000 + 320K, LENGTH = 64K

    GB_SAVE_RAM : ORIGIN = 0x20000000 + 320K + 64K, LENGTH = 128K

    /*
     * RAM banks 8 and 9 use a direct mapping. They can be used to have
     * memory areas dedicated for some specific job, improving predictability
     * of access times.
     * Example: Separate stacks for core0 and core1.
     */
    SRAM4 : ORIGIN = 0x20080000, LENGTH = 4K
    SRAM5 : ORIGIN = 0x20081000, LENGTH = 4K
}

SECTIONS {
    /* ### Boot ROM info
     *
     * Goes after .vector_table, to keep it in the first 4K of flash
     * where the Boot ROM (and picotool) can find it
     */
    .start_block : ALIGN(4)
    {
        __start_block_addr = .;
        KEEP(*(.start_block));
        KEEP(*(.boot_info));
    } > FLASH

} INSERT AFTER .vector_table;

/* move .text to start /after/ the boot info */
_stext = ADDR(.start_block) + SIZEOF(.start_block);

SECTIONS {
    /* ### Picotool 'Binary Info' Entries
     *
     * Picotool looks through this block (as we have pointers to it in our
     * header) to find interesting information.
     */
    .bi_entries : ALIGN(4)
    {
        /* We put this in the header */
        __bi_entries_start = .;
        /* Here are the entries */
        KEEP(*(.bi_entries));
        /* Keep this block a nice round size */
        . = ALIGN(4);
        /* We put this in the header */
        __bi_entries_end = .;
    } > FLASH
} INSERT AFTER .text;

SECTIONS {
    /* ### Boot ROM extra info
     *
     * Goes after everything in our program, so it can contain a signature.
     */
    .end_block : ALIGN(4)
    {
        __end_block_addr = .;
        KEEP(*(.end_block));
    } > FLASH

} INSERT AFTER .uninit;

SECTIONS {
    .gb_rom_memory (NOLOAD): ALIGN(4)
    {
        _s_gb_rom_memory = .;
        KEEP(*(.gb_rom_memory));
    } > GB_ROM_MEMORY

    .gb_save_ram (NOLOAD): ALIGN(4)
    {
        _s_gb_save_ram = .;
        KEEP(*(.gb_save_ram));
    } > GB_SAVE_RAM

    .gb_bus_mirror (NOLOAD): ALIGN(4)
    {
        _s_gb_bus_mirror = .;
        KEEP(*(.gb_bus_mirror));
    } > GB_BUS_MIRROR
} INSERT AFTER .end_block;

PROVIDE(start_to_end = __end_block_addr - __start_block_addr);
PROVIDE(end_to_start = __start_block_addr - __end_block_addr);

ASSERT(ORIGIN(GB_BUS_MIRROR) % 0x10000 == 0, "GB_BUS_MIRROR must be 64K aligned");
