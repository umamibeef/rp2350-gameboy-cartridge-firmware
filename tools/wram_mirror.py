#!/usr/bin/env python3
"""Read the work RAM mirror of the cartridge over its USB serial port.

    pip install pyserial
    python3 tools/wram_mirror.py PORT dump [FILE]   hexdump 0xC000-0xDFFF, or save it to FILE
    python3 tools/wram_mirror.py PORT watch         print the bytes that change between reads
    python3 tools/wram_mirror.py PORT verify        check the mirror against the test ROM

The test ROM is in tools/mirror_test_rom. Run verify while it is the game on the cartridge.
"""

import sys
import time

import serial

WRAM_START = 0xC000
WRAM_SIZE = 0x2000

# Layout written by tools/mirror_test_rom/mirror_test.c
TEST_HEADER = 0xC3F0
TEST_BLOCK_START = 0xC400
TEST_BLOCK_END = 0xDC00
TEST_STATE_DONE = 0xA5


def ask_line(port, command):
    port.reset_input_buffer()
    port.write(command)
    return port.readline().decode(errors="replace").strip()


def read_wram(port):
    port.reset_input_buffer()
    port.write(b"w")
    data = port.read(WRAM_SIZE)
    if len(data) != WRAM_SIZE:
        raise IOError(f"expected {WRAM_SIZE} bytes, got {len(data)}")
    return data


def hexdump(data, base):
    for offset in range(0, len(data), 16):
        row = data[offset:offset + 16]
        print(f"{base + offset:04X}  {row.hex(' ')}")


def test_pattern(addr, seed):
    lo = addr & 0xFF
    hi = (addr >> 8) & 0xFF
    return (lo ^ ((hi * 7) & 0xFF) ^ seed) & 0xFF


def verify_once(wram):
    """Returns (seed, mismatching addresses), or None if the test ROM is in the middle of a fill."""

    def at(addr):
        return wram[addr - WRAM_START]

    header = bytes(at(TEST_HEADER + i) for i in range(7))
    if header[:4] != b"MIRT":
        raise ValueError(f"no test ROM header at {TEST_HEADER:04X}: {header.hex(' ')}")
    seed, seed_inverted, state = header[4], header[5], header[6]
    if seed ^ seed_inverted != 0xFF or state != TEST_STATE_DONE:
        return None

    bad = [
        addr for addr in range(TEST_BLOCK_START, TEST_BLOCK_END)
        if at(addr) != test_pattern(addr, seed)
    ]
    return seed, bad


def verify(port, rounds=20):
    checked = failed = 0
    seeds = set()
    while checked < rounds:
        result = verify_once(read_wram(port))
        if result is None:
            time.sleep(0.1)
            continue
        seed, bad = result
        checked += 1
        seeds.add(seed)
        if bad:
            failed += 1
            shown = ", ".join(f"{a:04X}" for a in bad[:8])
            print(f"seed {seed:02X}: {len(bad)} wrong bytes, first at {shown}")
        else:
            print(f"seed {seed:02X}: all {TEST_BLOCK_END - TEST_BLOCK_START} bytes match")
        time.sleep(0.3)

    print(ask_line(port, b"d"))
    print(f"{checked - failed}/{checked} snapshots correct, {len(seeds)} different seeds seen")
    return failed == 0


def watch(port):
    previous = read_wram(port)
    while True:
        time.sleep(0.5)
        current = read_wram(port)
        changed = [i for i in range(WRAM_SIZE) if current[i] != previous[i]]
        if changed:
            shown = " ".join(f"{WRAM_START + i:04X}={current[i]:02X}" for i in changed[:12])
            more = f" (+{len(changed) - 12} more)" if len(changed) > 12 else ""
            print(f"{time.strftime('%H:%M:%S')}  {shown}{more}")
        previous = current


def main():
    if len(sys.argv) < 3 or sys.argv[2] not in ("dump", "watch", "verify"):
        sys.exit(__doc__)

    with serial.Serial(sys.argv[1], timeout=2) as port:
        version = ask_line(port, b"v")
        words = version.split()
        if words[:2] != ["croco-v2", "tether"] or not words[-1].isdigit() or int(words[-1]) < 2:
            sys.exit(f"firmware without work RAM mirror: {version or '<no answer>'}")

        command = sys.argv[2]
        if command == "dump":
            wram = read_wram(port)
            if len(sys.argv) > 3:
                with open(sys.argv[3], "wb") as f:
                    f.write(wram)
            else:
                hexdump(wram, WRAM_START)
        elif command == "watch":
            watch(port)
        elif command == "verify":
            sys.exit(0 if verify(port) else 1)


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        pass
