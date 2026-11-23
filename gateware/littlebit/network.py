"""The whole network on the board, Gate 2's top level (docs/littlebit.md, "Hardware
architecture"): r16, or another run's integer network, its integers hardwired into logic
(Variant A) or kept in block RAM (Variant B), run by `LittleBitNetwork`
(gateware/cores/littlebit.py) at 50 MHz from the PLL, fed and read over the UART at 115,200 baud.
The LED blinks as the blinky's does, at 50 MHz. E12, the clock push, builds it at the PLL's other
clocks.

The computer sends a frame: the byte 0xA5, then an image's 784 pixels, one byte each, as MNIST
stores them; the network shifts them itself. The design answers 0x5A; the 10 scores, each an
int32, least significant byte first; the digit, one byte; and the clocks the network took, a
uint32, least significant byte first: 1 + 40 + 1 + 4 = 46 bytes.

The pixels wait in a FIFO until the frame's last arrives, then go into the network back to back,
so the clocks it reports are the network's alone, not the serial link's: from the clock that took
the first pixel to the one that raised its answer, both counted. The answer is copied out of the
network when it is ready, and sent from the copy.

Bytes before a 0xA5 are dropped. A frame that stops part way, nothing arriving for 100 ms, is
dropped too, its pixels with it, so the next 0xA5 starts afresh. The computer sends a frame only
once the last one's answer is in: a frame's 784 bytes take 68 ms at 115,200 baud, and the network
answers in tens of microseconds, so a frame never waits on the one before.

This builds the bitstream and nothing more. Loading it is a flash, which a person approves in the
session and the lab log records (lab/targets.toml), done by hand with openFPGALoader. From the
repo root:

    python -m gateware.littlebit.network [--variant B] [--clock-mhz <MHz>] <build-dir> \
        [<integer.json>]

The network is r16's, models/littlebit/runs/r16/integer.json, unless another run's file is named:
the rank sweep's are built the same way, for their LUTs, block RAM and clock (docs/littlebit.md,
"Experiments"). It is Variant A unless `--variant B` is given, and runs at 50 MHz unless
`--clock-mhz` names another of the clocks the PLL makes (`ECPPLL` in
gateware/bringup/pll_echo.py). The UART's divisor, the blinky's count and the 100 ms of silence
that drop a frame follow from the clock.
"""

import argparse
import json
from decimal import Decimal
from pathlib import Path

from amaranth import Cat, Const, Elaboratable, Module, ResetInserter, Signal
from amaranth.lib.fifo import SyncFIFOBuffered

from gateware.boards.colorlight_i9 import ColorlightI9Platform
from gateware.bringup.blinky import Blinky
from gateware.bringup.littlebit_layer import ANSWER, FRAME
from gateware.bringup.pll_echo import CLOCK_HZ, ECPPLL, clock
from gateware.bringup.uart_echo import BAUD
from gateware.cores.littlebit import VARIANTS, LittleBitNetwork
from gateware.cores.uart import UARTRx, UARTTx

__all__ = ["FRAME", "ANSWER", "PIXELS", "NetworkOverUART", "NetworkBoard", "r16", "read_network"]

PIXELS = 784
NETWORK = Path(__file__).resolve().parents[2] / "models/littlebit/runs/r16/integer.json"


def r16():
    """r16's integer network, as integer.json holds it."""
    return read_network(NETWORK)


def read_network(path):
    """The integer network at `path`, as integer.json holds it."""
    return json.loads(Path(path).read_text())


class NetworkOverUART(Elaboratable):
    """`network`'s `LittleBitNetwork` behind the UART, from a clock of `clock_hz`. `divisor` is
    the UART's cycles a bit, and `quiet` the cycles without a byte after which a frame is dropped;
    each follows from the clock unless given, as the tests give small ones. `variant` is the
    network's, "A" or "B"."""

    def __init__(self, network, clock_hz, divisor=None, quiet=None, variant="A"):
        self.network = network
        self.variant = variant
        self.clock_hz = clock_hz
        self.divisor = divisor if divisor is not None else round(clock_hz / BAUD)
        self.quiet = quiet if quiet is not None else clock_hz // 10

    def elaborate(self, platform):
        m = Module()
        uart = platform.request("uart", 0)
        m.submodules.blinky = Blinky(self.clock_hz)
        m.submodules.rx = rx = UARTRx(self.divisor)
        m.submodules.tx = tx = UARTTx(self.divisor)
        m.d.comb += [rx.rx.eq(uart.rx.i), uart.tx.o.eq(tx.tx)]
        m.submodules.network = net = LittleBitNetwork(self.network, self.variant)

        # The frame's pixels, emptied with a dropped frame.
        drop = Signal()
        m.submodules.pixels = pixels = ResetInserter(drop)(SyncFIFOBuffered(width=8, depth=PIXELS))

        # Receiving: a frame's bytes into the FIFO, and once the last is in, the FIFO into the
        # network, until it answers.
        count = Signal(range(PIXELS))
        silent = Signal(range(self.quiet + 1))
        feeding = Signal()
        m.d.comb += [
            pixels.w_data.eq(rx.data),
            net.x.eq(pixels.r_data),
            net.x_valid.eq(feeding & pixels.r_rdy),
            pixels.r_en.eq(feeding & net.x_ready),
        ]
        with m.If(net.done):
            m.d.sync += feeding.eq(0)
        with m.FSM(name="receive"):
            with m.State("wait"):
                with m.If(rx.valid & (rx.data == FRAME)):
                    m.d.sync += [count.eq(0), silent.eq(0)]
                    m.next = "frame"
            with m.State("frame"):
                with m.If(rx.valid):
                    m.d.comb += pixels.w_en.eq(1)
                    m.d.sync += silent.eq(0)
                    with m.If(count == PIXELS - 1):
                        m.d.sync += feeding.eq(1)
                        m.next = "wait"
                    with m.Else():
                        m.d.sync += count.eq(count + 1)
                with m.Elif(silent == self.quiet):
                    m.d.comb += drop.eq(1)
                    m.next = "wait"
                with m.Else():
                    m.d.sync += silent.eq(silent + 1)

        # Sending: the answer, copied when the network raises it, a byte at a time from the
        # bottom of the copy.
        n_out = len(net.scores)
        assert len(net.digit) <= 8, f"{n_out} outputs: a digit takes more than a byte"
        answer = Cat(
            Const(ANSWER, 8), *net.scores, net.digit, Const(0, 8 - len(net.digit)), net.cycles
        )
        n_bytes = 1 + 4 * n_out + 1 + 4
        assert len(answer) == 8 * n_bytes
        copy = Signal(len(answer))
        left = Signal(range(n_bytes + 1))
        with m.If(net.done):
            m.d.sync += [copy.eq(answer), left.eq(n_bytes)]
        with m.Elif((left != 0) & tx.ready):
            m.d.comb += [tx.data.eq(copy[:8]), tx.start.eq(1)]
            m.d.sync += [copy.eq(copy >> 8), left.eq(left - 1)]
        return m


class NetworkBoard(Elaboratable):
    """A network over the UART, at `clock_hz` from the PLL, 50 MHz unless told otherwise: what the
    board runs. The network is the integer.json at `path`, r16's unless another is given, in
    `variant`, "A" or "B"."""

    def __init__(self, path=NETWORK, variant="A", clock_hz=CLOCK_HZ):
        if clock_hz not in ECPPLL:
            raise ValueError(f"the PLL has no dividers here for {clock_hz} Hz")
        self.path = path
        self.variant = variant
        self.clock_hz = clock_hz

    def elaborate(self, platform):
        m = Module()
        clock(m, platform, self.clock_hz)
        network = read_network(self.path)
        m.submodules.network = NetworkOverUART(network, self.clock_hz, variant=self.variant)
        return m


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        prog="python -m gateware.littlebit.network",
        description="Builds the network's bitstream for the Colorlight i9.",
    )
    parser.add_argument("build_dir", nargs="?", default="build")
    parser.add_argument("network", nargs="?", default=NETWORK, help="an integer.json; r16's if none")
    parser.add_argument("--variant", choices=VARIANTS, default="A")
    parser.add_argument("--clock-mhz", choices=[f"{hz / 1e6:g}" for hz in ECPPLL], default="50")
    args = parser.parse_args()
    clock_hz = int(Decimal(args.clock_mhz) * 1_000_000)
    ColorlightI9Platform().build(
        NetworkBoard(args.network, args.variant, clock_hz), name="littlebit_network",
        build_dir=args.build_dir,
    )
