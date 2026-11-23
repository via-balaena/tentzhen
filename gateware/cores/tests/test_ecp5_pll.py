# amaranth: UnusedElaboratable=no
"""Tests of gateware/cores/ecp5_pll.py's arithmetic and refusals, by pytest, with no simulator: the
PLL itself is the chip's, so what it does is for the board to show. From the repo root, in OSS CAD
Suite's environment:

    python3 -m pytest gateware
"""

from types import SimpleNamespace

import pytest
from amaranth import Signal
from amaranth.back import rtlil

from gateware.bringup.pll_echo import CLOCK_HZ, ECPPLL, PLLEcho
from gateware.cores.ecp5_pll import PLL

MHZ = 1_000_000


def pll(**dividers):
    settings = dict(clki_div=1, clkfb_div=2, clkop_div=12, clkop_cphase=5)
    settings.update(dividers)
    return PLL(Signal(), 25 * MHZ, **settings)


def test_ecppll_dividers_give_50_mhz_from_a_600_mhz_vco():
    """What `ecppll -i 25 -o 50` printed (ecppll 1.4-88-g2d12005): 50 MHz out, VCO 600 MHz."""
    p = pll()
    assert (p.f_out_hz, p.f_vco_hz) == (50 * MHZ, 600 * MHZ)


@pytest.mark.parametrize("hz", ECPPLL)
def test_each_clock_s_ecppll_dividers_give_it(hz):
    """Each clock pll_echo.py has ecppll's dividers for comes out of them at exactly that many
    hertz, with the PFD, VCO and dividers in the ranges ecppll uses, or `PLL` refuses them."""
    clki_div, clkfb_div, clkop_div, clkop_cphase = ECPPLL[hz]
    p = pll(clki_div=clki_div, clkfb_div=clkfb_div, clkop_div=clkop_div, clkop_cphase=clkop_cphase)
    assert p.f_out_hz == hz


@pytest.mark.parametrize(
    "dividers, refused",
    [
        (dict(clkop_div=6), "VCO"),  # 50 MHz x 6 = 300 MHz, under 400
        (dict(clkop_div=17), "VCO"),  # 850 MHz, over 800
        (dict(clki_div=9, clkfb_div=18), "PFD"),  # 25 / 9 = 2.78 MHz, under 3.125
        (dict(clkfb_div=81, clki_div=81, clkop_div=24), "CLKFB divider"),  # over 80
        (dict(clki_div=129), "CLKI divider"),
        (dict(clkop_div=0), "CLKOP divider"),
        (dict(clki_div=4, clkfb_div=1, clkop_div=80), "output"),  # 6.25 MHz, under 10
        (dict(clki_div=7, clkfb_div=3, clkop_div=40), "whole number"),  # 10.714... MHz
    ],
)
def test_pll_refuses_what_ecppll_would_not_use(dividers, refused):
    with pytest.raises(ValueError, match=refused):
        pll(**dividers)


def test_pll_refuses_an_input_ecppll_would_not_take():
    with pytest.raises(ValueError, match="input"):
        PLL(Signal(), 7 * MHZ, clki_div=1, clkfb_div=7, clkop_div=12, clkop_cphase=5)


def test_the_pll_echo_elaborates_at_50_mhz():
    """The board's design, with a stand-in platform: its PLL gives the 50 MHz its echo and blinky
    count from, or it refuses to elaborate."""
    pins = {
        "clk25": SimpleNamespace(i=Signal(name="clk25")),
        "uart": SimpleNamespace(rx=SimpleNamespace(i=Signal()), tx=SimpleNamespace(o=Signal())),
        "led": SimpleNamespace(o=Signal()),
    }
    platform = SimpleNamespace(
        default_clk="clk25",
        default_clk_frequency=25e6,
        request=lambda name, number: pins[name],
    )
    ports = [
        pins["clk25"].i,
        pins["uart"].rx.i,
        pins["uart"].tx.o,
        pins["led"].o,
    ]
    assert CLOCK_HZ == 50 * MHZ
    assert "cell \\EHXPLLL" in rtlil.convert(PLLEcho(), platform=platform, ports=ports)
