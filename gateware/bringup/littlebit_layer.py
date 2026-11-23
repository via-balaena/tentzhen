"""One LittleBit layer on the board, ahead of Gate 2 (docs/littlebit.md, "Milestones"): r16's
first layer, run by the layer engine (gateware/cores/littlebit.py) at 50 MHz from the PLL, fed and
read over the UART at 115,200 baud. The LED blinks as the blinky's does, at 50 MHz.

The computer sends a frame: the byte 0xA5, then the layer's inputs, one byte each, 784 for r16's
first layer (each a pixel shifted right by the network's pixel shift). The design answers 0x5A,
then for each of the layer's outputs, in order, y, one byte, and z, the int32 before the shift and
clamp, four bytes, least significant first: 1 + 256 x 5 = 1,281 bytes. The outputs wait in a FIFO
as long as the layer, since the engine makes them far faster than the UART sends them.

Bytes before a 0xA5 are dropped. A frame that stops part way, nothing arriving for 100 ms, is
dropped too, and the engine reset with it, so the next 0xA5 starts afresh. The computer sends a
frame only once the last one's answer is in.

On i9-2 it answered 1,000 MNIST test images and the layer's 6 test vectors exactly as the golden
model, every y and z (measured: lab/records/2026-11-23-i9-littlebit-layer.toml).

This builds the bitstream and nothing more. Loading it is a flash, which a person approves in the
session and the lab log records (lab/targets.toml), done by hand with openFPGALoader. From the
repo root:

    python -m gateware.bringup.littlebit_layer <build-dir>
"""

import json
import sys
from pathlib import Path

from amaranth import Cat, Elaboratable, Module, ResetInserter, Signal
from amaranth.lib.fifo import SyncFIFOBuffered

from gateware.boards.colorlight_i9 import ColorlightI9Platform
from gateware.bringup.blinky import Blinky
from gateware.bringup.pll_echo import CLOCK_HZ, clock_50_mhz
from gateware.bringup.uart_echo import BAUD
from gateware.cores.littlebit import LittleBitEngine
from gateware.cores.uart import UARTRx, UARTTx

__all__ = ["FRAME", "ANSWER", "LayerOverUART", "LayerBoard", "r16_layer"]

FRAME = 0xA5
ANSWER = 0x5A
NETWORK = Path(__file__).resolve().parents[2] / "models/littlebit/runs/r16/integer.json"


def r16_layer(index=0):
    """A hidden layer of r16's integer network, as integer.json holds it."""
    return json.loads(NETWORK.read_text())["layers"][index]


class LayerOverUART(Elaboratable):
    """`layer`'s engine behind the UART, from a clock of `clock_hz`. `divisor` is the UART's
    cycles a bit, and `quiet` the cycles without a byte after which a frame is dropped; each
    follows from the clock unless given, as the tests give small ones."""

    def __init__(self, layer, clock_hz, divisor=None, quiet=None):
        self.layer = layer
        self.clock_hz = clock_hz
        self.divisor = divisor if divisor is not None else round(clock_hz / BAUD)
        self.quiet = quiet if quiet is not None else clock_hz // 10

    def elaborate(self, platform):
        m = Module()
        uart = platform.request("uart", 0)
        n_in, n_out = self.layer["inputs"], self.layer["outputs"]
        m.submodules.blinky = Blinky(self.clock_hz)
        m.submodules.rx = rx = UARTRx(self.divisor)
        m.submodules.tx = tx = UARTTx(self.divisor)
        m.d.comb += [rx.rx.eq(uart.rx.i), uart.tx.o.eq(tx.tx)]

        # The engine, reset with a dropped frame; its outputs, y and z, wait in the FIFO.
        drop = Signal()
        m.submodules.engine = engine = ResetInserter(drop)(LittleBitEngine(self.layer))
        m.submodules.fifo = fifo = SyncFIFOBuffered(width=40, depth=n_out)
        m.d.comb += [
            fifo.w_data.eq(Cat(engine.y, engine.z)),
            fifo.w_en.eq(engine.y_valid),
        ]

        # Receiving: a frame's bytes go to the engine one at a time, each held until it is taken.
        held = Signal(8)
        holding = Signal()
        count = Signal(range(n_in))
        silent = Signal(range(self.quiet + 1))
        m.d.comb += [engine.x.eq(held), engine.x_valid.eq(holding)]
        with m.If(holding & engine.x_ready):
            m.d.sync += holding.eq(0)
        with m.FSM(name="receive"):
            with m.State("wait"):
                with m.If(rx.valid & (rx.data == FRAME)):
                    m.d.sync += [count.eq(0), silent.eq(0)]
                    m.next = "frame"
            with m.State("frame"):
                with m.If(rx.valid):
                    m.d.sync += [held.eq(rx.data), holding.eq(1), silent.eq(0)]
                    with m.If(count == n_in - 1):
                        m.next = "wait"
                    with m.Else():
                        m.d.sync += count.eq(count + 1)
                with m.Elif(silent == self.quiet):
                    m.d.comb += drop.eq(1)
                    m.d.sync += holding.eq(0)
                    m.next = "wait"
                with m.Else():
                    m.d.sync += silent.eq(silent + 1)

        # Sending: 0x5A, then each output's five bytes as it comes out of the FIFO.
        byte = Signal(range(5))
        sent = Signal(range(n_out))
        entry = fifo.r_data
        with m.FSM(name="send"):
            with m.State("idle"):
                with m.If(fifo.r_rdy & tx.ready):
                    m.d.comb += [tx.data.eq(ANSWER), tx.start.eq(1)]
                    m.d.sync += [byte.eq(0), sent.eq(0)]
                    m.next = "outputs"
            with m.State("outputs"):
                with m.If(fifo.r_rdy & tx.ready):
                    m.d.comb += [tx.data.eq(entry.word_select(byte, 8)), tx.start.eq(1)]
                    with m.If(byte == 4):
                        m.d.comb += fifo.r_en.eq(1)
                        m.d.sync += [byte.eq(0), sent.eq(sent + 1)]
                        with m.If(sent == n_out - 1):
                            m.next = "idle"
                    with m.Else():
                        m.d.sync += byte.eq(byte + 1)
        return m


class LayerBoard(Elaboratable):
    """r16's first layer over the UART, at 50 MHz from the PLL: what the board runs."""

    def elaborate(self, platform):
        m = Module()
        clock_50_mhz(m, platform)
        m.submodules.layer = LayerOverUART(r16_layer(0), CLOCK_HZ)
        return m


if __name__ == "__main__":
    build_dir = sys.argv[1] if len(sys.argv) > 1 else "build"
    ColorlightI9Platform().build(LayerBoard(), name="littlebit_layer", build_dir=build_dir)
