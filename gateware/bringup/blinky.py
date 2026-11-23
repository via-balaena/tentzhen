"""Gate 0's first half: the i9's LED blinks, half a second on, half a second off
(docs/littlebit.md, "Milestones"). That it does is unknown until a person sees it on the board.

This builds the bitstream and nothing more. Loading it into the board is a flash, which a person
approves in the session and the lab log records (lab/targets.toml), so it is done by hand with
openFPGALoader, never from here. From the repo root:

    python -m gateware.bringup.blinky <build-dir>
"""

import sys

from amaranth import Elaboratable, Module, Signal

from gateware.boards.colorlight_i9 import ColorlightI9Platform

CLOCK_HZ = 25_000_000


class Blinky(Elaboratable):
    """Switches the LED on or off every half second of its clock, which runs at `clock_hz`: the
    25 MHz oscillator's, unless it is told otherwise."""

    def __init__(self, clock_hz=CLOCK_HZ):
        self.clock_hz = clock_hz

    def elaborate(self, platform):
        m = Module()
        led = platform.request("led", 0)
        half = self.clock_hz // 2
        count = Signal(range(half))
        lit = Signal()
        with m.If(count == half - 1):
            m.d.sync += [count.eq(0), lit.eq(~lit)]
        with m.Else():
            m.d.sync += count.eq(count + 1)
        m.d.comb += led.o.eq(lit)
        return m


if __name__ == "__main__":
    build_dir = sys.argv[1] if len(sys.argv) > 1 else "build"
    ColorlightI9Platform().build(Blinky(), name="blinky", build_dir=build_dir)
