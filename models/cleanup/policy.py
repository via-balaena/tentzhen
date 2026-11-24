"""SocialJax's trained Clean Up policy, loaded safely and run as SocialJax runs it.

The checkpoint is a pickle, and unpickling can run any code a file names. This one names three
classes only, NumPy's array reconstruction, its array and its dtype (read with pickletools, which
runs nothing, 2026-11-23). `load` refuses any other, so loading it runs nothing else.

`ActorCritic` is the network as SocialJax's `algorithms/utils/networks.py` builds it at the pinned
commit (Apache-2.0): three convolutions of 32 channels, 5 x 5 then 3 x 3 twice, a dense layer of
64, then an actor head of 64 and 9 actions, and a critic head of 64 and 1, each but the last of a
head followed by ReLU. Flax's own `Conv` and `Dense`, at their defaults (stride 1, "same" padding),
give it the same parameter names as the checkpoint. Its distribution over actions, which SocialJax
gets from distrax, is the actor's logits here.
"""

import hashlib
import io
import pickle
import tomllib
from pathlib import Path

import flax.linen as nn
import jax
import jax.numpy as jnp

PINS = tomllib.loads(Path(__file__).with_name("socialjax.toml").read_text())
ALLOWED = {("numpy.core.multiarray", "_reconstruct"), ("numpy", "ndarray"), ("numpy", "dtype")}
ACTIONS = 9


class _Arrays(pickle.Unpickler):
    """An unpickler that builds NumPy arrays and refuses every other class."""

    def find_class(self, module, name):
        if (module, name) not in ALLOWED:
            raise pickle.UnpicklingError(f"the checkpoint names {module}.{name}, which is refused")
        return super().find_class(module, name)


def load(socialjax_folder):
    """The Clean Up checkpoint's parameters, a nested dict of float32 arrays, after its sha256 is
    checked against its pin."""
    data = (Path(socialjax_folder) / PINS["checkpoint"]["path"]).read_bytes()
    got = hashlib.sha256(data).hexdigest()
    if got != PINS["checkpoint"]["sha256"]:
        raise ValueError(f"the checkpoint's sha256 is {got}, not the pinned one")
    return _Arrays(io.BytesIO(data)).load()


class CNN(nn.Module):
    @nn.compact
    def __call__(self, x):
        x = nn.relu(nn.Conv(32, (5, 5))(x))
        x = nn.relu(nn.Conv(32, (3, 3))(x))
        x = nn.relu(nn.Conv(32, (3, 3))(x))
        x = x.reshape((x.shape[0], -1))
        return nn.relu(nn.Dense(64)(x))


class ActorCritic(nn.Module):
    @nn.compact
    def __call__(self, x):
        # Flax names each layer in the order it is built, so they are built in SocialJax's order:
        # the actor's Dense_0 and Dense_1, then the critic's Dense_2 and Dense_3.
        embedding = CNN()(x)
        actor = nn.relu(nn.Dense(64)(embedding))
        actor = nn.Dense(ACTIONS)(actor)
        critic = nn.relu(nn.Dense(64)(embedding))
        critic = nn.Dense(1)(critic)
        return actor, critic[:, 0]


def logits(params, observations):
    """The actor's logits for a batch of observations, each 11 x 11 x 19."""
    return ActorCritic().apply(params, observations)[0]


def act(params, observations, mode, key):
    """One action per observation: drawn from the policy's distribution ("sampled", as SocialJax
    plays it), its most likely, the first of a tie ("greedy", as the chip will pick), or uniform
    ("random")."""
    if mode == "random":
        return jax.random.randint(key, observations.shape[:1], 0, ACTIONS)
    out = logits(params, observations)
    if mode == "greedy":
        return jnp.argmax(out, axis=1)
    return jax.random.categorical(key, out, axis=1)
