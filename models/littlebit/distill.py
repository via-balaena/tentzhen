"""The compression sweep's students, each trained to act as SocialJax's Clean Up policy does
(docs/littlebit.md, "How small the policy can get"). From this folder:

    uv run --locked python distill.py <student> <teacher.npz> <out-folder> <rollouts.npz>...

<student> names one of SPECS. <teacher.npz> is the teacher as `models/cleanup/students.py teacher`
writes it, and each <rollouts.npz> what `models/cleanup/rollouts.py --record` writes: observations,
and the teacher's logits for each. It writes <out-folder>/student.npz, the student in the form
`models/cleanup/students.py` loads and the game plays; student.pt, its PyTorch state, LittleBit's
latent factors and scales included, for the integer form to start from; and summary.json.

The recipe, the same for every student and fixed before the first run:

- **What it learns.** Each observation's 9 logits, by the KL divergence of the student's action
  distribution from the teacher's, as the policy is drawn from in play. Nothing else, no rewards.
- **How.** Adam at 10^-3, cosine-decayed to 0 over 20 epochs of batches of 4,096, from seed 0. The
  last 5 % of the observations, the last games recorded, are held out and only measured. The
  environment variable DISTILL_SEED sets another seed, for the starting weights and the batches'
  order both, to see how much a result moves from one training run to the next.
- **Where it starts.** A student of the teacher's shape starts from the teacher's weights, its
  LittleBit layers by Dual-SVID of them, as the MNIST runs started from E1. A narrower one starts
  from PyTorch's default initialization, its LittleBit layers by Dual-SVID of that.

A student keeps the policy's score if its mean return over the 1,000 games seeded 1,000 to 1,999
is at least 90 % of the teacher's on the same games. The 90 % was set before the first run, with
100 games. It became 1,000 after the teacher's own 100-game means came out at 521.0 and 481.0 on
two sets of seeds, and before any student was scored.
"""

import json
import math
import os
import platform
import sys
import time
from pathlib import Path

import numpy as np
import torch
from torch import nn

from littlebit import LittleBitConv2d, LittleBitLinear

ACTIONS = 9
GRID = 11
CHANNELS = 19
EPOCHS = 20
BATCH = 4096
RATE = 1e-3
HELD_OUT = 0.05

TEACHER = {"convs": [[32, 5], [32, 3], [32, 3]], "dense": 64, "actor": 64}
CONVS = ("conv1", "conv2", "conv3")
LINEARS = ("dense", "actor")


def littlebit_all(rank):
    """LittleBit at `rank` on every layer but the 9 logits'."""
    return {name: rank for name in CONVS + LINEARS}


# Each student: its shape, and the rank of each layer that is LittleBit; the rest are float.
SPECS = {
    "t32": TEACHER | {"littlebit": {}},
    "w16": TEACHER | {"convs": [[16, 5], [16, 3], [16, 3]], "littlebit": {}},
    "w8": TEACHER | {"convs": [[8, 5], [8, 3], [8, 3]], "littlebit": {}},
    "w4": TEACHER | {"convs": [[4, 5], [4, 3], [4, 3]], "littlebit": {}},
    "dense-r16": TEACHER | {"littlebit": {"dense": 16}},
    "dense-r4": TEACHER | {"littlebit": {"dense": 4}},
    "all-r16": TEACHER | {"littlebit": littlebit_all(16)},
    "all-r8": TEACHER | {"littlebit": littlebit_all(8)},
    "all-r4": TEACHER | {"littlebit": littlebit_all(4)},
    "w8-all-r4": TEACHER | {"convs": [[8, 5], [8, 3], [8, 3]], "littlebit": littlebit_all(4)},
    # Added after the first ten were scored: the all-r students started from the teacher never
    # trained (0.0 in every game), while w8-all-r4, started fresh, did. So the same shapes,
    # started fresh, and w8 at rank 8, the most its 8-wide convolutions allow.
    "all-r16-fresh": TEACHER | {"littlebit": littlebit_all(16), "start": "fresh"},
    "all-r4-fresh": TEACHER | {"littlebit": littlebit_all(4), "start": "fresh"},
    "w8-all-r8": TEACHER | {"convs": [[8, 5], [8, 3], [8, 3]], "littlebit": littlebit_all(8)},
}


def shapes(spec):
    """Each layer as (name, kind, in, out, kernel, positions): kind "conv" or "linear"."""
    out, c = [], CHANNELS
    for i, (width, kernel) in enumerate(spec["convs"]):
        out.append((f"conv{i + 1}", "conv", c, width, kernel, GRID * GRID))
        c = width
    out.append(("dense", "linear", GRID * GRID * c, spec["dense"], 1, 1))
    out.append(("actor", "linear", spec["dense"], spec["actor"], 1, 1))
    out.append(("logits", "linear", spec["actor"], ACTIONS, 1, 1))
    return out


def costs(spec):
    """What a decision computes: multiply-adds in the float layers; in the LittleBit ones, the
    sign additions, r (fan-in + out) a position, and the scale multiplications, fan-in + r + out a
    position. And the weights each holds: float weights, and LittleBit's sign bits and scales."""
    total = {"float_macs": 0, "sign_additions": 0, "scale_multiplications": 0,
             "float_weights": 0, "sign_bits": 0, "scales": 0}
    for name, _, n_in, n_out, kernel, positions in shapes(spec):
        fan_in = n_in * kernel * kernel
        rank = spec["littlebit"].get(name)
        if rank is None:
            total["float_macs"] += positions * fan_in * n_out
            total["float_weights"] += fan_in * n_out
        else:
            total["sign_additions"] += positions * rank * (fan_in + n_out)
            total["scale_multiplications"] += positions * (fan_in + rank + n_out)
            total["sign_bits"] += rank * (fan_in + n_out)
            total["scales"] += fan_in + rank + n_out
    return total


class Student(nn.Module):
    """The student as `models/cleanup/students.py` plays it, taking observations 11 x 11 x 19 and
    flattening the grid as Flax does, row by row and within a cell channel by channel."""

    def __init__(self, spec, teacher=None):
        super().__init__()
        self.layers = nn.ModuleDict()
        for name, kind, n_in, n_out, kernel, _ in shapes(spec):
            rank = spec["littlebit"].get(name)
            if rank is not None and rank > min(n_in * kernel * kernel, n_out):
                raise ValueError(f"{name} is {n_out} x {n_in * kernel * kernel}: no rank {rank}")
            if kind == "conv":
                dense = nn.Conv2d(n_in, n_out, kernel, padding=kernel // 2)
            else:
                dense = nn.Linear(n_in, n_out)
            if teacher is not None:
                weight, bias = teacher[name]
                with torch.no_grad():
                    dense.weight.copy_(weight)
                    dense.bias.copy_(bias)
            if rank is None:
                self.layers[name] = dense
            elif kind == "conv":
                self.layers[name] = LittleBitConv2d.from_dense(dense.weight, dense.bias, rank)
            else:
                self.layers[name] = LittleBitLinear.from_dense(dense.weight, dense.bias, rank)
        self.convs = [n for n in self.layers if n.startswith("conv")]

    def forward(self, x):
        x = x.permute(0, 3, 1, 2)
        for name in self.convs:
            x = torch.relu(self.layers[name](x))
        x = x.permute(0, 2, 3, 1).flatten(1)
        x = torch.relu(self.layers["dense"](x))
        x = torch.relu(self.layers["actor"](x))
        return self.layers["logits"](x)

    def dense_weights(self):
        """Each layer's float weight and bias, in Flax's layouts."""
        out = {}
        for name, layer in self.layers.items():
            if isinstance(layer, (LittleBitConv2d, LittleBitLinear)):
                weight = layer.dense()
                bias = layer.linear.bias if isinstance(layer, LittleBitConv2d) else layer.bias
            else:
                weight, bias = layer.weight, layer.bias
            weight = weight.permute(2, 3, 1, 0) if weight.dim() == 4 else weight.T
            out[name] = (weight.detach().cpu().numpy(), bias.detach().cpu().numpy())
        return out


def read_teacher(path):
    """The teacher's layers as PyTorch holds them: a convolution's kernel out x in x k x k, a
    linear layer's weight out x in."""
    with np.load(path, allow_pickle=False) as f:
        out = {}
        for name, kind, *_ in shapes(TEACHER | {"littlebit": {}}):
            kernel = torch.from_numpy(f[f"{name}/kernel"])
            weight = kernel.permute(3, 2, 0, 1) if kind == "conv" else kernel.T
            out[name] = (weight.contiguous(), torch.from_numpy(f[f"{name}/bias"]))
    return out


def kl(student_logits, teacher_logits):
    """The mean KL divergence of the student's action distribution from the teacher's, in nats."""
    teacher = torch.log_softmax(teacher_logits, dim=1)
    student = torch.log_softmax(student_logits, dim=1)
    return (teacher.exp() * (teacher - student)).sum(dim=1).mean()


def measure(net, obs, logits):
    """The held-out KL, and how often the student's most likely action is the teacher's."""
    net.eval()
    total, same = 0.0, 0
    with torch.no_grad():
        for i in range(0, len(obs), BATCH):
            out = net(obs[i:i + BATCH].float())
            total += kl(out, logits[i:i + BATCH]).item() * len(out)
            same += (out.argmax(1) == logits[i:i + BATCH].argmax(1)).sum().item()
    net.train()
    return total / len(obs), same / len(obs)


def main(name, teacher_path, out_folder, rollouts):
    if name not in SPECS:
        sys.exit(f"no student {name!r}; the students are {', '.join(SPECS)}")
    spec = SPECS[name] | {"name": name}
    seed = int(os.environ.get("DISTILL_SEED", "0"))
    torch.manual_seed(seed)
    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    obs, logits = [], []
    for path in rollouts:
        with np.load(path, allow_pickle=False) as f:
            obs.append(f["observations"])
            logits.append(f["logits"])
    obs = torch.from_numpy(np.concatenate(obs)).to(device)
    logits = torch.from_numpy(np.concatenate(logits)).to(device)
    held = int(len(obs) * HELD_OUT)
    train_obs, train_logits = obs[:-held], logits[:-held]
    test_obs, test_logits = obs[-held:], logits[-held:]

    same_shape = spec["convs"] == TEACHER["convs"] and spec.get("start") != "fresh"
    net = Student(spec, read_teacher(teacher_path) if same_shape else None).to(device)
    steps = EPOCHS * math.ceil(len(train_obs) / BATCH)
    optimizer = torch.optim.Adam(net.parameters(), lr=RATE)
    schedule = torch.optim.lr_scheduler.CosineAnnealingLR(optimizer, steps)
    start = time.perf_counter()
    curve = [{"epoch": 0, "train_kl": None, "held_out_kl": None, "held_out_same_top": None}]
    curve[0]["held_out_kl"], curve[0]["held_out_same_top"] = measure(net, test_obs, test_logits)
    for epoch in range(1, EPOCHS + 1):
        order = torch.randperm(len(train_obs), device=device)
        total = 0.0
        for i in range(0, len(order), BATCH):
            batch = order[i:i + BATCH]
            loss = kl(net(train_obs[batch].float()), train_logits[batch])
            optimizer.zero_grad()
            loss.backward()
            optimizer.step()
            schedule.step()
            total += loss.item() * len(batch)
        held_kl, same = measure(net, test_obs, test_logits)
        curve.append({"epoch": epoch, "train_kl": total / len(train_obs),
                      "held_out_kl": held_kl, "held_out_same_top": same})
        print(f"{name} epoch {epoch}: train KL {total / len(train_obs):.4f}, held-out KL "
              f"{held_kl:.4f}, same top action {same:.3f}", flush=True)
    seconds = time.perf_counter() - start

    out = Path(out_folder)
    out.mkdir(parents=True, exist_ok=True)
    arrays = {"spec": np.array(json.dumps(spec))}
    for layer, (weight, bias) in net.dense_weights().items():
        arrays[f"{layer}/kernel"] = weight.astype(np.float32)
        arrays[f"{layer}/bias"] = bias.astype(np.float32)
    np.savez(out / "student.npz", **arrays)
    torch.save(net.state_dict(), out / "student.pt")
    summary = {
        "student": spec,
        "costs": costs(spec),
        "teacher_costs": costs(TEACHER | {"littlebit": {}}),
        "started_from": "the teacher" if same_shape else "PyTorch's default initialization",
        "recipe": {"epochs": EPOCHS, "batch": BATCH, "rate": RATE, "schedule": "cosine to 0",
                   "held_out": HELD_OUT, "seed": seed, "loss": "KL(teacher || student)"},
        "rollouts": [str(Path(p).resolve()) for p in rollouts],
        "observations": {"train": len(train_obs), "held_out": held},
        "curve": curve,
        "seconds": seconds,
        "versions": {"python": platform.python_version(), "torch": torch.__version__,
                     "numpy": np.__version__},
        "device": torch.cuda.get_device_name(0) if device.type == "cuda" else "cpu",
    }
    (out / "summary.json").write_text(json.dumps(summary, indent=2) + "\n", newline="\n")
    print(f"wrote {out / 'student.npz'}: held-out KL {curve[-1]['held_out_kl']:.4f}, "
          f"{seconds:.0f} s")


if __name__ == "__main__":
    if len(sys.argv) < 5:
        sys.exit(__doc__)
    main(sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4:])
