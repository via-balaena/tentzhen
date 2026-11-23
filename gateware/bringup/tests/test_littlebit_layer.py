"""cocotb tests of gateware/bringup/littlebit_layer.py, r16's first layer behind the UART, on
Verilator, run by pytest: the board design less its PLL, which Verilator has no model of. A stand-in
platform hands it plain signals for its UART and its LED. The UART runs at a divisor of 8 here, to
keep each test short; the board's, 434 at 50 MHz, is the echo's, tested in test_uart_echo.py. The
answers are checked against the golden model's, from the test vectors
(gateware/cores/tests/littlebit_vectors.json). From the repo root, in OSS CAD Suite's
environment:

    python3 -m pytest gateware
"""

import json
from pathlib import Path
from types import SimpleNamespace

import cocotb
from amaranth import Signal
from cocotb.clock import Clock
from cocotb.triggers import FallingEdge

from gateware.bringup.littlebit_layer import ANSWER, FRAME, LayerOverUART, r16_layer
from gateware.sim import run

DIVISOR = 8
QUIET = 40 * DIVISOR
VECTORS = Path(__file__).resolve().parents[2] / "cores/tests/littlebit_vectors.json"


def r16_layer_1_case():
    return next(c for c in json.loads(VECTORS.read_text())["cases"] if c["name"] == "r16-layer-1")


def layer_over_uart(test, tmp_path):
    uart_rx, uart_tx, led = Signal(name="uart_rx"), Signal(name="uart_tx"), Signal(name="led")
    pins = {
        "uart": SimpleNamespace(rx=SimpleNamespace(i=uart_rx), tx=SimpleNamespace(o=uart_tx)),
        "led": SimpleNamespace(o=led),
    }
    platform = SimpleNamespace(request=lambda name, number: pins[name])
    top = LayerOverUART(r16_layer(0), 50_000_000, divisor=DIVISOR, quiet=QUIET)
    tests = [test]
    run(top, [uart_rx, uart_tx, led], "littlebit_layer", "test_littlebit_layer", tests, tmp_path,
        platform)


def test_littlebit_layer_answers(tmp_path):
    layer_over_uart("littlebit_layer_answers_each_frame_as_the_golden_model_does", tmp_path)


def test_littlebit_layer_drops_a_part_frame(tmp_path):
    layer_over_uart("littlebit_layer_drops_a_frame_left_part_way_and_answers_the_next", tmp_path)


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
    """Reads `n` bytes from the design's TX line, each bit in its middle."""
    got = []
    budget = 0
    while len(got) < n:
        await FallingEdge(dut.clk)
        budget += 1
        assert budget < (n + 10) * 20 * DIVISOR, f"{len(got)} of {n} bytes came"
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


def check(answer, steps, which):
    """An answer is 0x5A, then each output's y and its z, little-endian: the golden model's."""
    n_out = len(steps["y"])
    assert len(answer) == 1 + 5 * n_out
    assert answer[0] == ANSWER, f"{which}: answer header {answer[0]:#04x}"
    for o in range(n_out):
        y = answer[1 + 5 * o]
        z = int.from_bytes(bytes(answer[2 + 5 * o : 6 + 5 * o]), "little", signed=True)
        assert (y, z) == (steps["y"][o], steps["z"][o]), (
            f"{which}, output {o}: y {y}, z {z}; the golden model's y {steps['y'][o]}, "
            f"z {steps['z'][o]}"
        )


@cocotb.test()
async def littlebit_layer_answers_each_frame_as_the_golden_model_does(dut):
    """Two frames, one after the other's answer: a vector drawn from 0 to 127, then all 127. Each
    answer is 0x5A and the golden model's y and z for all 256 outputs. A stray byte before the
    first frame is dropped."""
    c = r16_layer_1_case()
    await start(dut)
    await send(dut, [0x00])
    for j in (2, 1):
        listening = cocotb.start_soon(receive(dut, 1 + 5 * 256))
        await send(dut, [FRAME, *c["inputs"][j]])
        check(await listening, c["steps"][j], f"input {j}")


@cocotb.test()
async def littlebit_layer_drops_a_frame_left_part_way_and_answers_the_next(dut):
    """A frame that stops after 300 of its 784 bytes, then silence past the design's limit, then a
    whole frame: the answer is the golden model's for the whole frame alone, and nothing answers
    the part one."""
    c = r16_layer_1_case()
    await start(dut)
    await send(dut, [FRAME, *c["inputs"][3][:300]])
    for _ in range(2 * QUIET):
        await FallingEdge(dut.clk)
        assert int(dut.uart_tx.value) == 1, "the part frame was answered"
    listening = cocotb.start_soon(receive(dut, 1 + 5 * 256))
    await send(dut, [FRAME, *c["inputs"][4]])
    check(await listening, c["steps"][4], "input 4")
