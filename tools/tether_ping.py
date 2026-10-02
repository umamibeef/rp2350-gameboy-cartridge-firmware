#!/usr/bin/env python3
"""Poll the cartridge tether over its USB serial port.

Sends the status command once a second and prints the answer, so it is easy to
see whether the port keeps answering while a game is running.

    pip install pyserial
    python3 tools/tether_ping.py /dev/cu.usbmodemXXXX
"""

import sys
import time

import serial


def ask(port, command):
    port.reset_input_buffer()
    port.write(command)
    return port.readline().decode(errors="replace").strip()


def main():
    if len(sys.argv) != 2:
        sys.exit(f"usage: {sys.argv[0]} <serial port>")

    with serial.Serial(sys.argv[1], timeout=1) as port:
        print("version:", ask(port, b"v") or "<no answer>")

        sent = answered = 0
        while True:
            sent += 1
            state = ask(port, b"s")
            if state:
                answered += 1
            print(f"{time.strftime('%H:%M:%S')}  state={state or '<no answer>'}  "
                  f"answered {answered}/{sent}")
            time.sleep(1)


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        pass
