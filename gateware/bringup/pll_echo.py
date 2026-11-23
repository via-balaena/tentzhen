"""The PLL test, ahead of the Phase 2 layer engine (docs/littlebit.md, "The board"): Gate 0's echo
and blinky, clocked at 50 MHz from the ECP5's PLL instead of at 25 MHz from the oscillator.

The PLL's dividers are what `ecppll -i 25 -o 50` gave (gateware/cores/ecp5_pll.py): the 25 MHz
oscillator, divided by 1 and multiplied by 2, from a VCO at 600 MHz that CLKOP divides by 12. The
echo's divisor is then round(50 MHz / 115,200) = 434, and the blinky counts 25,000,000 cycles a
half second. Run from 25 MHz instead, the same design would blink once every two seconds and
echo at 57,600 baud, not 115,200. On i9-2 it echoes at 115,200 and not at 57,600, and blinks about
once a second: the PLL locks and the design runs near 50 MHz (measured:
lab/records/2026-11-23-i9-pll-echo.toml). How near is not measured.

This builds the bitstream and nothing more. Loading it is a flash, which a person approves in the
session and the lab log records (lab/targets.toml), done by hand with openFPGALoader. From the
repo root:

    python -m gateware.bringup.pll_echo <build-dir>
"""

import sys

from amaranth import ClockDomain, Elaboratable, Module

from gateware.boards.colorlight_i9 import ColorlightI9Platform
from gateware.bringup.uart_echo import UARTEcho
from gateware.cores.ecp5_pll import PLL

CLOCK_HZ = 50_000_000

# What `ecppll -i 25 -o <MHz>` writes (ecppll 1.4-88-g2d12005, OSS CAD Suite 2026-11-21) for each
# whole number of hertz from 50 to 80 MHz that the PLL can make from the 25 MHz oscillator: the
# CLKI, CLKFB and CLKOP dividers, and CLKOP's coarse phase. 50 MHz is the board's clock. The rest
# are for E12, the clock push (docs/littlebit.md, "E12, the highest clock that closes"), and no
# board has run them.
ECPPLL = {
    50_000_000: (1, 2, 12, 5),
    53_125_000: (8, 17, 11, 5),
    55_000_000: (5, 11, 11, 5),
    56_250_000: (4, 9, 11, 5),
    59_375_000: (8, 19, 10, 4),
    60_000_000: (5, 12, 10, 4),
    62_500_000: (2, 5, 10, 4),
    65_000_000: (5, 13, 9, 4),
    65_625_000: (8, 21, 9, 4),
    68_750_000: (4, 11, 9, 4),
    70_000_000: (5, 14, 9, 4),
    71_875_000: (8, 23, 8, 3),
    75_000_000: (1, 3, 8, 4),
    78_125_000: (8, 25, 8, 4),
    80_000_000: (5, 16, 7, 3),
}


def clock(m, platform, hz):
    """Gives module `m` the `sync` domain at `hz`, from the PLL fed by the board's 25 MHz
    oscillator, with the dividers ecppll gave for `hz`."""
    m.domains.sync = ClockDomain("sync")
    clk25 = platform.request(platform.default_clk, 0)
    f_in = int(platform.default_clk_frequency)
    clki_div, clkfb_div, clkop_div, clkop_cphase = ECPPLL[hz]
    m.submodules.pll = pll = PLL(
        clk25.i, f_in, clki_div=clki_div, clkfb_div=clkfb_div, clkop_div=clkop_div,
        clkop_cphase=clkop_cphase,
    )
    assert pll.f_out_hz == hz, f"the PLL gives {pll.f_out_hz} Hz, not {hz}"


def clock_50_mhz(m, platform):
    """Gives module `m` the `sync` domain at 50 MHz, from the PLL fed by the board's 25 MHz
    oscillator, as this design runs it."""
    clock(m, platform, CLOCK_HZ)


class PLLEcho(Elaboratable):
    """The echo and the blinky, in a 50 MHz domain made by the PLL from the 25 MHz oscillator."""

    def elaborate(self, platform):
        m = Module()
        clock_50_mhz(m, platform)
        m.submodules.echo = UARTEcho(CLOCK_HZ)
        return m


if __name__ == "__main__":
    build_dir = sys.argv[1] if len(sys.argv) > 1 else "build"
    ColorlightI9Platform().build(PLLEcho(), name="pll_echo", build_dir=build_dir)
