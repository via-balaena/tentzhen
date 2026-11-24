"""How fast SocialJax's Clean Up runs here, with and without its trained policy choosing the moves:
the bar a hardwired policy would have to beat for running simulations with trained agents
(docs/littlebit.md, "After MNIST: Clean Up's policy"). From this folder, after fetch.py:

    uv run --locked python bench.py <socialjax-folder> [<games at once>...]

Adapted from new-melting-pot's `sims/jaxtown/bench.py` (github.com/YoungMoolahBaby/new-melting-pot
at 1f3cc3b, Apache-2.0), which times the game alone. For each number of games at once, and for each
of two loads, it compiles a run of 1,000 steps ahead, runs it once to warm up, then times it five
times and reports the median, with the fastest and slowest beside it (one timing under WSL varies
from run to run, new-melting-pot found):

- random: the game alone, each agent's move drawn uniformly, as new-melting-pot times it;
- policy: the game, with every agent's move drawn from SocialJax's trained policy, the network's
  forward pass for all 7 agents of every game each step.

A game step is all 7 agents acting once, so one game step is 7 agent decisions. Each run keeps a sum
of the rewards and of the observations, so that the compiler can't skip computing them. On an
NVIDIA GPU it also reads the board's power draw while it times (`Power`), for the energy a game
step takes. Run on the GPU, nothing else may use it: two JAX processes sharing one under WSL slow
each other about 30 times, as new-melting-pot found.
"""

import os
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
from datetime import datetime
from pathlib import Path

os.environ.setdefault("XLA_PYTHON_CLIENT_PREALLOCATE", "false")

import jax  # noqa: E402
import jax.numpy as jnp  # noqa: E402

import fetch  # noqa: E402
import policy  # noqa: E402

AGENTS = 7
STEPS = 1000
GAMES = (1, 16, 256, 1024)
TIMINGS = 5
IDLE_S = 3


class Power:
    """The GPU board's power draw while the benchmark runs, read every 100 ms: nvidia-smi's
    `power.draw.instant` and `power.draw.average` (over the last second), each of which its own
    help gives as accurate to within 5 W. It is the board alone, not the rest of the computer."""

    def __init__(self):
        self.log = tempfile.TemporaryFile(mode="w+")
        self.process = subprocess.Popen(
            ["nvidia-smi", "--query-gpu=timestamp,power.draw.instant,power.draw.average",
             "--format=csv,noheader,nounits", "-lms", "100"],
            stdout=self.log, stderr=subprocess.DEVNULL, text=True,
        )

    def stop(self):
        """Each reading as (seconds since the epoch, instant watts, 1-second average watts)."""
        self.process.terminate()
        self.process.wait()
        self.log.seek(0)
        readings = []
        for line in self.log:
            fields = [f.strip() for f in line.split(",")]
            try:
                when = datetime.strptime(fields[0], "%Y/%m/%d %H:%M:%S.%f").timestamp()
                readings.append((when, float(fields[1]), float(fields[2])))
            except (ValueError, IndexError):
                continue
        return readings


def mean_watts(readings, start, end):
    """The mean (instant, 1-second average) watts of the readings from start to end."""
    inside = [r for r in readings if start <= r[0] <= end]
    if not inside:
        return None
    return statistics.mean(r[1] for r in inside), statistics.mean(r[2] for r in inside)


def make_run(env, params, games, load):
    """A run of `games` games for STEPS steps under `load`, jitted."""

    def run(key):
        key, k = jax.random.split(key)
        obs, state = jax.vmap(env.reset)(jax.random.split(k, games))

        def step(carry, _):
            obs, state, key = carry
            key, choose, move = jax.random.split(key, 3)
            if load == "random":
                actions = jax.random.randint(choose, (games, AGENTS), 0, policy.ACTIONS)
            else:
                flat = obs.reshape(games * AGENTS, *obs.shape[2:])
                actions = policy.act(params, flat, "sampled", choose).reshape(games, AGENTS)
            obs, state, reward, _, _ = jax.vmap(env.step)(
                jax.random.split(move, games), state, actions
            )
            return (obs, state, key), (reward.sum(), obs.sum(dtype=jnp.float32))

        _, (rewards, obs_sums) = jax.lax.scan(step, (obs, state, key), None, STEPS)
        return rewards.sum(), obs_sums.sum()

    return jax.jit(run)


def main(socialjax_folder, games):
    fetch.check(socialjax_folder)
    sys.path.insert(0, str(Path(socialjax_folder).resolve()))
    from socialjax.environments.cleanup.clean_up import Clean_up

    env = Clean_up(
        num_agents=AGENTS, num_inner_steps=STEPS, shared_rewards=False, cnn=True, jit=True
    )
    params = policy.load(socialjax_folder)
    print("JAX", jax.__version__, "on", jax.devices())
    power = Power() if jax.default_backend() == "gpu" and shutil.which("nvidia-smi") else None
    idle = (time.time(), time.time() + IDLE_S)
    if power:
        time.sleep(IDLE_S)
    rows = []
    for n in games:
        for load in ("random", "policy"):
            run = make_run(env, params, n, load)
            start = time.perf_counter()
            compiled = run.lower(jax.random.PRNGKey(0)).compile()
            compile_s = time.perf_counter() - start
            jax.block_until_ready(compiled(jax.random.PRNGKey(1)))
            seconds, rewards = [], []
            window = time.time()
            for t in range(TIMINGS):
                start = time.perf_counter()
                reward, _ = jax.block_until_ready(compiled(jax.random.PRNGKey(2 + t)))
                seconds.append(time.perf_counter() - start)
                rewards.append(float(reward) / n)
            window = (window, time.time())
            rates = sorted(n * STEPS / s for s in seconds)
            steps, slowest, fastest = statistics.median(rates), rates[0], rates[-1]
            rows.append((n, load, steps, slowest, fastest, window, sum(seconds)))
            print(f"{n:5d} games, {load:6s}: {statistics.median(seconds):8.2f} s, "
                  f"{steps:12,.0f} game steps/s ({slowest:,.0f} to {fastest:,.0f}), "
                  f"{AGENTS * steps:13,.0f} agent decisions/s (compile {compile_s:.1f} s, "
                  f"return {statistics.mean(rewards):.1f} a game)", flush=True)
    readings = power.stop() if power else []
    print(f"\nMedian of {TIMINGS} timings, with the slowest and fastest.")
    if readings:
        still = mean_watts(readings, *idle)
        print(f"Board power: nvidia-smi's power.draw.instant and power.draw.average every 100 ms "
              f"(each to within 5 W), averaged over each row's {TIMINGS} timings; idle, with JAX "
              f"loaded: {still[0]:.1f} W instant, {still[1]:.1f} W average. The energy is the "
              f"instant watts times the {TIMINGS} timings' seconds, over their game steps.")
    print("\n| games at once | load | game steps a second | slowest to fastest "
          "| agent decisions a second |" + (" board watts, instant / 1 s average "
                                            "| millijoules a game step |" if readings else ""))
    print("|---|---|---|---|---|" + ("---|---|" if readings else ""))
    for n, load, steps, slowest, fastest, window, total_s in rows:
        line = (f"| {n:,} | {load} | {steps:,.0f} | {slowest:,.0f} to {fastest:,.0f} "
                f"| {AGENTS * steps:,.0f} |")
        if readings:
            watts = mean_watts(readings, *window)
            if watts is None:
                line += " - | - |"
            else:
                joules = watts[0] * total_s / (TIMINGS * n * STEPS)
                line += f" {watts[0]:.1f} / {watts[1]:.1f} | {1000 * joules:.3f} |"
        print(line)


if __name__ == "__main__":
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    main(sys.argv[1], [int(g) for g in sys.argv[2:]] or list(GAMES))
