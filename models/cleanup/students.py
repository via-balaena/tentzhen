"""Smaller policies for Clean Up, each trained to act as SocialJax's does: the students of the
compression sweep (docs/littlebit.md, "How small the policy can get").

A student is the teacher's shape with its sizes changed: convolutions, each a width and a square
kernel, at stride 1 with "same" padding, each followed by ReLU; the grid flattened, row by row
and within a cell channel by channel, as Flax flattens it; a dense layer and ReLU; the actor's
hidden layer and ReLU; then the 9 actions' logits. It has no critic: only acting is run.

A student's file, `student.npz`, holds its `spec` as JSON, and each layer's float weights in
Flax's layouts: a convolution's kernel as (kernel, kernel, in, out), a dense layer's as (in, out),
and each bias. A LittleBit layer is written as the dense weights its factors make, so the game
plays it as trained. `models/littlebit/distill.py` writes them, and `teacher` writes the teacher's
own weights in the same form, as a student with the teacher's shape.
"""

import json
import sys
from pathlib import Path

import flax.linen as nn
import jax.numpy as jnp
import numpy as np

import policy

TEACHER_SPEC = {"convs": [[32, 5], [32, 3], [32, 3]], "dense": 64, "actor": 64}


def layer_names(spec):
    """conv1, conv2, ..., then dense, actor and logits."""
    return [f"conv{i + 1}" for i in range(len(spec["convs"]))] + ["dense", "actor", "logits"]


class Student(nn.Module):
    spec: dict

    @nn.compact
    def __call__(self, x):
        for i, (width, kernel) in enumerate(self.spec["convs"]):
            x = nn.relu(nn.Conv(width, (kernel, kernel), name=f"conv{i + 1}")(x))
        x = x.reshape((x.shape[0], -1))
        x = nn.relu(nn.Dense(self.spec["dense"], name="dense")(x))
        x = nn.relu(nn.Dense(self.spec["actor"], name="actor")(x))
        return nn.Dense(policy.ACTIONS, name="logits")(x)


def load(path):
    """A student's spec and its parameters, as Flax applies them."""
    with np.load(path, allow_pickle=False) as f:
        spec = json.loads(str(f["spec"]))
        params = {
            name: {"kernel": jnp.asarray(f[f"{name}/kernel"]), "bias": jnp.asarray(f[f"{name}/bias"])}
            for name in layer_names(spec)
        }
    return spec, {"params": params}


def logits(spec, params, observations):
    return Student(spec).apply(params, observations)


def teacher(socialjax_folder, out):
    """Writes SocialJax's trained policy as a student file: its convolutions, its dense layer and
    its actor head, without the critic."""
    p = policy.load(socialjax_folder)["params"]
    cnn = p["CNN_0"]
    layers = {
        "conv1": cnn["Conv_0"],
        "conv2": cnn["Conv_1"],
        "conv3": cnn["Conv_2"],
        "dense": cnn["Dense_0"],
        "actor": p["Dense_0"],
        "logits": p["Dense_1"],
    }
    arrays = {"spec": np.array(json.dumps(TEACHER_SPEC | {"name": "teacher"}))}
    for name, layer in layers.items():
        arrays[f"{name}/kernel"] = np.asarray(layer["kernel"], dtype=np.float32)
        arrays[f"{name}/bias"] = np.asarray(layer["bias"], dtype=np.float32)
    np.savez(out, **arrays)
    # The file plays as the checkpoint does: the same logits, to float32's last bits.
    spec, params = load(out)
    obs = np.random.default_rng(0).integers(0, 2, (16, 11, 11, 19)).astype(np.float32)
    ours = np.asarray(logits(spec, params, obs))
    theirs = np.asarray(policy.logits(policy.load(socialjax_folder), obs))
    if not np.allclose(ours, theirs, rtol=0, atol=1e-5):
        raise ValueError("the teacher's student file gives other logits than the checkpoint")
    print(f"wrote {out}, which gives the checkpoint's logits")


if __name__ == "__main__":
    if len(sys.argv) != 4 or sys.argv[1] != "teacher":
        sys.exit("uv run --locked python students.py teacher <socialjax-folder> <teacher.npz>")
    teacher(sys.argv[2], Path(sys.argv[3]))
