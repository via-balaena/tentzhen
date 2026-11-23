"""E2 and E3, the baselines between E1 and the LittleBit sweep (docs/littlebit.md, "Experiments"):
E1's 784 -> 256 -> 128 -> 10 MLP with its two hidden layers' weights int8 (E2) or binary with one
scale per output (E3), started from E1's committed weights (runs/e1) and fine-tuned as the LittleBit
runs are, so that a baseline and a LittleBit run differ only in their hidden layers. The output
layer stays dense. From this folder:

    uv run --locked python train_baseline.py <mnist-folder> (int8 | binary) <out-folder>

It writes weights.f32 and summary.json to the out folder as train_rank.py does, and crates/littlebit
re-scores them with its own arithmetic (`e2_scores_what_its_summary_says` and
`e3_scores_what_its_summary_says`). The model kept is the one after the last epoch; nothing is
chosen by the test set.

What is chosen here, before the first run:

- Everything train_rank.py chooses but the LittleBit layer: pixels enter as floor(p / 2) / 128;
  each hidden layer's outputs are uint8 times 2^e, each e fixed before training as the smallest
  that clamps nothing the started model gives on the training images; E1's training, every
  parameter trained, cross-entropy on the labels, no distillation; and the output layer dense,
  float in training.
- int8 (E2): the hidden layers are dense, and their weights stay float in training, as the output
  layer's and the LittleBit scales do. The exporter makes them int8, each matrix on one
  power-of-two grid, as it makes the output layer's, and check 1 measures what that costs.
- binary (E3): each hidden layer is y = h * (sign(W) x) + b, one scale h per output
  (littlebit.BinaryLinear), with train_rank.py's sign() and its straight-through gradient. W starts
  as E1's weights, h as the mean of |W| along each output's row, and b as E1's bias.
"""

import sys

import torch
from torch import nn

from e1_fp32 import correct
from littlebit import Activations, BinaryLinear
from runfiles import read_run
from train_rank import (
    ACTIVATIONS,
    E1,
    RECIPE,
    SIGN,
    calibrate,
    fine_tune,
    hardware_mnist,
    output_layer,
    setup,
    write_run,
)

KINDS = {
    "int8": {
        "run": "E2",
        "model": (
            "784 -> 256 -> 128 -> 10 MLP: layers 0 and 2 dense, their weights float in training "
            "and int8 at export, each followed by uint8 activations; layer 4 dense float"
        ),
        "init": "E1's weights and biases, as they are",
    },
    "binary": {
        "run": "E3",
        "model": (
            "784 -> 256 -> 128 -> 10 MLP: layers 0 and 2 binary, h * (sign(W) x) + b with one "
            "scale per output, each followed by uint8 activations; layer 4 dense float"
        ),
        "init": (
            "W as E1's weights, h as the mean of |W| along each output's row, b as E1's bias"
        ),
    },
}


def dense(weight, bias):
    """A Linear holding `weight` (out x in) and `bias`."""
    n_out, n_in = weight.shape
    layer = nn.Linear(n_in, n_out)
    with torch.no_grad():
        layer.weight.copy_(weight)
        layer.bias.copy_(bias)
    return layer


def model(start, kind):
    """E1's MLP with layers 0 and 2 dense (int8) or binary (binary), each followed by uint8
    activations, and E1's output layer as it is."""
    out = output_layer(start)
    hidden = dense if kind == "int8" else BinaryLinear.from_dense
    return nn.Sequential(
        hidden(start["0.weight"], start["0.bias"]),
        Activations(),
        hidden(start["2.weight"], start["2.bias"]),
        Activations(),
        out,
    )


def main(mnist_folder, kind, out_folder):
    if kind not in KINDS:
        sys.exit(__doc__)
    device = setup()
    data = hardware_mnist(mnist_folder, device)
    xtr, _, xte, yte = data

    start_summary, start = read_run(E1)
    net = model(start, kind).to(device)
    calibrate(net, xtr)
    exponents = [net[1].exponent, net[3].exponent]
    init_test = correct(net, xte, yte)
    for i in (0, 2):
        w = net[i].weight.detach()
        print(f"layer {i}: |weight| up to {float(w.abs().max()):.3f}, "
              f"{float((w.abs() > 1).float().mean()) * 100:.2f}% beyond 1")
    print(f"activation exponents {exponents}; start: test {init_test}/10000")

    curve, seconds = fine_tune(net, *data)
    k = KINDS[kind]
    quantized = {
        "weights": kind,
        "start": {"run": start_summary["run"], "weights_sha256": start_summary["weights_sha256"]},
        "init": k["init"],
    }
    if kind == "binary":
        quantized["sign"] = SIGN
    quantized.update({
        "activations": ACTIVATIONS,
        "activation_exponents": exponents,
        "init_test_correct": init_test,
    })
    head = {
        "run": k["run"],
        "model": k["model"],
        "weights": start_summary["weights"],
        "parameters": sum(p.numel() for p in net.parameters()),
        "recipe": RECIPE,
        "quantized": quantized,
    }
    write_run(out_folder, net, head, data, curve, seconds)


if __name__ == "__main__":
    if len(sys.argv) != 4:
        sys.exit(__doc__)
    main(sys.argv[1], sys.argv[2], sys.argv[3])
