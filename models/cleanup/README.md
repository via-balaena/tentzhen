# models/cleanup

SocialJax's Clean Up and its trained policy, run on the computer: the float baseline for project
#3's first real workload (`docs/littlebit.md`, "After MNIST: Clean Up's policy"). The chip is for
running simulations with agents already trained, so the policy is SocialJax's own checkpoint, not
one trained here.

## Setting it up

You need [uv](https://docs.astral.sh/uv/) and git. From this folder:

    uv sync --locked
    uv run --locked python fetch.py D:\tentzhen-downloads\SocialJax

The first installs Python 3.12 and what `uv.lock` pins: JAX 0.6.2 on the CPU, Flax 0.10.7, chex,
NumPy and Pillow, the versions SocialJax pins. SocialJax's own list asks for `jax[cuda12]`, which
has no Windows build, and for much its environments never import. The second clones SocialJax
(Apache-2.0) at the commit `socialjax.toml` pins, outside the repo, and refuses it unless the
commit and the checkpoint's sha256 are the pins. To keep the environment off a small system drive,
point uv at another one first, as `models/littlebit/README.md` does, with
`UV_PROJECT_ENVIRONMENT` set to a folder of its own.

## Loading the policy

The checkpoint is a pickle. Unpickling can run any code a file names, so `policy.load` allows only
the three classes this one names, NumPy's array reconstruction, its array and its dtype, and
refuses any other. `pickletools`, which runs nothing, read those three from it (2026-11-23).

## Playing it

    uv run --locked python play.py D:\tentzhen-downloads\SocialJax sampled 100 sampled.json

plays 100 games of Clean Up as the checkpoint was trained on it: 7 agents sharing the one policy,
1,000 steps, individual rewards. Each agent acts by drawing from the policy (`sampled`, as SocialJax
plays it), by its most likely action (`greedy`), or uniformly (`random`). It writes each game's
return and what the observations held. The results are in `docs/littlebit.md`, "The float
baseline".
