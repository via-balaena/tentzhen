"""MNIST, fetched from the mirror torchvision uses and checked against pinned sha256s before use.

The mirror and the pins are in mnist.toml beside this file, which crates/littlebit reads too.
MNIST's terms of use are unread (docs/littlebit.md, "Open questions"), so its files are never
committed: they live in a folder named on the command line.
"""

import gzip
import hashlib
import sys
import tomllib
import urllib.request
from pathlib import Path

import numpy as np

FILES = (
    "train-images-idx3-ubyte.gz",
    "train-labels-idx1-ubyte.gz",
    "t10k-images-idx3-ubyte.gz",
    "t10k-labels-idx1-ubyte.gz",
)


def _pins():
    """mnist.toml, refused unless it has exactly its three keys and a pin for each of the files."""
    path = Path(__file__).with_name("mnist.toml")
    pins = tomllib.loads(path.read_text())
    if set(pins) != {"mirror", "pinned", "sha256"} or set(pins["sha256"]) != set(FILES):
        raise ValueError(f"{path.name}: not a mirror, a note and a sha256 for each of {FILES}")
    return pins["mirror"], pins["sha256"]


MIRROR, SHA256 = _pins()


def fetch(folder):
    """Downloads each file missing from `folder`, then checks them all."""
    folder = Path(folder)
    folder.mkdir(parents=True, exist_ok=True)
    for name in SHA256:
        path = folder / name
        if not path.exists():
            urllib.request.urlretrieve(MIRROR + name, path)
    check(folder)


def check(folder):
    """Refuses a file whose sha256 is not its pin."""
    for name, pin in SHA256.items():
        got = hashlib.sha256((Path(folder) / name).read_bytes()).hexdigest()
        if got != pin:
            raise ValueError(f"{name}: its sha256 is {got}, not the pinned {pin}")


def _idx(path, magic, shape):
    """An IDX file's array of uint8: its magic number and sizes are checked against `shape`."""
    data = gzip.decompress(path.read_bytes())
    dims = len(shape)
    words = [int.from_bytes(data[4 * i : 4 * i + 4], "big") for i in range(dims + 1)]
    if words != [magic, *shape]:
        raise ValueError(f"{path.name}: header {words}, not {[magic, *shape]}")
    return np.frombuffer(data, dtype=np.uint8, offset=4 * (dims + 1)).reshape(shape)


def load(folder):
    """The training and test sets, after `check`: images as uint8 (n, 28, 28), labels as uint8 (n,)."""
    folder = Path(folder)
    check(folder)
    return (
        (
            _idx(folder / "train-images-idx3-ubyte.gz", 2051, (60000, 28, 28)),
            _idx(folder / "train-labels-idx1-ubyte.gz", 2049, (60000,)),
        ),
        (
            _idx(folder / "t10k-images-idx3-ubyte.gz", 2051, (10000, 28, 28)),
            _idx(folder / "t10k-labels-idx1-ubyte.gz", 2049, (10000,)),
        ),
    )


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit("usage: python mnist.py <folder>, to fetch MNIST into it and check it")
    fetch(sys.argv[1])
    print(f"{sys.argv[1]}: all four files match their pins")
