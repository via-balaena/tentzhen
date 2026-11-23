"""One rank of the LittleBit sweep (docs/littlebit.md, "Experiments"): the 784 -> 256 -> 128 -> 10
MLP with its two hidden layers made LittleBit layers at rank r, started from E1's committed weights
(runs/e1) and fine-tuned by the recipe's steps 2-4. The output layer stays dense. From this folder:

    uv run --locked python train_rank.py <mnist-folder> <rank> <out-folder> [<run> <start-folder>]

Given a run's name and another start, a dense run of the same form as E1 with other hidden widths,
it makes that MLP's hidden layers LittleBit layers instead: E10 is `train_rank.py <mnist-folder> 9
runs\\e10 E10 runs\\e10-start`.

It writes weights.f32 and summary.json to the out folder as e1_fp32.py does (runfiles.py), and
crates/littlebit re-scores them with its own arithmetic (`r16_scores_what_its_summary_says`, for
r = 16). The model kept is the one after the last epoch; nothing is chosen by the test set.

What the recipe leaves open is chosen here, before the first run:

- Dual-SVID's splits, which the paper does not give: see littlebit.dual_svid.
- Pixels enter as floor(p / 2) / 128: the hardware's inputs, 0-255 scaled to 0-127
  (docs/littlebit.md, "Fixed-point formats"), on a grid of 2^-7.
- Each hidden layer's output is uint8 times 2^e (littlebit.Uint8), with e fixed before training
  for each layer: the smallest that clamps nothing the started model gives on the 60,000 training
  images, layer 1's first and then layer 2's on layer 1's quantized output.
- The output layer's weights and the scales g, l and h stay float in training. The exporter makes
  them integers, and check 1 measures what that costs.
- Training is E1's: Adam at 1e-3, cosine decay by step to 0 over 20 epochs, batches of 128 in an
  order shuffled each epoch, seed 0, PyTorch's deterministic algorithms, cross-entropy on the
  labels. Every parameter trains: the latent factors, the scales, the biases and the output layer.
  The paper also distils from the full-precision model (section 4.1); this does not.

train_baseline.py trains E2 and E3 with the same steps (setup, hardware_mnist, calibrate,
fine_tune and write_run), so the runs differ only in their hidden layers.
"""

import math
import os
import platform
import sys
import time
from pathlib import Path

# cuBLAS gives the same results run to run only with a fixed workspace, set before CUDA starts.
os.environ.setdefault("CUBLAS_WORKSPACE_CONFIG", ":4096:8")

import numpy as np
import torch
import torch.nn.functional as F
from torch import nn

import mnist
from e1_fp32 import BATCH, EPOCHS, LEARNING_RATE, SEED, correct, shape
from littlebit import Activations, LittleBitLinear, exponent_for
from runfiles import read_run, write_summary, write_weights

E1 = Path(__file__).with_name("runs") / "e1"
PIXELS = "floor(p / 2) / 128"
SIGN = "+1 where w >= 0, else -1; the gradient passed where |w| <= 1"
ACTIVATIONS = (
    "floor(y / 2^e) clamped to 0..255, times 2^e; the gradient passed where 0 <= y < 256 * 2^e"
)
RECIPE = {
    "optimizer": "Adam",
    "learning_rate": LEARNING_RATE,
    "schedule": "cosine, by step, to 0",
    "epochs": EPOCHS,
    "batch": BATCH,
    "seed": SEED,
    "pixels": PIXELS,
    "deterministic_algorithms": True,
}


def output_layer(start):
    """The start's output layer, as it is: E1's, unless another start is given."""
    out = nn.Linear(start["4.weight"].shape[1], start["4.weight"].shape[0])
    with torch.no_grad():
        out.weight.copy_(start["4.weight"])
        out.bias.copy_(start["4.bias"])
    return out


def widths(start):
    """The widths of the start's MLP, from its input to its outputs."""
    w = [start["0.weight"].shape[1]]
    return w + [start[f"{i}.weight"].shape[0] for i in (0, 2, 4)]


def model(start, rank):
    """E1's MLP with layers 0 and 2 made LittleBit layers by Dual-SVID from E1's weights, each
    followed by uint8 activations, and E1's output layer as it is."""
    out = output_layer(start)
    return nn.Sequential(
        LittleBitLinear.from_dense(start["0.weight"], start["0.bias"], rank),
        Activations(),
        LittleBitLinear.from_dense(start["2.weight"], start["2.bias"], rank),
        Activations(),
        out,
    )


@torch.no_grad()
def calibrate(net, x):
    """Sets each Activations' exponent to the smallest that clamps nothing on `x`, in order, each
    layer on the quantized output of the one before."""
    a = x
    for layer, activations in [(net[0], net[1]), (net[2], net[3])]:
        y = layer(a)
        activations.exponent = exponent_for(float(y.max()))
        a = activations(y)


def setup():
    """E1's seed and PyTorch's deterministic algorithms, set before anything else runs; gives the
    device to train on."""
    torch.manual_seed(SEED)
    torch.use_deterministic_algorithms(True)
    torch.backends.cudnn.benchmark = False
    return "cuda" if torch.cuda.is_available() else "cpu"


def hardware_mnist(mnist_folder, device):
    """MNIST's training images and labels, then its test images and labels, on `device`, each
    pixel p entering as floor(p / 2) / 128 (PIXELS)."""
    (train_x, train_y), (test_x, test_y) = mnist.load(mnist_folder)

    def images(a):
        return torch.from_numpy((a.reshape(len(a), -1) >> 1).astype(np.float32) / 128).to(device)

    def labels(a):
        return torch.from_numpy(a.astype(np.int64)).to(device)

    return images(train_x), labels(train_y), images(test_x), labels(test_y)


def fine_tune(net, xtr, ytr, xte, yte):
    """Trains every parameter of `net` by E1's recipe: Adam at 1e-3, cosine decay by step to 0
    over 20 epochs, batches of 128 in an order shuffled each epoch from seed 0, cross-entropy on
    the labels. The test set is scored after each epoch to show the curve, and nothing is chosen
    by it. Gives the curve and the seconds the training took."""
    device = xtr.device
    steps_per_epoch = math.ceil(len(xtr) / BATCH)
    opt = torch.optim.Adam(net.parameters(), lr=LEARNING_RATE)
    sched = torch.optim.lr_scheduler.CosineAnnealingLR(opt, T_max=EPOCHS * steps_per_epoch)
    order = torch.Generator().manual_seed(SEED)

    curve = []
    start_time = time.perf_counter()
    for epoch in range(1, EPOCHS + 1):
        net.train()
        perm = torch.randperm(len(xtr), generator=order).to(device)
        total_loss = 0.0
        for i in range(0, len(xtr), BATCH):
            batch = perm[i : i + BATCH]
            loss = F.cross_entropy(net(xtr[batch]), ytr[batch])
            opt.zero_grad(set_to_none=True)
            loss.backward()
            opt.step()
            sched.step()
            total_loss += float(loss.detach()) * len(batch)
        row = {
            "epoch": epoch,
            "train_loss": total_loss / len(xtr),
            "test_correct": correct(net, xte, yte),
        }
        curve.append(row)
        print(f"epoch {epoch:2d}  train loss {row['train_loss']:.4f}  test {row['test_correct']}/10000")
    return curve, time.perf_counter() - start_time


def write_run(out_folder, net, head, data, curve, seconds):
    """Writes weights.f32 and summary.json to `out_folder`. The summary is `head` (the run, the
    model, its weights and parameters, the recipe and the run's own section), then what every run
    has. `data` is hardware_mnist's four tensors."""
    xtr, ytr, xte, _ = data
    out = Path(out_folder)
    out.mkdir(parents=True, exist_ok=True)
    layout, sha256 = write_weights(net, out / "weights.f32")
    summary = dict(head)
    summary.update({
        "mnist_sha256": mnist.SHA256,
        "train_correct": correct(net, xtr, ytr),
        "train_images": len(xtr),
        "test_correct": curve[-1]["test_correct"],
        "test_images": len(xte),
        "curve": curve,
        "layout": layout,
        "weights_sha256": sha256,
        "seconds": seconds,
        "device": (
            torch.cuda.get_device_name(0) if xtr.device.type == "cuda" else platform.processor()
        ),
        "versions": {
            "python": platform.python_version(),
            "torch": torch.__version__,
            "cuda": torch.version.cuda,
            "cudnn": torch.backends.cudnn.version(),
            "numpy": np.__version__,
        },
    })
    write_summary(out / "summary.json", summary)
    print(
        f"test accuracy {summary['test_correct'] / 100:.2f}%  "
        f"train {summary['train_correct']}/{len(xtr)}  "
        f"weights sha256 {summary['weights_sha256'][:16]}  {seconds:.1f} s"
    )


def main(mnist_folder, rank, out_folder, run=None, start_folder=E1):
    device = setup()
    data = hardware_mnist(mnist_folder, device)
    xtr, _, xte, yte = data

    start_summary, start = read_run(start_folder)
    net = model(start, rank).to(device)
    calibrate(net, xtr)
    exponents = [net[1].exponent, net[3].exponent]
    init_test = correct(net, xte, yte)
    for i in (0, 2):
        for name in ("u", "v"):
            p = getattr(net[i], name).detach()
            print(f"layer {i} {name}: |latent| up to {float(p.abs().max()):.3f}, "
                  f"{float((p.abs() > 1).float().mean()) * 100:.2f}% beyond 1")
    print(f"activation exponents {exponents}; Dual-SVID start: test {init_test}/10000")

    curve, seconds = fine_tune(net, *data)
    head = {
        "run": run or f"r{rank}",
        "model": (
            f"{shape(widths(start))} MLP: layers 0 and 2 LittleBit at r = {rank}, each followed by "
            "uint8 activations; layer 4 dense float"
        ),
        "weights": start_summary["weights"],
        "parameters": sum(p.numel() for p in net.parameters()),
        "recipe": RECIPE,
        "littlebit": {
            "rank": rank,
            "start": {"run": start_summary["run"], "weights_sha256": start_summary["weights_sha256"]},
            "init": (
                "Dual-SVID: U' and V' from W's truncated SVD, each with the square root of the "
                "singular values; h and g from rank-one fits of |U'| and |V'|, each fit split by "
                "the square root of its singular value; l the product of the fits' other halves"
            ),
            "sign": SIGN,
            "activations": ACTIVATIONS,
            "activation_exponents": exponents,
            "init_test_correct": init_test,
        },
    }
    write_run(out_folder, net, head, data, curve, seconds)


if __name__ == "__main__":
    if len(sys.argv) == 4:
        main(sys.argv[1], int(sys.argv[2]), sys.argv[3])
    elif len(sys.argv) == 6:
        main(sys.argv[1], int(sys.argv[2]), sys.argv[3], sys.argv[4], Path(sys.argv[5]))
    else:
        sys.exit(__doc__)
