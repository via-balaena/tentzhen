"""A training run's two files, as `runs/<run>/` holds them: weights.f32, every parameter as
little-endian float32 one after another, and summary.json, what was run and what it scored, whose
`layout` gives each parameter's name and shape in that order and whose `weights_sha256` is the
weights file's. No pickle. crates/littlebit reads the same two files (`run::Run::load`).
"""

import hashlib
import json
import math
from pathlib import Path

import numpy as np
import torch


def write_weights(net, path):
    """Writes every parameter, in the state dict's order, as little-endian float32, row-major in
    PyTorch's shapes (a Linear's weight is out x in); gives the layout and the file's sha256."""
    layout, data = [], bytearray()
    for name, t in net.state_dict().items():
        layout.append({"name": name, "shape": list(t.shape)})
        data += t.detach().cpu().contiguous().numpy().astype("<f4").tobytes()
    path.write_bytes(bytes(data))
    return layout, hashlib.sha256(data).hexdigest()


def write_summary(path, summary):
    """Writes the summary as JSON, LF on every system: the repo keeps text files LF
    (.gitattributes)."""
    path.write_text(json.dumps(summary, indent=2) + "\n", newline="\n")


def read_run(folder):
    """The summary and the parameters, by name as float32 tensors on the CPU, of the run in
    `folder`; refused unless the weights' sha256 is the summary's and their size the layout's."""
    folder = Path(folder)
    summary = json.loads((folder / "summary.json").read_text())
    data = (folder / "weights.f32").read_bytes()
    got = hashlib.sha256(data).hexdigest()
    if got != summary["weights_sha256"]:
        raise ValueError(f"{folder}: weights.f32's sha256 is {got}, not the summary's")
    if len(data) % 4:
        raise ValueError(f"{folder}: weights.f32 is not whole float32s")
    values = np.frombuffer(data, dtype="<f4")
    tensors, at = {}, 0
    for t in summary["layout"]:
        n = math.prod(t["shape"])
        if at + n > len(values):
            raise ValueError(f"{folder}: weights.f32 ends inside {t['name']}")
        tensors[t["name"]] = torch.from_numpy(values[at : at + n].astype(np.float32)).reshape(
            t["shape"]
        )
        at += n
    if at != len(values):
        raise ValueError(f"{folder}: weights.f32 is longer than the layout")
    return summary, tensors
