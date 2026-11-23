"""cocotb tests of gateware/cores/littlebit.py, the LittleBit layer engine, on Verilator, run by
pytest (docs/littlebit.md, "cocotb tests"): its weight ROMs read back against integer.json, and
the layer against the golden model, on the test vectors crates/littlebit/src/vectors.rs writes
(littlebit_vectors.json). Each engine test runs on both variants, the tables in logic (A) and in
memories (B). From the repo root, in OSS CAD Suite's environment:

    python3 -m pytest gateware

A test drives the engine's inputs and reads its outputs at the clock's falling edge, half a cycle
from the rising edge the design acts on.
"""

import json
import random
from pathlib import Path

import cocotb
import pytest
from amaranth import Elaboratable, Module, Signal
from cocotb.clock import Clock
from cocotb.triggers import FallingEdge, Timer

from gateware.cores.littlebit import ROM, VARIANTS, LittleBitEngine, sign_bits
from gateware.sim import run

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
VECTORS = json.loads((HERE / "littlebit_vectors.json").read_text())


def case(name):
    return next(c for c in VECTORS["cases"] if c["name"] == name)


def layer_of(c):
    """A case's layer: in the vectors file, or in the network file it names."""
    if "layer" in c:
        return c["layer"]
    return json.loads((ROOT / c["network"]).read_text())["layers"][c["layer_index"]]


class SignROMs(Elaboratable):
    """r16 layer 1's two sign ROMs, as the engine builds them, each index and data a port."""

    def __init__(self, layer):
        rank = layer["rank"]
        self.v = ROM([sign_bits(row) for row in layer["v"]], rank)
        columns = [
            sum(1 << o for o, row in enumerate(layer["u"]) if row[k] == "-") for k in range(rank)
        ]
        self.u = ROM(columns, layer["outputs"])
        self.v_index = Signal.like(self.v.index, name="v_index")
        self.v_data = Signal.like(self.v.data, name="v_data")
        self.u_index = Signal.like(self.u.index, name="u_index")
        self.u_data = Signal.like(self.u.data, name="u_data")

    def elaborate(self, platform):
        m = Module()
        m.submodules.v = self.v
        m.submodules.u = self.u
        m.d.comb += [
            self.v.index.eq(self.v_index),
            self.v_data.eq(self.v.data),
            self.u.index.eq(self.u_index),
            self.u_data.eq(self.u.data),
        ]
        return m

    def ports(self):
        return [self.v_index, self.v_data, self.u_index, self.u_data]


def test_littlebit_sign_roms(tmp_path):
    roms = SignROMs(layer_of(case("r16-layer-1")))
    tests = ["littlebit_sign_roms_read_back_r16_layer_1s_v_rows_and_u_columns"]
    run(roms, roms.ports(), "sign_roms", "test_littlebit", tests, tmp_path)


def engine(name, variant, test, tmp_path):
    e = LittleBitEngine(layer_of(case(name)), variant)
    run(e, e.ports(), "littlebit_engine", "test_littlebit", [test], tmp_path)


@pytest.mark.parametrize("variant", VARIANTS)
def test_littlebit_engine_r16_layer_1(variant, tmp_path):
    test = "littlebit_engine_computes_r16_layer_1_as_the_golden_model_does"
    engine("r16-layer-1", variant, test, tmp_path)


@pytest.mark.parametrize("variant", VARIANTS)
def test_littlebit_engine_r16_layer_2(variant, tmp_path):
    test = "littlebit_engine_computes_r16_layer_2_as_the_golden_model_does"
    engine("r16-layer-2", variant, test, tmp_path)


@pytest.mark.parametrize("variant", VARIANTS)
def test_littlebit_engine_convolution(variant, tmp_path):
    test = "littlebit_engine_computes_each_window_of_a_convolution_as_the_golden_model_does"
    engine("convolution-3x3", variant, test, tmp_path)


@cocotb.test()
async def littlebit_sign_roms_read_back_r16_layer_1s_v_rows_and_u_columns(dut):
    """Every row of V, 784 of them, and every column of U, 16, read back as integer.json has
    them: bit k of V's row i is 1 where row i's sign k is `-`, and bit o of U's column k where
    output o's sign k is."""
    layer = layer_of(case("r16-layer-1"))
    for i, row in enumerate(layer["v"]):
        dut.v_index.value = i
        await Timer(1, unit="ns")
        bits = int(dut.v_data.value)
        assert [(bits >> k) & 1 for k in range(16)] == [int(c == "-") for c in row], f"V row {i}"
    for k in range(layer["rank"]):
        dut.u_index.value = k
        await Timer(1, unit="ns")
        bits = int(dut.u_data.value)
        column = [int(row[k] == "-") for row in layer["u"]]
        assert [(bits >> o) & 1 for o in range(layer["outputs"])] == column, f"U column {k}"


@cocotb.test()
async def littlebit_engine_computes_r16_layer_1_as_the_golden_model_does(dut):
    """r16's first layer, 784 -> 256 at rank 16, on 6 vectors back to back: all zeros, all 127,
    and 4 drawn from 0 to 127. Every y and z is the golden model's, and each vector takes
    784 + 16 + 256 + 1 = 1,057 clocks."""
    await computes(dut, case("r16-layer-1"), gaps=False)


@cocotb.test()
async def littlebit_engine_computes_r16_layer_2_as_the_golden_model_does(dut):
    """r16's second layer, 256 -> 128 at rank 16, on 6 vectors back to back: all zeros, all 255,
    and 4 drawn from 0 to 255. Every y and z is the golden model's, and each vector takes
    256 + 16 + 128 + 1 = 401 clocks."""
    await computes(dut, case("r16-layer-2"), gaps=False)


@cocotb.test()
async def littlebit_engine_computes_each_window_of_a_convolution_as_the_golden_model_does(dut):
    """A 3 x 3 convolution from 2 channels to 4, at rank 3: the 25 windows of its 5 x 5 grid,
    with a gap before about a third of the inputs. Every y and z is the golden model's."""
    await computes(dut, case("convolution-3x3"), gaps=True)


async def computes(dut, c, gaps):
    """Streams each of the case's inputs into the engine, with gaps if `gaps`, and checks every
    output against the case's steps. With no gaps, it also checks the clocks a vector takes."""
    layer = layer_of(c)
    n_in, n_out, rank = layer["inputs"], layer["outputs"], layer["rank"]
    rng = random.Random(0)
    dut.x_valid.value = 0
    dut.x.value = 0
    dut.rst.value = 1
    Clock(dut.clk, 20, unit="ns").start()
    for _ in range(3):
        await FallingEdge(dut.clk)
    dut.rst.value = 0

    async def send():
        for vector in c["inputs"]:
            assert len(vector) == n_in
            for value in vector:
                while True:
                    await FallingEdge(dut.clk)
                    if gaps and rng.random() < 1 / 3:
                        dut.x_valid.value = 0
                        continue
                    dut.x.value = value
                    dut.x_valid.value = 1
                    # x_ready follows the engine's state alone, so it is settled here, and the
                    # input is taken on the rising edge ahead.
                    if int(dut.x_ready.value):
                        break
        await FallingEdge(dut.clk)
        dut.x_valid.value = 0

    async def receive():
        outputs, lasts, clock = [], [], 0
        budget = len(c["inputs"]) * (3 * n_in + rank + n_out) + 100
        while len(outputs) < len(c["inputs"]) * n_out:
            await FallingEdge(dut.clk)
            clock += 1
            assert clock < budget, f"{len(outputs)} outputs after {clock} clocks"
            if int(dut.y_valid.value):
                outputs.append((int(dut.y.value), dut.z.value.to_signed()))
                if int(dut.y_last.value):
                    lasts.append(clock)
                    assert len(outputs) % n_out == 0, f"y_last on output {len(outputs) - 1}"
        return outputs, lasts

    cocotb.start_soon(send())
    outputs, lasts = await cocotb.start_soon(receive())
    for j, steps in enumerate(c["steps"]):
        got = outputs[j * n_out : (j + 1) * n_out]
        for o, ((y, z), want_y, want_z) in enumerate(zip(got, steps["y"], steps["z"])):
            assert (y, z) == (want_y, want_z), (
                f"{c['name']}, input {j}, output {o}: y {y}, z {z}; the golden model's y "
                f"{want_y}, z {want_z} (its a {steps['a']}, t {steps['t']})"
            )
    assert len(lasts) == len(c["inputs"])
    if not gaps:
        period = n_in + rank + n_out + 1
        assert [b - a for a, b in zip(lasts, lasts[1:])] == [period] * (len(lasts) - 1)
