"""The ECP5's PLL, EHXPLLL, making one clock domain from an input clock: its primary output, CLKOP,
fed back to itself.

The dividers are given, not searched for. Project Trellis's `ecppll` finds them, and the same
suite carries it (OSS CAD Suite 2026-11-21: ecppll 1.4-88-g2d12005). `ecppll -i 25 -o 50` gives
the ones `gateware/bringup/pll_echo.py` uses. The frequencies follow from the dividers as ecppll's
source works them out (libtrellis/tools/ecppll.cpp at 2d12005, its opening comment, from runs of
Lattice's Clarity Designer):

    f_pfd = f_in / clki_div,  f_out = f_pfd * clkfb_div,  f_vco = f_out * clkop_div

`PLL` refuses dividers or frequencies outside the ranges that file uses, in MHz: input 8 to 400,
PFD 3.125 to 400, VCO 400 to 800, output 10 to 400; CLKI and CLKOP dividers 1 to 128, and CLKFB
1 to 80. It also refuses an output that isn't a whole number of hertz, since a design counts its
cycles from it. Both refusals are `tested`: tests/test_ecp5_pll.py. Set as pll_echo.py sets it, the
PLL locks on the i9 and its domain runs near 50 MHz (measured:
lab/records/2026-11-23-i9-pll-echo.toml). Other settings are `unknown` until a board runs them.

`PLL` drives the domain's clock and reset; a module above it defines the domain, since Amaranth
0.5.10 passes a domain down to submodules only, never up (`Fragment._propagate_domains`, in
amaranth/hdl/_ir.py). The domain is held in reset until the PLL reports lock, and leaves reset on
its own clock's edge (`amaranth.lib.cdc.ResetSynchronizer`).
"""

from fractions import Fraction

from amaranth import ClockSignal, Elaboratable, Instance, Module, Signal
from amaranth.lib.cdc import ResetSynchronizer

__all__ = ["PLL"]

MHZ = 1_000_000

# ecppll.cpp at 2d12005: INPUT_, PFD_, VCO_ and OUTPUT_MIN and _MAX, and its divider loops.
INPUT_HZ = (8 * MHZ, 400 * MHZ)
PFD_HZ = (Fraction(3_125_000), 400 * MHZ)
VCO_HZ = (400 * MHZ, 800 * MHZ)
OUTPUT_HZ = (10 * MHZ, 400 * MHZ)
CLKI_DIV = (1, 128)
CLKFB_DIV = (1, 80)
CLKOP_DIV = (1, 128)


def _mhz(hz):
    """`hz` in MHz, as ecppll writes it in an attribute: "25", or "12.5"."""
    f = Fraction(hz, MHZ)
    return str(f.numerator) if f.denominator == 1 else str(float(f))


def _within(name, value, bounds, unit=1, units=""):
    low, high = bounds
    if not low <= value <= high:
        show = lambda v: f"{float(v / unit):g}"
        raise ValueError(
            f"the PLL's {name}, {show(value)}{units}, is outside {show(low)} to {show(high)}{units}"
        )


class PLL(Elaboratable):
    """Clock domain `domain`'s clock and reset from `clk_in`, a clock of `f_in_hz`, through one
    EHXPLLL.

    `f_out_hz` and `f_vco_hz` are what the dividers give. `clkop_cphase` is CLKOP's coarse phase,
    in VCO cycles; with CLKOP as its own feedback, it is ecppll's value for these dividers."""

    def __init__(self, clk_in, f_in_hz, *, clki_div, clkfb_div, clkop_div, clkop_cphase,
                 domain="sync"):
        _within("input", f_in_hz, INPUT_HZ, MHZ, " MHz")
        _within("CLKI divider", clki_div, CLKI_DIV)
        _within("CLKFB divider", clkfb_div, CLKFB_DIV)
        _within("CLKOP divider", clkop_div, CLKOP_DIV)
        f_pfd = Fraction(f_in_hz, clki_div)
        _within("PFD", f_pfd, PFD_HZ, MHZ, " MHz")
        f_out = f_pfd * clkfb_div
        _within("output", f_out, OUTPUT_HZ, MHZ, " MHz")
        _within("VCO", f_out * clkop_div, VCO_HZ, MHZ, " MHz")
        if f_out.denominator != 1:
            raise ValueError(f"the PLL's output, {float(f_out)} Hz, is not a whole number of Hz")
        self.clk_in = clk_in
        self.f_in_hz = f_in_hz
        self.f_out_hz = int(f_out)
        self.f_vco_hz = int(f_out * clkop_div)
        self.clki_div = clki_div
        self.clkfb_div = clkfb_div
        self.clkop_div = clkop_div
        self.clkop_cphase = clkop_cphase
        self.domain = domain
        self.locked = Signal()

    def elaborate(self, platform):
        m = Module()
        clk = Signal()
        # As ecppll writes it: the same attributes, parameters and tied inputs.
        m.submodules.pll = Instance(
            "EHXPLLL",
            a_FREQUENCY_PIN_CLKI=_mhz(self.f_in_hz),
            a_FREQUENCY_PIN_CLKOP=_mhz(self.f_out_hz),
            a_ICP_CURRENT="12",
            a_LPF_RESISTOR="8",
            a_MFG_ENABLE_FILTEROPAMP="1",
            a_MFG_GMCREF_SEL="2",
            p_PLLRST_ENA="DISABLED",
            p_INTFB_WAKE="DISABLED",
            p_STDBY_ENABLE="DISABLED",
            p_DPHASE_SOURCE="DISABLED",
            p_OUTDIVIDER_MUXA="DIVA",
            p_OUTDIVIDER_MUXB="DIVB",
            p_OUTDIVIDER_MUXC="DIVC",
            p_OUTDIVIDER_MUXD="DIVD",
            p_CLKI_DIV=self.clki_div,
            p_CLKOP_ENABLE="ENABLED",
            p_CLKOP_DIV=self.clkop_div,
            p_CLKOP_CPHASE=self.clkop_cphase,
            p_CLKOP_FPHASE=0,
            p_FEEDBK_PATH="CLKOP",
            p_CLKFB_DIV=self.clkfb_div,
            i_RST=0,
            i_STDBY=0,
            i_CLKI=self.clk_in,
            o_CLKOP=clk,
            i_CLKFB=clk,
            i_PHASESEL0=0,
            i_PHASESEL1=0,
            i_PHASEDIR=1,
            i_PHASESTEP=1,
            i_PHASELOADREG=1,
            i_PLLWAKESYNC=0,
            i_ENCLKOP=0,
            o_LOCK=self.locked,
        )
        m.d.comb += ClockSignal(self.domain).eq(clk)
        m.submodules.reset = ResetSynchronizer(~self.locked, domain=self.domain)
        return m
