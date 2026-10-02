/* Test ROM for the work RAM mirror of the cartridge.
 *
 * Fills 0xC400 to 0xDBFF with a pattern which depends on the address and on a seed. The seed
 * changes about once a second, so the whole block gets rewritten over and over. A header in
 * front of the block tells the host which seed to expect and whether the block is complete.
 * tools/wram_mirror.py verify reads the mirror over the tether and checks every byte.
 *
 * 0xC000 to 0xC09F is the shadow OAM of GBDK, its variables follow at 0xC0A0 and the stack
 * grows down from 0xDFFF. Neither comes close to the block.
 */

#include <gb/gb.h>
#include <stdint.h>
#include <stdio.h>

#define HEADER ((volatile uint8_t *)0xC3F0)
#define BLOCK_START 0xC400u
#define BLOCK_END 0xDC00u

#define STATE_FILLING 0x00u
#define STATE_DONE 0xA5u

static uint8_t pattern(uint16_t addr, uint8_t seed)
{
    uint8_t lo = (uint8_t)addr;
    uint8_t hi = (uint8_t)(addr >> 8);
    return lo ^ (uint8_t)(hi * 7u) ^ seed;
}

void main(void)
{
    uint8_t seed = 0;

    HEADER[0] = 'M';
    HEADER[1] = 'I';
    HEADER[2] = 'R';
    HEADER[3] = 'T';

    while (1) {
        seed++;

        HEADER[6] = STATE_FILLING;
        HEADER[4] = seed;
        HEADER[5] = (uint8_t)~seed;

        for (uint16_t addr = BLOCK_START; addr != BLOCK_END; addr++) {
            *(volatile uint8_t *)addr = pattern(addr, seed);
        }

        HEADER[6] = STATE_DONE;

        printf("seed %hx\n", seed);

        for (uint8_t frame = 0; frame < 60; frame++) {
            wait_vbl_done();
        }
    }
}
