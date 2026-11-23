# models/littlebit

Project #3's model training, in PyTorch (`docs/littlebit.md`, "Languages and layout"). E1, the
FP32 baseline, came first. The LittleBit layer at r = 16 came next (`docs/littlebit.md`, "The
first LittleBit run"), then the baselines E2 and E3 ("The baselines, E2 and E3"), the rest of
the rank sweep, and E10, a wider network ("E10, the wider network").

## Running E1

Fetch MNIST into a folder outside the repo, and train. `mnist.py` refuses any file whose sha256 is
not its pin in `mnist.toml`; MNIST's terms are unread, so its files are never committed.

    uv run --locked python mnist.py D:\tentzhen-data\mnist
    uv run --locked python e1_fp32.py D:\tentzhen-data\mnist runs\e1

The second writes `runs/e1/weights.f32` (every parameter, as little-endian float32) and
`runs/e1/summary.json` (what was run and what it scored, with the weights' sha256), which are
committed. On the lab computer it took 16 s and got 9,831 of the 10,000 test images right
(98.31 %); six runs gave the same weights, byte for byte (2026-11-22).

That figure is `tested`: `e1_scores_what_its_summary_says`, in `crates/littlebit`, re-scores the
committed weights with its own arithmetic, and the Quality Gate runs it. To run it yourself, point
`MNIST_DIR` at the folder MNIST is in; the tests that need MNIST fail without it, rather than pass
unchecked:

    $env:MNIST_DIR = "D:\tentzhen-data\mnist"
    cargo test -p tentzhen-littlebit

## Running the LittleBit layer

`littlebit.py` is the layer, written from the paper's text. `train_rank.py` trains the MLP with
its two hidden layers made LittleBit layers at one rank. It starts from E1's committed weights in
`runs/e1`:

    uv run --locked python train_rank.py D:\tentzhen-data\mnist 16 runs\r16

It writes the same two files as E1, to `runs/r16`. On the lab computer it took about 50 s and got
9,713 of the 10,000 test images right (97.13 %, 1.18 points under E1). Three runs gave the same
weights, byte for byte (2026-11-22). `r16_scores_what_its_summary_says` re-scores them, with the
same `MNIST_DIR`. Where the recipe is silent, what the run chose is in `train_rank.py`'s docstring
and in `docs/littlebit.md`, "The first LittleBit run".

The rest of the sweep runs the same way, with only the rank changed: `4`, `8`, `32` and `64`
write `runs/r4`, `runs/r8`, `runs/r32` and `runs/r64`. Each took about a minute, and two runs at
each rank gave the same weights, byte for byte (2026-11-23). In float they got 8,802, 9,464,
9,772 and 9,814 of the test images right, which `the_sweep_scores_what_its_summaries_say`
re-scores. Each is exported as r16 is, below. E11's search for the largest rank that fits the
board added `runs/r40`, `r42`, `r43`, `r44`, `r48` and `r56` the same way (`docs/littlebit.md`,
"E11, the largest rank that fits").

## Running the baselines

`train_baseline.py` trains E2 and E3, the baselines between E1 and the LittleBit sweep. It takes
E1's MLP with its two hidden layers' weights int8 (E2) or binary with one scale per output (E3),
and fine-tunes it from `runs/e1` as `train_rank.py` does, with the same steps:

    uv run --locked python train_baseline.py D:\tentzhen-data\mnist int8 runs\e2
    uv run --locked python train_baseline.py D:\tentzhen-data\mnist binary runs\e3

On the lab computer they took 24 s and 38 s. E2 got 9,835 of the 10,000 test images right in float
(98.35 %), and E3 got 9,824 (98.24 %). Two runs of each gave the same weights, byte for byte
(2026-11-23). `e2_scores_what_its_summary_says` and `e3_scores_what_its_summary_says` re-score
them, with the same `MNIST_DIR`. Where they differ from r16's recipe is in `train_baseline.py`'s
docstring and in `docs/littlebit.md`, "The baselines, E2 and E3". Moving the shared steps out of
`train_rank.py` left r16's weights as they were: a run after the change gave the committed
`weights.f32`, byte for byte.

## Running E10, the wider network

E10 is the 784 → 1024 → 512 → 10 MLP at r = 9, the largest rank whose hidden layers hold no more
bits than E7's (`docs/littlebit.md`, "E10, the wider network"). It needs a start of its own
shape, trained as E1 was, and then the LittleBit run on it:

    uv run --locked python e1_fp32.py D:\tentzhen-data\mnist runs\e10-start E10-start 1024 512
    uv run --locked python train_rank.py D:\tentzhen-data\mnist 9 runs\e10 E10 runs\e10-start

On the lab computer they took 17 s and 55 s. The start got 9,856 of the 10,000 test images right
in float (98.56 %), and E10 got 9,555 (95.55 %). Two runs of each gave the same weights, byte for
byte (2026-11-23). `e10_scores_what_its_summaries_say` re-scores both, with the same `MNIST_DIR`.
E10 is exported as r16 is, below. Giving the two scripts a shape left E1, r16 and E2 as they were:
a run of each after the change gave its committed `weights.f32`, byte for byte.

## Training with the signs fixed

`train_frozen.py` starts from r16 and fine-tunes it on a task with only some parameters free: none,
the output layer, the scales (every sign fixed), or everything (`docs/littlebit.md`, "Training with
the signs fixed"). Fashion-MNIST is one of its tasks: fetch its four files from
zalandoresearch/fashion-mnist's `data/fashion` into a folder outside the repo, and the script checks
each against its sha256. For example:

    uv run --locked python train_frozen.py D:\tentzhen-data\mnist D:\tentzhen-data\fashion-mnist fashion scales D:\tentzhen-build\frozen\fashion-scales

It writes a summary.json, what was free and what it scored, with the weights' sha256, to the out
folder. Its runs aren't committed, and no test re-scores them: their counts are PyTorch's. On the
lab computer each took 22 to 54 s, and two runs of each gave the same weights, byte for byte
(2026-11-23).

## The integer export

The hardware doesn't run float. The exporter turns a LittleBit run into integers: int8 scales,
signs, int32 biases and the shifts between them. It writes them beside the run as `integer.json`,
the one file the golden model and, later, the gateware read. It's in Rust, in `crates/littlebit`,
and needs no PyTorch. From the repo root, with `MNIST_DIR` set as above:

    cargo run -p tentzhen-littlebit -- export models/littlebit/runs/r16

It also prints what the integer network gets right. At r = 16 that's 9,717 of the 10,000 test
images, 4 more than the float run's 9,713: check 1, `tested` by
`r16_integer_is_within_half_a_point_of_float`. The Quality Gate re-runs the export and fails if
the committed `integer.json` is not what it writes (`r16_integer_json_is_what_the_exporter_writes`).
How each value is chosen is in `docs/littlebit.md`, "Fixed-point formats".

Run on E2 or E3, the same command prints what the integer network gets right and writes no file:
a baseline runs on the computer only, so no gateware reads its integers. E2 gets 9,833 right and
E3 gets 9,824 (`e2_and_e3_integer_are_within_half_a_point_of_float`).

## Setting it up

You need [uv](https://docs.astral.sh/uv/). From this folder:

    uv sync --locked

That installs Python 3.12.15 and the packages `uv.lock` pins: PyTorch 2.15.0 and NumPy 2.5.5
among them. On Windows and Linux PyTorch is its CUDA 13.2 build, from PyTorch's own index; on
macOS it is PyPI's. `--locked` stops if `uv.lock` no longer matches `pyproject.toml`, so nothing
installs that the lock does not name.

On Windows the environment came to 3.0 GB (2026-11-22). To keep it off a small system drive, point
uv at another one before you run it, in PowerShell for example:

    $env:UV_CACHE_DIR = "D:\uv\cache"
    $env:UV_PYTHON_INSTALL_DIR = "D:\uv\python"
    $env:UV_PROJECT_ENVIRONMENT = "D:\venvs\tentzhen-littlebit"

To see that PyTorch finds the GPU:

    uv run --locked python -c "import torch; print(torch.__version__, torch.cuda.is_available())"

On the lab computer, an RTX 4070 Ti with driver 621.01, that printed `2.15.0+cu132 True`. A GPU is
not required: the networks in `docs/littlebit.md` are small, and PyTorch runs them on the CPU too.
How long a run takes either way is unknown until one is timed.
