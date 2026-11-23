"""Fixed signs, trainable scales (docs/littlebit.md, "Training with the signs fixed"): r16, as
committed in runs/r16, fine-tuned on a task with only some of its parameters free, to see how much
a chip whose signs are hardwired could still learn by changing what it keeps in registers or block
RAM. From this folder:

    uv run --locked python train_frozen.py <mnist-folder> <fashion-folder> <task> <regime> <out>

<task> is what it learns:
- mnist: MNIST as r16 learnt it, the control.
- inverted: MNIST with every pixel p made 255 - p, the same digits drawn light on dark.
- fashion: Fashion-MNIST (zalandoresearch/fashion-mnist, MIT), MNIST's format, 10 kinds of
  clothing in place of digits; its files are checked against the sha256s in FASHION below.

<regime> is what trains:
- none: nothing; r16 is scored as it is.
- output: the output layer alone, 128 x 10 weights and 10 biases: 1,290 values.
- scales: the output layer, and each hidden layer's g, l, h and bias, with U and V fixed, so every
  sign stays as r16 has it: 3,130 values.
- all: every parameter, the latent U and V too, so signs can flip: 25,914 values, what a new
  bitstream could hold.
- built, for the inverted task only: nothing trains. Layer 1's g is negated and its bias moved by
  what the all-127/128 image gives, inside what `scales` may change. The inverted input is 127/128
  - x, since floor((255 - p) / 2) = 127 - floor(p / 2), so the network is r16 on MNIST again, and
  the regime can learn the task exactly.

Chosen before the first run: r16's recipe as it is (Adam at 1e-3, cosine decay by step to 0 over
20 epochs, batches of 128 shuffled from seed 0, cross-entropy, deterministic algorithms); r16's
activation exponents, fixed as the hardware's shifts are; pixels as floor(p / 2) / 128 after the
task's change. The test set is scored after each epoch to show the curve, and nothing is chosen by
it. The scales and the output layer stay float, as in r16's training: what integers would cost is
`unknown` until an export measures it.
"""

import hashlib
import json
import math
import os
import sys
import time
from pathlib import Path

os.environ.setdefault("CUBLAS_WORKSPACE_CONFIG", ":4096:8")

import numpy as np
import torch
import torch.nn.functional as F
from torch import nn

import mnist
from e1_fp32 import BATCH, EPOCHS, LEARNING_RATE, SEED, correct
from littlebit import Activations, LittleBitLinear
from runfiles import read_run, write_summary
from train_rank import setup

R16 = Path(__file__).with_name("runs") / "r16"
# Fashion-MNIST's files, their sha256s as downloaded on 2026-11-23 from the repo's data/fashion;
# each file's MD5 is the one its README lists.
FASHION = {
    "train-images-idx3-ubyte.gz": "3aede38d61863908ad78613f6a32ed271626dd12800ba2636569512369268a84",
    "train-labels-idx1-ubyte.gz": "a04f17134ac03560a47e3764e11b92fc97de4d1bfaf8ba1a3aa29af54cc90845",
    "t10k-images-idx3-ubyte.gz": "346e55b948d973a97e58d2351dde16a484bd415d4595297633bb08f03db6a073",
    "t10k-labels-idx1-ubyte.gz": "67da17c76eaffca5446c3361aaab5c3cd6d1c2608764d35dfb1850b086bf8dd5",
}
TASKS = ("mnist", "inverted", "fashion")
REGIMES = ("none", "output", "scales", "all", "built")


def fashion(folder):
    """Fashion-MNIST's training and test sets, as mnist.load gives MNIST's, after each file is
    checked against its sha256."""
    folder = Path(folder)
    for name, pin in FASHION.items():
        got = hashlib.sha256((folder / name).read_bytes()).hexdigest()
        if got != pin:
            raise ValueError(f"{name}: its sha256 is {got}, not the pinned {pin}")
    idx = mnist._idx
    return (
        (
            idx(folder / "train-images-idx3-ubyte.gz", 2051, (60000, 28, 28)),
            idx(folder / "train-labels-idx1-ubyte.gz", 2049, (60000,)),
        ),
        (
            idx(folder / "t10k-images-idx3-ubyte.gz", 2051, (10000, 28, 28)),
            idx(folder / "t10k-labels-idx1-ubyte.gz", 2049, (10000,)),
        ),
    )


def data(task, mnist_folder, fashion_folder, device):
    """The task's training and test images and labels on `device`, each pixel p entering as
    floor(p / 2) / 128 after the task's change."""
    if task == "fashion":
        (train_x, train_y), (test_x, test_y) = fashion(fashion_folder)
    else:
        (train_x, train_y), (test_x, test_y) = mnist.load(mnist_folder)
    if task == "inverted":
        train_x, test_x = 255 - train_x, 255 - test_x

    def images(a):
        return torch.from_numpy((a.reshape(len(a), -1) >> 1).astype(np.float32) / 128).to(device)

    def labels(a):
        return torch.from_numpy(a.astype(np.int64)).to(device)

    return images(train_x), labels(train_y), images(test_x), labels(test_y)


def r16(device):
    """r16 as committed: its parameters and its activation exponents."""
    summary, tensors = read_run(R16)
    rank = summary["littlebit"]["rank"]
    out = tensors["4.weight"]
    net = nn.Sequential(
        LittleBitLinear(784, 256, rank),
        Activations(),
        LittleBitLinear(256, 128, rank),
        Activations(),
        nn.Linear(out.shape[1], out.shape[0]),
    )
    net.load_state_dict(tensors)
    net[1].exponent, net[3].exponent = summary["littlebit"]["activation_exponents"]
    return summary, net.to(device)


def free(net, regime):
    """Sets which parameters train, by `regime`; gives their names."""
    for name, p in net.named_parameters():
        layer, kind = name.split(".")
        if regime == "all":
            p.requires_grad = True
        elif regime == "scales":
            p.requires_grad = layer == "4" or kind in ("g", "l", "h", "bias")
        elif regime == "output":
            p.requires_grad = layer == "4"
        else:
            p.requires_grad = False
    return [name for name, p in net.named_parameters() if p.requires_grad]


@torch.no_grad()
def build_inverted(net):
    """Makes r16 take inverted MNIST as it takes MNIST: layer 1's g negated, and its bias moved by
    what the all-127/128 image adds, so that each of its outputs is what it gave the image before
    inversion. Every sign stays."""
    layer = net[0]
    full = torch.full((1, layer.g.numel()), 127 / 128, device=layer.g.device)
    shift = layer(full)[0] - layer.bias
    layer.g.neg_()
    layer.bias.add_(shift)


def signs(net):
    """Every sign of the hidden layers' U and V, as the forward pass takes them."""
    return torch.cat([(getattr(net[i], m) >= 0).flatten() for i in (0, 2) for m in ("u", "v")])


def main(mnist_folder, fashion_folder, task, regime, out_folder):
    if task not in TASKS or regime not in REGIMES or (regime == "built" and task != "inverted"):
        sys.exit(__doc__)
    device = setup()
    xtr, ytr, xte, yte = data(task, mnist_folder, fashion_folder, device)
    start_summary, net = r16(device)
    names = free(net, regime)
    trainable = [p for p in net.parameters() if p.requires_grad]
    before = signs(net).clone()
    start_test = correct(net, xte, yte)
    print(f"{task}, {regime}: {sum(p.numel() for p in trainable)} values free; "
          f"r16 as it is: test {start_test}/10000")
    same = None
    if regime == "built":
        _, plain = r16(device)
        _, _, mte, _ = data("mnist", mnist_folder, fashion_folder, device)
        build_inverted(net)
        with torch.no_grad():
            plain.eval(), net.eval()
            same = int((plain(mte).argmax(1) == net(xte).argmax(1)).sum())
        print(f"built: the same digit as r16 on MNIST on {same} of {len(xte)} test images")

    curve = []
    seconds = 0.0
    if trainable:
        steps_per_epoch = math.ceil(len(xtr) / BATCH)
        opt = torch.optim.Adam(trainable, lr=LEARNING_RATE)
        sched = torch.optim.lr_scheduler.CosineAnnealingLR(opt, T_max=EPOCHS * steps_per_epoch)
        order = torch.Generator().manual_seed(SEED)
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
            row = {"epoch": epoch, "train_loss": total_loss / len(xtr),
                   "test_correct": correct(net, xte, yte)}
            curve.append(row)
            print(f"epoch {epoch:2d}  train loss {row['train_loss']:.4f}  "
                  f"test {row['test_correct']}/10000")
        seconds = time.perf_counter() - start_time

    flipped = int((signs(net) != before).sum())
    weights = b"".join(
        t.detach().cpu().contiguous().numpy().astype("<f4").tobytes()
        for t in net.state_dict().values()
    )
    summary = {
        "run": f"r16-{task}-{regime}",
        "task": task,
        "regime": regime,
        "start": {"run": start_summary["run"], "weights_sha256": start_summary["weights_sha256"]},
        "free": names,
        "free_values": sum(p.numel() for p in trainable),
        "signs": int(before.numel()),
        "signs_flipped": flipped,
        "start_test_correct": start_test,
        "same_digit_as_r16_on_mnist": same,
        "train_correct": correct(net, xtr, ytr),
        "test_correct": correct(net, xte, yte),
        "curve": curve,
        "weights_sha256": hashlib.sha256(weights).hexdigest(),
        "seconds": seconds,
    }
    out = Path(out_folder)
    out.mkdir(parents=True, exist_ok=True)
    write_summary(out / "summary.json", summary)
    print(f"{task}, {regime}: test {summary['test_correct']}/10000, train "
          f"{summary['train_correct']}/60000, {flipped} of {before.numel()} signs flipped, "
          f"weights sha256 {summary['weights_sha256'][:16]}, {seconds:.1f} s")


if __name__ == "__main__":
    if len(sys.argv) != 6:
        sys.exit(__doc__)
    main(*sys.argv[1:])
