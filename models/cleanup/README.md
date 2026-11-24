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

## JAX's random numbers, for the golden model

    uv run --locked python random_vectors.py
    uv run --locked python random_vectors.py --check

The first has JAX draw from 9 keys every way Clean Up draws (`split`, `uniform`, `randint` and
`permutation`, and the Threefry hash beneath them) and writes what it drew to
`random_vectors.json`. The golden model's copy of JAX's random numbers, `crates/cleanup`, has to
draw the same, bit for bit (`jax_s_random_numbers_are_these_bit_for_bit`). The second fails unless
the committed file is what this JAX writes. It needs no SocialJax. The Quality Gate has no JAX, so
it checks the Rust against the file, and the file against JAX is checked by hand.

## Timing it

    uv run --locked python bench.py D:\tentzhen-downloads\SocialJax

times the game at 1, 16, 256 and 1,024 games at once, with moves drawn at random and with the
policy choosing them, as the median of 5 timings of 1,000 steps. On an NVIDIA GPU it also reads the
board's power through nvidia-smi. JAX's CUDA build has no Windows wheel, so on the GPU it runs under
WSL2, with the `cuda` extra (`uv run --locked --extra cuda ...`). Nothing else may use the GPU while
it runs: two JAX processes sharing one under WSL slow each other about 30 times. The results are in
`docs/littlebit.md`, "The goal: cheaper than a GPU per game step".
