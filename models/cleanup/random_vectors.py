"""JAX's random numbers for a few keys and sizes, written out as the test vectors that the golden
model's copy of them, crates/cleanup/src/random.rs, has to match bit for bit. From this folder:

    uv run --locked python random_vectors.py           # writes random_vectors.json
    uv run --locked python random_vectors.py --check   # fails unless the file is what it writes

It calls only `jax.random`, on the CPU, at the JAX that uv.lock pins, and needs no SocialJax.
JAX's random numbers are Threefry-2x32 (jax/_src/prng.py) in the layout JAX calls "partitionable",
the default since JAX 0.5.0 (jax/_src/config.py, `jax_threefry_partitionable`). SocialJax's Clean Up
draws every random number it uses through split, uniform, randint (as int8 and int16) and
permutation (clean_up.py and movement.py at the commit socialjax.toml pins), so those are what this
writes, with the raw hash beneath them.
"""

import json
import os
import sys
from pathlib import Path

os.environ["JAX_PLATFORMS"] = "cpu"

import jax  # noqa: E402
import jax.numpy as jnp  # noqa: E402
import numpy as np  # noqa: E402
from jax._src import prng  # noqa: E402

OUT = Path(__file__).with_name("random_vectors.json")

# The three known-answer tests of Random123's Threefry-2x32, 20 rounds, as (key, counter).
HASHES = [
    ((0x00000000, 0x00000000), (0x00000000, 0x00000000)),
    ((0xFFFFFFFF, 0xFFFFFFFF), (0xFFFFFFFF, 0xFFFFFFFF)),
    ((0x13198A2E, 0x03707344), (0x243F6A88, 0x85A308D3)),
]
SEEDS = [0, 1, 42, 2**31 - 1, -1, -(2**31)]
SPLITS = [2, 3, 7]
SIZES = [1, 2, 3, 7, 10, 100]
# (minval, maxval, dtype): SocialJax's own draws, an empty and a reversed range, and the ends of
# each type.
RANDINTS = [
    (0, 3, "int8"),
    (0, 4, "int16"),
    (0, 9, "int32"),
    (-5, 5, "int32"),
    (0, 1, "int32"),
    (3, 3, "int32"),
    (5, 2, "int16"),
    (0, 300, "int16"),
    (-128, 127, "int8"),
    (-(2**15), 2**15 - 1, "int16"),
    (0, 2**31 - 1, "int32"),
    (-(2**31), 2**31 - 1, "int32"),
]
RANDINT_SIZE = 50
# A shuffle sorts once below 1,626 elements and twice from there (random.py, `_shuffle`).
PERMUTATIONS = [1, 2, 7, 30, 100, 1625, 1626, 1700]


def u32(a):
    return [int(v) for v in np.asarray(a, dtype=np.uint32).ravel()]


def keys():
    """The keys each draw is made with: each seed's, and three split from seed 0's."""
    out = [jax.random.PRNGKey(s) for s in SEEDS]
    out += list(jax.random.split(jax.random.PRNGKey(0), 3))
    return out


def vectors():
    data = {"jax": jax.__version__, "hash": [], "seed": [], "split": [], "bits": [],
            "uniform": [], "randint": [], "permutation": [], "rows": []}
    for key, count in HASHES:
        out = prng.threefry_2x32(np.uint32(key), np.uint32(count))
        data["hash"].append({"key": list(key), "count": list(count), "out": u32(out)})
    for s in SEEDS:
        data["seed"].append({"seed": s, "key": u32(jax.random.PRNGKey(s))})
    for i, key in enumerate(keys()):
        k = u32(key)
        for n in SPLITS:
            split = np.asarray(jax.random.split(key, n), dtype=np.uint32)
            data["split"].append({"key": k, "n": n, "keys": [u32(row) for row in split]})
        for n in SIZES:
            bits = jax.random.bits(key, (n,), jnp.uint32)
            data["bits"].append({"key": k, "bits": u32(bits)})
            uniform = np.asarray(jax.random.uniform(key, (n,)), dtype=np.float32)
            data["uniform"].append({"key": k, "bits": u32(uniform.view(np.uint32))})
        for lo, hi, dtype in RANDINTS:
            values = jax.random.randint(key, (RANDINT_SIZE,), lo, hi, dtype=dtype)
            data["randint"].append({"key": k, "minval": lo, "maxval": hi, "dtype": dtype,
                                    "values": [int(v) for v in np.asarray(values)]})
        if i < 2:
            for n in PERMUTATIONS:
                perm = jax.random.permutation(key, n)
                data["permutation"].append({"key": k, "n": n,
                                            "order": [int(v) for v in np.asarray(perm)]})
        # A 2-D array is shuffled by its rows, as SocialJax shuffles its spawn points.
        rows = jnp.arange(14, dtype=jnp.int16).reshape(7, 2)
        shuffled = np.asarray(jax.random.permutation(key, rows))
        data["rows"].append({"key": k, "rows": [[int(v) for v in r] for r in shuffled]})
    return data


def dump(data):
    """One entry a line, so that a change shows as the entries it touches."""
    lines = ["{", f'  "jax": {json.dumps(data["jax"])},']
    parts = [p for p in data if p != "jax"]
    for j, part in enumerate(parts):
        lines.append(f'  "{part}": [')
        entries = data[part]
        for i, entry in enumerate(entries):
            comma = "," if i < len(entries) - 1 else ""
            lines.append("    " + json.dumps(entry, separators=(",", ":")) + comma)
        lines.append("  ]" + ("," if j < len(parts) - 1 else ""))
    lines.append("}")
    return "\n".join(lines) + "\n"


def main(args):
    text = dump(vectors())
    if args == ["--check"]:
        if OUT.read_text(encoding="utf-8") != text:
            sys.exit(f"{OUT.name} is not what random_vectors.py writes at JAX {jax.__version__}")
        print(f"{OUT.name} is what random_vectors.py writes at JAX {jax.__version__}")
    elif not args:
        OUT.write_text(text, encoding="utf-8", newline="\n")
        print(f"wrote {OUT.name}")
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main(sys.argv[1:])
