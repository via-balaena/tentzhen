"""A UART: 8 data bits, no parity, one stop bit, least significant bit first.

`divisor` is the clock's cycles per bit: 217 for 115,200 baud from a 25 MHz clock. At that divisor
both halves echo on the i9 (measured: lab/records/2026-11-22-i9-uart-echo.toml), and at 434 from
the PLL's 50 MHz (measured: lab/records/2026-11-23-i9-pll-echo.toml). What the
docstrings below say is tested at a divisor of 8 by cocotb on Verilator, in tests/test_uart.py.
"""

from amaranth import Cat, Const, Elaboratable, Module, Signal
from amaranth.lib.cdc import FFSynchronizer

__all__ = ["UARTRx", "UARTTx"]


class UARTRx(Elaboratable):
    """Receives bytes from `rx`. `valid` is high for one cycle with each byte in `data`; a byte
    whose stop bit is low is dropped."""

    def __init__(self, divisor):
        self.divisor = divisor
        self.rx = Signal(init=1)
        self.data = Signal(8)
        self.valid = Signal()

    def elaborate(self, platform):
        m = Module()
        line = Signal(init=1)
        m.submodules.sync = FFSynchronizer(self.rx, line, init=1)
        timer = Signal(range(self.divisor))
        bit = Signal(range(8))
        shift = Signal(8)
        m.d.sync += self.valid.eq(0)
        with m.FSM():
            with m.State("idle"):
                with m.If(~line):
                    m.d.sync += timer.eq(self.divisor // 2 - 1)
                    m.next = "start"
            # Half a bit in, the middle of the start bit: still low, or it was a glitch.
            with m.State("start"):
                with m.If(timer != 0):
                    m.d.sync += timer.eq(timer - 1)
                with m.Elif(~line):
                    m.d.sync += [timer.eq(self.divisor - 1), bit.eq(0)]
                    m.next = "data"
                with m.Else():
                    m.next = "idle"
            # The middle of each data bit, a bit apart, each shifted in from the top.
            with m.State("data"):
                with m.If(timer != 0):
                    m.d.sync += timer.eq(timer - 1)
                with m.Else():
                    m.d.sync += [shift.eq(Cat(shift[1:], line)), timer.eq(self.divisor - 1)]
                    with m.If(bit == 7):
                        m.next = "stop"
                    with m.Else():
                        m.d.sync += bit.eq(bit + 1)
            with m.State("stop"):
                with m.If(timer != 0):
                    m.d.sync += timer.eq(timer - 1)
                with m.Else():
                    with m.If(line):
                        m.d.sync += [self.data.eq(shift), self.valid.eq(1)]
                    m.next = "idle"
        return m


class UARTTx(Elaboratable):
    """Sends `data` on `tx` when `start` is high while `ready` is: `ready` stays low until the
    byte's stop bit has lasted a full bit."""

    def __init__(self, divisor):
        self.divisor = divisor
        self.tx = Signal(init=1)
        self.data = Signal(8)
        self.start = Signal()
        self.ready = Signal()

    def elaborate(self, platform):
        m = Module()
        timer = Signal(range(self.divisor))
        # The start bit, the data and the stop bit, sent from bit 0 up.
        shift = Signal(10)
        left = Signal(range(11))
        m.d.comb += self.ready.eq((timer == 0) & (left == 0))
        with m.If(timer != 0):
            m.d.sync += timer.eq(timer - 1)
        with m.Elif(left != 0):
            m.d.sync += [
                self.tx.eq(shift[0]),
                shift.eq(shift >> 1),
                left.eq(left - 1),
                timer.eq(self.divisor - 1),
            ]
        with m.Elif(self.start):
            m.d.sync += [shift.eq(Cat(Const(0, 1), self.data, Const(1, 1))), left.eq(10)]
        return m
