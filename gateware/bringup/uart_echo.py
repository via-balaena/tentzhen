"""Gate 0's second half: bytes from the computer echo back over the extension board's DAPLink at
115,200 baud, while the LED blinks as the blinky's does (docs/littlebit.md, "Milestones"). It
does: measured, lab/records/2026-11-22-i9-uart-echo.toml.

Each byte received goes into a 16-byte FIFO and is sent back as soon as the transmitter is free, so
bytes that arrive back to back are not dropped while one is still going out.

This builds the bitstream and nothing more. Loading it is a flash, which a person approves in the
session and the lab log records (lab/targets.toml), done by hand with openFPGALoader. From the
repo root:

    python -m gateware.bringup.uart_echo <build-dir>
"""

import sys

from amaranth import Elaboratable, Module
from amaranth.lib.fifo import SyncFIFO

from gateware.boards.colorlight_i9 import ColorlightI9Platform
from gateware.bringup.blinky import CLOCK_HZ, Blinky
from gateware.cores.uart import UARTRx, UARTTx

BAUD = 115_200


class UARTEcho(Elaboratable):
    """Sends back every byte it receives, and blinks the LED, from a clock at `clock_hz`: the 25 MHz
    oscillator's, unless it is told otherwise."""

    def __init__(self, clock_hz=CLOCK_HZ):
        self.clock_hz = clock_hz

    def elaborate(self, platform):
        m = Module()
        uart = platform.request("uart", 0)
        divisor = round(self.clock_hz / BAUD)
        m.submodules.blinky = Blinky(self.clock_hz)
        m.submodules.rx = rx = UARTRx(divisor)
        m.submodules.tx = tx = UARTTx(divisor)
        m.submodules.fifo = fifo = SyncFIFO(width=8, depth=16)
        m.d.comb += [
            rx.rx.eq(uart.rx.i),
            uart.tx.o.eq(tx.tx),
            fifo.w_data.eq(rx.data),
            fifo.w_en.eq(rx.valid),
            tx.data.eq(fifo.r_data),
            tx.start.eq(fifo.r_rdy),
            fifo.r_en.eq(tx.ready),
        ]
        return m


if __name__ == "__main__":
    build_dir = sys.argv[1] if len(sys.argv) > 1 else "build"
    ColorlightI9Platform().build(UARTEcho(), name="uart_echo", build_dir=build_dir)
