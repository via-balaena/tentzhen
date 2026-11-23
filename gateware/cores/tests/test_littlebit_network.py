"""cocotb tests of gateware/cores/littlebit.py's output layer, `DenseEngine`, and its whole
network, `LittleBitNetwork`, on Verilator, run by pytest (docs/littlebit.md, "cocotb tests"),
against the golden model: on the test vectors crates/littlebit/src/vectors.rs writes
(littlebit_vectors.json), and on MNIST's test images, check 2 of Gate 2. The rank sweep's other
networks, r4, r8, r32 and r64, E11's r42 and E10, the wider network, are each given their test
vectors too. r16 runs in
both variants, its tables in logic (A) and in block RAM (B, E9): on its test vectors, and on all
10,000 MNIST test images in A and the first 1,000 in B.

MNIST is never committed, so the golden model scores its images when the test runs: the pytest
test runs `cargo run -p tentzhen-littlebit -- mnist-vectors`, which reads MNIST from the folder
MNIST_DIR names and writes the images and their scores into the test's own temporary folder.
LITTLEBIT_MNIST_IMAGES, if set, says how many test images to score, at most. From the repo root,
in OSS CAD Suite's environment, with cargo on the path:

    MNIST_DIR=<folder> python3 -m pytest gateware

A test drives the inputs and reads the outputs at the clock's falling edge, half a cycle from the
rising edge the design acts on.
"""

import json
import os
import subprocess
from pathlib import Path

import cocotb
import pytest
from cocotb.clock import Clock
from cocotb.triggers import FallingEdge, RisingEdge, with_timeout
from cocotb.utils import get_sim_time

from gateware.cores.littlebit import DenseEngine, LittleBitNetwork
from gateware.sim import run

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
VECTORS = json.loads((HERE / "littlebit_vectors.json").read_text())
PERIOD_NS = 20
# r16, an image at a time with its pixels back to back: layer 1 hands its first output to layer 2
# 784 + 1 + 16 + 3 = 804 clocks after taking its first pixel (in, V's last add, the latents, and
# U's three clocks to an output); layer 2 hands its first to the output layer 256 + 1 + 16 + 3 =
# 276 clocks after that; the output layer takes its 128, adds the last in a clock later, and runs
# the argmax over its 10 outputs, one a clock, the last raising `done`: 804 + 276 + 128 + 1 + 10.
R16_CLOCKS = 1_219
# The networks of the rank sweep's other runs, and E10's, each a case of the test vectors after
# r16's.
SWEEP = [c["name"] for c in VECTORS["network_cases"][1:]]
# The MNIST test images r16 is given in each variant: all of them in A, check 2; in B, whose
# arithmetic is A's and whose every image reads every row of every table, the first 1,000, as the
# board was given in check 3.
MNIST_IMAGES = {"A": 10_000, "B": 1_000}


def dense_case(name):
    return next(c for c in VECTORS["dense_cases"] if c["name"] == name)


def network_of(path):
    return json.loads((ROOT / path).read_text())


def network_clocks(network):
    """The clocks `network` takes for an image, its pixels back to back, as R16_CLOCKS counts them
    for r16: each hidden layer hands its first output on inputs + 1 + rank + 3 clocks after taking
    its first input, and the output layer takes its inputs, adds the last a clock later and runs
    the argmax, a clock an output. That is 1,187 + 2r for the 784 -> 256 -> 128 -> 10 MLP, and
    2,339 + 2r for E10's 784 -> 1024 -> 512 -> 10."""
    hidden = sum(l["inputs"] + 1 + l["rank"] + 3 for l in network["layers"])
    o = network["output"]
    return hidden + o["inputs"] + 1 + o["outputs"]


def dense_layer(c):
    return c["layer"] if "layer" in c else network_of(c["network"])["output"]


def dense(name, test, tmp_path):
    e = DenseEngine(dense_layer(dense_case(name)))
    run(e, e.ports(), "littlebit_dense", "test_littlebit_network", [test], tmp_path)


def test_littlebit_output_layer_r16(tmp_path):
    test = "littlebit_output_layer_scores_r16s_output_layer_as_the_golden_model_does"
    dense("r16-output", test, tmp_path)


def test_littlebit_output_layer_extremes(tmp_path):
    test = "littlebit_output_layer_reaches_both_ends_of_an_int32_and_picks_the_first_of_a_tie"
    dense("dense-extremes", test, tmp_path)


@pytest.mark.parametrize("variant", MNIST_IMAGES)
def test_littlebit_network(variant, tmp_path):
    most = MNIST_IMAGES[variant]
    images = str(min(most, int(os.environ.get("LITTLEBIT_MNIST_IMAGES", most))))
    mnist = tmp_path / "mnist.json"
    command = ["cargo", "run", "-q", "--locked", "-p", "tentzhen-littlebit", "--"]
    subprocess.run([*command, "mnist-vectors", images, str(mnist)], cwd=ROOT, check=True)
    net = LittleBitNetwork(network_of(VECTORS["network_cases"][0]["network"]), variant)
    tests = [
        "littlebit_network_scores_r16s_test_vectors_as_the_golden_model_does",
        "littlebit_network_scores_mnist_test_images_as_the_golden_model_does",
    ]
    run(net, net.ports(), "littlebit_network", "test_littlebit_network", tests, tmp_path,
        env={"LITTLEBIT_MNIST": str(mnist), "LITTLEBIT_MNIST_IMAGES": images})


@pytest.mark.parametrize("name", SWEEP)
def test_littlebit_network_sweep(name, tmp_path):
    c = next(c for c in VECTORS["network_cases"] if c["name"] == name)
    net = LittleBitNetwork(network_of(c["network"]))
    tests = ["littlebit_network_scores_the_sweep_s_test_vectors_as_the_golden_model_does"]
    run(net, net.ports(), "littlebit_network", "test_littlebit_network", tests, tmp_path,
        env={"LITTLEBIT_CASE": name})


async def start(dut):
    dut.x_valid.value = 0
    dut.x.value = 0
    dut.rst.value = 1
    Clock(dut.clk, PERIOD_NS, unit="ns").start()
    for _ in range(3):
        await FallingEdge(dut.clk)
    dut.rst.value = 0
    await FallingEdge(dut.clk)


async def feed(dut, values):
    """Hands `values` in one a clock, each held until it is taken. Returns the time, in ns, of the
    falling edge before the rising edge that took the first."""
    first = None
    for value in values:
        dut.x.value = value
        dut.x_valid.value = 1
        while True:
            # x_ready follows the design's state alone, so it is settled here, and the input is
            # taken on the rising edge ahead.
            ready = int(dut.x_ready.value)
            if ready and first is None:
                first = get_sim_time(unit="ns")
            await FallingEdge(dut.clk)
            if ready:
                break
    dut.x_valid.value = 0
    return first


async def answer(dut, n_out, clocks):
    """Waits at most `clocks` clocks for `done`, then reads the scores and the digit."""
    await with_timeout(RisingEdge(dut.done), clocks * PERIOD_NS, "ns")
    await FallingEdge(dut.clk)
    assert int(dut.done.value) == 1
    scores = [getattr(dut, f"score_{o}").value.to_signed() for o in range(n_out)]
    return scores, int(dut.digit.value)


async def scores_each_input(dut, c):
    """An output layer's case: each input, one at a time, and its scores and digit checked."""
    n_in, n_out = len(c["inputs"][0]), len(c["scores"][0])
    await start(dut)
    for j, x in enumerate(c["inputs"]):
        await feed(dut, x)
        # The last input is added in a clock later, then the argmax takes a clock an output.
        scores, digit = await answer(dut, n_out, 2 + n_out)
        assert (scores, digit) == (c["scores"][j], c["digits"][j]), (
            f"{c['name']}, input {j}: scores {scores}, digit {digit}; the golden model's "
            f"{c['scores'][j]}, {c['digits'][j]}"
        )
        assert n_in == len(x)


@cocotb.test()
async def littlebit_output_layer_scores_r16s_output_layer_as_the_golden_model_does(dut):
    """r16's output layer, 128 -> 10, given all zeros, all 255 and 4 inputs drawn from 0 to 255,
    one at a time: every score and digit is the golden model's."""
    await scores_each_input(dut, dense_case("r16-output"))


@cocotb.test()
async def littlebit_output_layer_reaches_both_ends_of_an_int32_and_picks_the_first_of_a_tie(dut):
    """A made-up output layer, 4 -> 5, whose scores reach -2^31 and 2^31 - 1 and tie: every
    score and digit is the golden model's, the first of the highest picked when a later output
    ties it."""
    await scores_each_input(dut, dense_case("dense-extremes"))


async def scores_each_image(dut, c, clocks=R16_CLOCKS):
    """A network's case: each image, one at a time, its pixels back to back, the next started on
    the clock the last is done. Every score, the digit, and the clocks each image took, both as
    the design counts them and as the test does, are checked, against `clocks`."""
    n_out = len(c["scores"][0])
    await start(dut)
    wrong = []
    for j, image in enumerate(c["images"]):
        first = await feed(dut, image)
        scores, digit = await answer(dut, n_out, 2 * clocks)
        seen = round((get_sim_time(unit="ns") - first) / PERIOD_NS)
        cycles = int(dut.cycles.value)
        if (scores, digit) != (c["scores"][j], c["digits"][j]):
            wrong.append(
                f"image {j}: scores {scores}, digit {digit}; the golden model's "
                f"{c['scores'][j]}, {c['digits'][j]}"
            )
        assert (cycles, seen) == (clocks, clocks), (
            f"{c['name']}, image {j}: {cycles} clocks counted, {seen} seen, not {clocks}"
        )
    dut._log.info(
        f"{c['name']}: {len(c['images']) - len(wrong)} of {len(c['images'])} images scored as "
        f"the golden model scores them, every score and digit; {clocks} clocks each"
    )
    assert not wrong, f"{c['name']}: {len(wrong)} images differ: " + "; ".join(wrong[:5])


@cocotb.test()
async def littlebit_network_scores_r16s_test_vectors_as_the_golden_model_does(dut):
    """r16, 784 -> 256 -> 128 -> 10, given an image of all zeros, one of all 255 and 4 drawn from
    0 to 255: every score and digit is the golden model's, and each image takes 1,219 clocks."""
    await scores_each_image(dut, VECTORS["network_cases"][0])


@cocotb.test()
async def littlebit_network_scores_the_sweep_s_test_vectors_as_the_golden_model_does(dut):
    """One of the rank sweep's other networks, or E10's, the case LITTLEBIT_CASE names, given
    r16's six test images: every score and digit is that network's golden model's, and each image
    takes the clocks network_clocks counts: 1,195 at r = 4, 1,315 at r = 64, and 2,357 for E10."""
    c = next(c for c in VECTORS["network_cases"] if c["name"] == os.environ["LITTLEBIT_CASE"])
    await scores_each_image(dut, c, network_clocks(network_of(c["network"])))


@cocotb.test()
async def littlebit_network_scores_mnist_test_images_as_the_golden_model_does(dut):
    """r16 given MNIST's test images, as many as the pytest test scored, LITTLEBIT_MNIST_IMAGES:
    all 10,000 in Variant A unless told fewer, check 2. Every score and digit is the golden
    model's, and each image takes 1,219 clocks."""
    c = json.loads(Path(os.environ["LITTLEBIT_MNIST"]).read_text())
    assert len(c["images"]) == int(os.environ["LITTLEBIT_MNIST_IMAGES"])
    await scores_each_image(dut, c)
