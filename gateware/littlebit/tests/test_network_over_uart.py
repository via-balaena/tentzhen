"""cocotb tests of gateware/littlebit/network.py, r16 behind the UART, on Verilator, run by pytest:
the board design less its PLL, which Verilator has no model of. A stand-in platform hands it plain
signals for its UART and its LED. The UART runs at a divisor of 8 here, to keep each test short;
the board's, 434 at 50 MHz, is the echo's, tested in gateware/bringup/tests/test_uart_echo.py. The
answers are checked against the golden model's, from the test vectors
(gateware/cores/tests/littlebit_vectors.json). From the repo root, in OSS CAD Suite's environment:

    python3 -m pytest gateware
"""

import json
from pathlib import Path
from types import SimpleNamespace

import cocotb
from amaranth import Signal
from cocotb.clock import Clock
from cocotb.triggers import FallingEdge

from gateware.littlebit.network import ANSWER, FRAME, PIXELS, NetworkOverUART, r16
from gateware.sim import run

DIVISOR = 8
QUIET = 40 * DIVISOR
VECTORS = Path(__file__).resolve().parents[2] / "cores/tests/littlebit_vectors.json"
# The clocks r16 takes an image, worked out in gateware/cores/tests/test_littlebit_network.py.
R16_CLOCKS = 1_219
ANSWER_BYTES = 1 + 4 * 10 + 1 + 4


def r16_case():
    return next(c for c in json.loads(VECTORS.read_text())["network_cases"] if c["name"] == "r16")


def network_over_uart(test, tmp_path):
    uart_rx, uart_tx, led = Signal(name="uart_rx"), Signal(name="uart_tx"), Signal(name="led")
    pins = {
        "uart": SimpleNamespace(rx=SimpleNamespace(i=uart_rx), tx=SimpleNamespace(o=uart_tx)),
        "led": SimpleNamespace(o=led),
    }
    platform = SimpleNamespace(request=lambda name, number: pins[name])
    top = NetworkOverUART(r16(), 50_000_000, divisor=DIVISOR, quiet=QUIET)
    run(top, [uart_rx, uart_tx, led], "littlebit_network_over_uart", "test_network_over_uart",
        [test], tmp_path, platform)


def test_network_over_uart_answers(tmp_path):
    network_over_uart("littlebit_network_answers_each_frame_as_the_golden_model_does", tmp_path)


def test_network_over_uart_drops_a_part_frame(tmp_path):
    network_over_uart(
        "littlebit_network_drops_a_frame_left_part_way_and_answers_the_next", tmp_path
    )


async def cycles(dut, n):
    for _ in range(n):
        await FallingEdge(dut.clk)


async def start(dut):
    dut.uart_rx.value = 1
    dut.rst.value = 1
    Clock(dut.clk, 20, unit="ns").start()
    await cycles(dut, 3)
    dut.rst.value = 0
    await cycles(dut, 3)


async def send(dut, data):
    """Sends `data` into the design's RX line, back to back, 8N1."""
    for byte in data:
        for bit in [0, *((byte >> i) & 1 for i in range(8)), 1]:
            dut.uart_rx.value = bit
            await cycles(dut, DIVISOR)


async def receive(dut, n):
    """Reads `n` bytes from the design's TX line, each bit in its middle. It listens from before
    the frame is sent, so it waits for the frame's 785 bytes, the network and the answer, with
    room to spare."""
    got = []
    clock = 0
    budget = 2 * (1 + PIXELS + n) * 10 * DIVISOR + 2 * R16_CLOCKS
    while len(got) < n:
        await FallingEdge(dut.clk)
        clock += 1
        assert clock < budget, f"{len(got)} of {n} bytes came in {clock} clocks"
        if int(dut.uart_tx.value):
            continue
        await cycles(dut, DIVISOR // 2)
        assert int(dut.uart_tx.value) == 0, "a start bit shorter than half a bit"
        byte = 0
        for i in range(8):
            await cycles(dut, DIVISOR)
            byte |= int(dut.uart_tx.value) << i
        await cycles(dut, DIVISOR)
        assert int(dut.uart_tx.value) == 1, f"no stop bit after {byte:#04x}"
        got.append(byte)
    return got


def check(answer, c, j):
    """An answer is 0x5A, the 10 scores, each an int32 least significant byte first, the digit,
    and the clocks, a uint32 least significant byte first: the golden model's scores and digit,
    and r16's clocks."""
    assert len(answer) == ANSWER_BYTES
    assert answer[0] == ANSWER, f"image {j}: answer header {answer[0]:#04x}"
    scores = [
        int.from_bytes(bytes(answer[1 + 4 * o : 5 + 4 * o]), "little", signed=True)
        for o in range(10)
    ]
    digit = answer[41]
    clocks = int.from_bytes(bytes(answer[42:46]), "little")
    assert (scores, digit) == (c["scores"][j], c["digits"][j]), (
        f"image {j}: scores {scores}, digit {digit}; the golden model's {c['scores'][j]}, "
        f"{c['digits'][j]}"
    )
    assert clocks == R16_CLOCKS, f"image {j}: {clocks} clocks, not {R16_CLOCKS}"


@cocotb.test()
async def littlebit_network_answers_each_frame_as_the_golden_model_does(dut):
    """Two frames, one after the other's answer: an image drawn from 0 to 255, then one of all
    255. Each answer is 0x5A, the golden model's 10 scores and digit, and 1,219 clocks. A stray
    byte before the first frame is dropped."""
    c = r16_case()
    await start(dut)
    await send(dut, [0x00])
    for j in (2, 1):
        listening = cocotb.start_soon(receive(dut, ANSWER_BYTES))
        await send(dut, [FRAME, *c["images"][j]])
        check(await listening, c, j)


@cocotb.test()
async def littlebit_network_drops_a_frame_left_part_way_and_answers_the_next(dut):
    """A frame that stops after 300 of its 784 pixels, then silence past the design's limit, then
    a whole frame: the answer is the golden model's for the whole frame alone, and nothing
    answers the part one."""
    c = r16_case()
    await start(dut)
    await send(dut, [FRAME, *c["images"][3][:300]])
    for _ in range(2 * QUIET):
        await FallingEdge(dut.clk)
        assert int(dut.uart_tx.value) == 1, "the part frame was answered"
    listening = cocotb.start_soon(receive(dut, ANSWER_BYTES))
    await send(dut, [FRAME, *c["images"][4]])
    check(await listening, c, 4)
