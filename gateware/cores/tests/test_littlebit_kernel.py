# amaranth: UnusedElaboratable=no
# (The tests of the kernel's refusals make kernels that are never elaborated.)

"""Tests of gateware/cores/littlebit_kernel.py, the hardwired kernel: cocotb on Verilator, run by
pytest, for the kernel against the golden model on the test vectors' Clean Up conv1 case
(littlebit_vectors.json, from crates/littlebit/src/vectors.rs); and plain pytest for its parts.
From the repo root, in OSS CAD Suite's environment:

    python3 -m pytest gateware

A test drives the kernel's inputs and reads its outputs at the clock's falling edge, half a cycle
from the rising edge the design acts on.
"""

import json
import os
import random
from pathlib import Path

import cocotb
import pytest
from amaranth.back import rtlil
from cocotb.clock import Clock
from cocotb.triggers import FallingEdge

from gateware.cores.littlebit_kernel import WindowKernel, channel_widths, csd
from gateware.sim import run

HERE = Path(__file__).resolve().parent
VECTORS = json.loads((HERE / "littlebit_vectors.json").read_text())
CASE = next(c for c in VECTORS["cases"] if c["name"] == "clean-up-conv1")
# The case's windows are flattened as the golden model's `windows` flattens them, row, column,
# then channel, and the inventory channels, 16 and 17, are its bytes (vectors.rs).
WIDTHS = channel_widths(CASE["layer"]["inputs"], 19, (16, 17), "rows")


@pytest.mark.parametrize("every", [1, 2])
def test_littlebit_kernel_clean_up_conv1(every, tmp_path):
    k = WindowKernel(CASE["layer"], WIDTHS, every)
    test = "littlebit_kernel_computes_each_window_of_clean_up_conv1_as_the_golden_model_does"
    run(k, k.ports(), "littlebit_kernel", "test_littlebit_kernel", [test], tmp_path,
        env={"KERNEL_LATENCY": str(k.latency)})


def test_csd_digits_sum_to_every_int8_and_its_negation():
    """Each digit is +1 or -1 and no two are next to each other, for -128 to 128."""
    for c in range(-128, 129):
        digits = csd(c)
        assert sum(d << e for e, d in digits) == c, c
        assert all(d in (1, -1) for _, d in digits), c
        exponents = [e for e, _ in digits]
        assert all(b - a >= 2 for a, b in zip(exponents, exponents[1:])), c


def test_channel_widths_follow_the_windows_order():
    rows = channel_widths(475, 19, (16, 17), "rows")
    channels = channel_widths(475, 19, (16, 17), "channels")
    assert rows[:19] == [1] * 16 + [8, 8, 1] and rows == rows[:19] * 25
    assert channels == [1] * 400 + [8] * 50 + [1] * 25
    with pytest.raises(ValueError):
        channel_widths(475, 19, (16, 17), "columns")


def test_littlebit_kernel_multiplies_nothing():
    """Every scale is shifts and adds, and every table logic: no multiply, no memory."""
    text = rtlil.convert(WindowKernel(CASE["layer"], WIDTHS), ports=[])
    assert "$mul" not in text and "$mem" not in text


def test_littlebit_kernel_refuses_a_layer_that_could_pass_an_int32():
    layer = dict(CASE["layer"], bias=[(1 << 31) - 1] + CASE["layer"]["bias"][1:])
    with pytest.raises(ValueError, match="past an int32"):
        WindowKernel(layer, WIDTHS)


def test_littlebit_kernel_refuses_widths_that_dont_fit():
    with pytest.raises(ValueError, match="widths"):
        WindowKernel(CASE["layer"], WIDTHS[:-1])
    with pytest.raises(ValueError, match="widths"):
        WindowKernel(CASE["layer"], [9] * len(WIDTHS))


def pack(window):
    word, at = 0, 0
    for x, w in zip(window, WIDTHS):
        assert x < 1 << w, f"{x} doesn't fit {w} bits"
        word |= x << at
        at += w
    return word


def to_signed(value, bits):
    return value - (1 << bits) if value >> (bits - 1) else value


@cocotb.test()
async def littlebit_kernel_computes_each_window_of_clean_up_conv1_as_the_golden_model_does(dut):
    """A layer of Clean Up's conv1's shape, 5 x 5 over 19 channels to 8 at rank 4: the 121
    windows of its 11 x 11 grid, back to back, then again with a gap before about a third of them.
    Every y and z is the golden model's, and each window's comes out the kernel's latency after it
    went in."""
    latency = int(os.environ["KERNEL_LATENCY"])
    outputs = CASE["layer"]["outputs"]
    dut.x_valid.value = 0
    dut.x.value = 0
    dut.rst.value = 1
    Clock(dut.clk, 20, unit="ns").start()
    for _ in range(3):
        await FallingEdge(dut.clk)
    dut.rst.value = 0
    for gaps in (False, True):
        rng = random.Random(0)
        sent, sent_at, got, clock = 0, [], [], 0
        while len(got) < len(CASE["inputs"]):
            if sent < len(CASE["inputs"]) and not (gaps and rng.random() < 1 / 3):
                dut.x.value = pack(CASE["inputs"][sent])
                dut.x_valid.value = 1
                sent_at.append(clock)
                sent += 1
            else:
                dut.x_valid.value = 0
            await FallingEdge(dut.clk)
            clock += 1
            assert clock < 3 * len(CASE["inputs"]) + latency + 10, f"{len(got)} out at {clock}"
            if int(dut.y_valid.value):
                y, z = int(dut.y.value), int(dut.z.value)
                got.append((
                    [(y >> (8 * o)) & 0xFF for o in range(outputs)],
                    [to_signed((z >> (32 * o)) & 0xFFFFFFFF, 32) for o in range(outputs)],
                    clock,
                ))
        dut.x_valid.value = 0
        for j, (steps, (y, z, at)) in enumerate(zip(CASE["steps"], got)):
            assert (y, z) == (steps["y"], steps["z"]), (
                f"window {j}, gaps {gaps}: y {y}, z {z}; the golden model's y {steps['y']}, "
                f"z {steps['z']}"
            )
            assert at - sent_at[j] == latency, f"window {j}, gaps {gaps}: {at - sent_at[j]}"
        for _ in range(latency + 2):
            await FallingEdge(dut.clk)
        assert not int(dut.y_valid.value)
