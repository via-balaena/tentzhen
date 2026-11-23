"""The Colorlight i9 v7.2, for Amaranth: only what Gate 0 uses (docs/littlebit.md, "Hardware").

The balls and their I/O standard are LiteX's for the i9 v7.2 (litex-boards at 53dc2ed0,
litex_boards/platforms/colorlight_i5.py: clk25 and serial in _io_v7_0, the LED moved to L2 in
_io_v7_2), and Muse Lab's Amaranth platform gives the same for the clock and the LED
(wuxx/Colorlight-FPGA-Projects at 016e5889, src/amaranth_boards/colorlight_i9_r7_2.py). On our
board the clock and the LED are measured (lab/records/2026-11-22-i9-blinky.toml), and so is the
UART (lab/records/2026-11-22-i9-uart-echo.toml).
"""

from amaranth.build import Attrs, Clock, Pins, Resource
from amaranth.vendor import LatticePlatform
from amaranth_boards.resources import LEDResources, UARTResource

__all__ = ["ColorlightI9Platform"]


class ColorlightI9Platform(LatticePlatform):
    """An i9 v7.2, whose FPGA is an LFE5U-45F-6BG381C."""

    device = "LFE5U-45F"
    package = "BG381"
    speed = "6"
    default_clk = "clk25"

    resources = [
        # The 25 MHz oscillator.
        Resource("clk25", 0, Pins("P3", dir="i"), Clock(25e6), Attrs(IO_TYPE="LVCMOS33")),
        # The LED, lit when its ball is driven low.
        *LEDResources(pins="L2", invert=True, attrs=Attrs(IO_TYPE="LVCMOS33")),
        # The UART to the extension board's DAPLink: J17 out of the FPGA, H18 in.
        UARTResource(0, rx="H18", tx="J17", attrs=Attrs(IO_TYPE="LVCMOS33")),
    ]
    connectors = []

    @property
    def command_templates(self):
        # ecppack (Project Trellis 1.4-88-g2d12005, in OSS CAD Suite 2026-11-21) crashes on Windows
        # 11 with exit 0xC0000374, a heap corruption: both times it was asked for an SVF file too,
        # writing no bitstream, and without one in 4 of 7 runs from Amaranth's build script and 1
        # of 13 run by hand (2026-11-21 and 22). Each design's bitstreams that it did write had
        # one sha256. On 2026-11-23 it crashed in 3 of 6 builds from the script, each after writing
        # a .bit byte-identical to one measured before. openFPGALoader loads the .bit, so no SVF is
        # asked for. When a build crashes with no .bit written,
        # `ecppack --input <name>.config --bit <name>.bit` run by hand in its folder finishes it.
        templates = super().command_templates
        if self.toolchain != "Trellis":
            return templates
        return [t.replace("--svf {{name}}.svf", "") for t in templates]
