"""SocialJax's Clean Up played by its trained policy, the float baseline the hardwired policy is
to stand beside (docs/littlebit.md, "After MNIST: Clean Up's policy"). From this folder, after
fetch.py:

    uv run --locked python play.py <socialjax-folder> (sampled | greedy | random) <games> <out.json>

Chosen before the first run: the game as the checkpoint was trained on it (7 agents, 1,000 steps,
individual rewards, SocialJax's other defaults; algorithms/IPPO/config/ippo_cnn_cleanup.yaml), the
one policy shared by all 7 agents, and games seeded 0, 1, 2 and on, all played at once. An agent
acts by drawing from the policy ("sampled", as SocialJax plays it), by taking its most likely
action ("greedy", as the chip will pick), or uniformly ("random", for scale). Each game's return
is its 7 agents' rewards summed over its 1,000 steps. It also records each of the observation's 19
channels' largest value over every step of every game, and whether every value was 0 or 1: what
the chip's input has to hold. And it counts how often each of the 9 actions was taken.
"""

import json
import platform
import sys
import time
from pathlib import Path

import jax
import jax.numpy as jnp
import numpy as np

import fetch
import policy

AGENTS = 7
STEPS = 1000
MODES = ("sampled", "greedy", "random")


def play(socialjax_folder, mode, games):
    """Each game's rewards per agent, (games, 7); whether every observation was 0 or 1; and each
    channel's largest value."""
    fetch.check(socialjax_folder)
    sys.path.insert(0, str(Path(socialjax_folder).resolve()))
    from socialjax.environments.cleanup.clean_up import Clean_up

    env = Clean_up(
        num_agents=AGENTS, num_inner_steps=STEPS, shared_rewards=False, cnn=True, jit=True
    )
    params = policy.load(socialjax_folder)

    def game(seed):
        key, reset = jax.random.split(jax.random.PRNGKey(seed))
        obs, state = env.reset(reset)

        def step(carry, _):
            obs, state, key = carry
            key, choose, move = jax.random.split(key, 3)
            binary = jnp.all((obs == 0) | (obs == 1))
            largest = obs.max(axis=(0, 1, 2))
            actions = policy.act(params, obs, mode, choose)
            counts = jnp.bincount(actions, length=policy.ACTIONS)
            obs, state, reward, _, _ = env.step(move, state, actions)
            return (obs, state, key), (reward, binary, largest, counts)

        _, (rewards, binary, largest, counts) = jax.lax.scan(
            step, (obs, state, key), None, length=STEPS
        )
        return rewards.sum(axis=0), jnp.all(binary), largest.max(axis=0), counts.sum(axis=0)

    rewards, binary, largest, counts = jax.jit(jax.vmap(game))(jnp.arange(games))
    return (
        np.asarray(rewards),
        bool(np.all(binary)),
        np.asarray(largest).max(axis=0),
        np.asarray(counts).sum(axis=0),
    )


def main(socialjax_folder, mode, games, out):
    if mode not in MODES:
        sys.exit(__doc__)
    start = time.perf_counter()
    rewards, binary, largest, counts = play(socialjax_folder, mode, games)
    seconds = time.perf_counter() - start
    returns = rewards.sum(axis=1)
    summary = {
        "game": "SocialJax Clean Up: 7 agents, 1,000 steps, individual rewards",
        "socialjax": {"repo": fetch.PINS["repo"], "commit": fetch.PINS["commit"]},
        "checkpoint": fetch.PINS["checkpoint"],
        "mode": mode,
        "games": games,
        "seeds": f"0 to {games - 1}",
        "returns": [float(r) for r in returns],
        "return_mean": float(returns.mean()),
        "return_std": float(returns.std(ddof=1)) if games > 1 else None,
        "agent_means": [float(r) for r in rewards.mean(axis=0)],
        "observations_binary": binary,
        "channel_largest": [float(v) for v in largest],
        "action_counts": [int(c) for c in counts],
        "seconds": seconds,
        "versions": {
            "python": platform.python_version(),
            "jax": jax.__version__,
            "numpy": np.__version__,
        },
        "device": str(jax.devices()[0]),
    }
    Path(out).write_text(json.dumps(summary, indent=2) + "\n", newline="\n")
    print(
        f"{mode}: {games} games, return {summary['return_mean']:.1f} "
        f"(sd {summary['return_std']:.1f}), observations all 0 or 1: {binary}, {seconds:.1f} s"
    )


if __name__ == "__main__":
    if len(sys.argv) != 5:
        sys.exit(__doc__)
    main(sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4])
