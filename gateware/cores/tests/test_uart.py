"""cocotb tests of gateware/cores/uart.py, on Verilator, run by pytest (docs/littlebit.md,
"Verification": a unit test per block, the UART loopback first). From the repo root, in OSS CAD
Suite's environment:

    python3 -m pytest gateware

The cores run at a divisor of 8 here, to keep each test short. A test drives the design's inputs and
reads its outputs at the clock's falling edge, half a cycle from the rising edge the design acts on.
"""

import cocotb
from amaranth import Elaboratable, Module, Signal
from cocotb.clock import Clock
from cocotb.triggers import FallingEdge

from gateware.cores.uart import UARTRx, UARTTx
from gateware.sim import run

DIVISOR = 8


class Pair(Elaboratable):
    """A transmitter and a receiver side by side, each line a port, so a test can join them or
    drive the receiver's line itself."""

    def __init__(self):
        self.tx_data = Signal(8)
        self.tx_start = Signal()
        self.tx_ready = Signal()
        self.tx_line = Signal()
        self.rx_line = Signal()
        self.rx_data = Signal(8)
        self.rx_valid = Signal()

    def elaborate(self, platform):
        m = Module()
        m.submodules.tx = tx = UARTTx(DIVISOR)
        m.submodules.rx = rx = UARTRx(DIVISOR)
        m.d.comb += [
            tx.data.eq(self.tx_data),
            tx.start.eq(self.tx_start),
            self.tx_ready.eq(tx.ready),
            self.tx_line.eq(tx.tx),
            rx.rx.eq(self.rx_line),
            self.rx_data.eq(rx.data),
            self.rx_valid.eq(rx.valid),
        ]
        return m

    def ports(self):
        return [
            self.tx_data,
            self.tx_start,
            self.tx_ready,
            self.tx_line,
            self.rx_line,
            self.rx_data,
            self.rx_valid,
        ]


def test_uart(tmp_path):
    pair = Pair()
    tests = [
        "uart_carries_every_byte_back_to_back",
        "uart_tx_holds_each_bit_a_divisor_and_its_stop_bit_a_full_bit",
        "uart_rx_drops_a_byte_whose_stop_bit_is_low",
        "uart_rx_ignores_a_glitch_shorter_than_half_a_bit",
    ]
    run(pair, pair.ports(), "uart_pair", "test_uart", tests, tmp_path)


async def start(dut):
    """The clock running, the design out of reset, the receiver's line idle."""
    dut.rx_line.value = 1
    dut.tx_start.value = 0
    dut.tx_data.value = 0
    dut.rst.value = 1
    Clock(dut.clk, 10, unit="ns").start()
    for _ in range(3):
        await FallingEdge(dut.clk)
    dut.rst.value = 0
    for _ in range(3):
        await FallingEdge(dut.clk)


async def cycles(dut, n):
    for _ in range(n):
        await FallingEdge(dut.clk)


async def send(dut, byte):
    """Puts `byte` on the transmitter as soon as it is ready."""
    while not int(dut.tx_ready.value):
        await FallingEdge(dut.clk)
    dut.tx_data.value = byte
    dut.tx_start.value = 1
    await FallingEdge(dut.clk)
    dut.tx_start.value = 0


async def frame(dut, byte, stop=1):
    """Drives the receiver's line with one frame of `byte`, each bit for a divisor of cycles."""
    for bit in [0, *((byte >> i) & 1 for i in range(8)), stop]:
        dut.rx_line.value = bit
        await cycles(dut, DIVISOR)
    dut.rx_line.value = 1


async def received(dut, n):
    """The bytes the receiver gives over the next `n` cycles."""
    got = []
    for _ in range(n):
        await FallingEdge(dut.clk)
        if int(dut.rx_valid.value):
            got.append(dut.rx_data.value.to_unsigned())
    return got


@cocotb.test()
async def uart_carries_every_byte_back_to_back(dut):
    """The transmitter's line joined to the receiver's: every byte value, sent back to back as
    fast as the transmitter takes them, arrives once and in order."""
    await start(dut)

    async def join():
        while True:
            await FallingEdge(dut.clk)
            dut.rx_line.value = int(dut.tx_line.value)

    cocotb.start_soon(join())
    receiving = cocotb.start_soon(received(dut, 256 * 11 * DIVISOR + 100))
    for byte in range(256):
        await send(dut, byte)
    assert await receiving == list(range(256))


@cocotb.test()
async def uart_tx_holds_each_bit_a_divisor_and_its_stop_bit_a_full_bit(dut):
    """One byte, 0x55, on the transmitter's line: the start bit, each data bit least significant
    first and the stop bit last a divisor of cycles each, and `ready` comes back only at the end of
    the stop bit."""
    await start(dut)
    assert int(dut.tx_ready.value) and int(dut.tx_line.value) == 1
    dut.tx_data.value = 0x55
    dut.tx_start.value = 1
    await FallingEdge(dut.clk)
    dut.tx_start.value = 0
    line, ready = [], []
    for _ in range(10 * DIVISOR + 1):
        line.append(int(dut.tx_line.value))
        ready.append(int(dut.tx_ready.value))
        await FallingEdge(dut.clk)
    bits = [0, *((0x55 >> i) & 1 for i in range(8)), 1]
    assert line == [1] + [b for b in bits for _ in range(DIVISOR)]
    assert ready == [0] * (10 * DIVISOR) + [1]


@cocotb.test()
async def uart_rx_drops_a_byte_whose_stop_bit_is_low(dut):
    """A frame whose stop bit is low gives no byte; the next good frame still does."""
    await start(dut)
    bad = cocotb.start_soon(received(dut, 14 * DIVISOR))
    await frame(dut, 0xA5, stop=0)
    assert await bad == []
    good = cocotb.start_soon(received(dut, 12 * DIVISOR))
    await frame(dut, 0x3C)
    assert await good == [0x3C]


@cocotb.test()
async def uart_rx_ignores_a_glitch_shorter_than_half_a_bit(dut):
    """The line low for less than half a bit is no start bit: it gives no byte, and the next good
    frame still gives its own."""
    await start(dut)
    quiet = cocotb.start_soon(received(dut, 3 * DIVISOR))
    dut.rx_line.value = 0
    await cycles(dut, DIVISOR // 2 - 2)
    dut.rx_line.value = 1
    assert await quiet == []
    good = cocotb.start_soon(received(dut, 12 * DIVISOR))
    await frame(dut, 0x96)
    assert await good == [0x96]
