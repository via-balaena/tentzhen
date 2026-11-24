"""Clean Up played by the teacher or by a student, for the compression sweep (docs/littlebit.md,
"How small the policy can get"). From this folder, after fetch.py:

    uv run --locked python rollouts.py <socialjax-folder> <actor> <games> <first-seed> <out> [--record]

<actor> is `teacher`, SocialJax's checkpoint, or a student's file (`students.py`). The game is
`play.py`'s: 7 agents sharing the actor, 1,000 steps, individual rewards, games seeded <first-seed>
and on, all played at once, each agent drawing its action from the actor's logits, as SocialJax
plays. It writes <out>.json: each game's return, a game's return being its 7 agents' rewards
summed over its 1,000 steps. With --record, it also writes <out>.npz, what a student is trained
on: every agent's observation at every step, as uint8 (it refuses one that is not a whole number
from 0 to 255), and the teacher's logits for each, whoever acted.

The games' seeds keep training and scoring apart: the teacher's training games are seeded from 0,
and every actor is scored on the 1,000 games seeded 1,000 to 1,999.
"""

import argparse
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
import students

AGENTS = 7
STEPS = 1000


def play(socialjax_folder, actor, games, first_seed, record):
    fetch.check(socialjax_folder)
    sys.path.insert(0, str(Path(socialjax_folder).resolve()))
    from socialjax.environments.cleanup.clean_up import Clean_up

    env = Clean_up(
        num_agents=AGENTS, num_inner_steps=STEPS, shared_rewards=False, cnn=True, jit=True
    )
    teacher = policy.load(socialjax_folder)
    if actor == "teacher":
        act_logits = lambda obs: policy.logits(teacher, obs)  # noqa: E731
    else:
        spec, params = students.load(actor)
        act_logits = lambda obs: students.logits(spec, params, obs)  # noqa: E731

    def game(seed):
        key, reset = jax.random.split(jax.random.PRNGKey(seed))
        obs, state = env.reset(reset)

        def step(carry, _):
            obs, state, key = carry
            key, choose, move = jax.random.split(key, 3)
            out = act_logits(obs)
            actions = jax.random.categorical(choose, out, axis=1)
            seen = ()
            if record:
                whole = jnp.all((obs == jnp.round(obs)) & (obs >= 0) & (obs <= 255))
                taught = out if actor == "teacher" else policy.logits(teacher, obs)
                seen = (obs.astype(jnp.uint8), taught, whole)
            obs, state, reward, _, _ = env.step(move, state, actions)
            return (obs, state, key), (reward, seen)

        _, (rewards, seen) = jax.lax.scan(step, (obs, state, key), None, length=STEPS)
        return rewards.sum(axis=0), seen

    seeds = jnp.arange(first_seed, first_seed + games)
    rewards, seen = jax.jit(jax.vmap(game))(seeds)
    return np.asarray(rewards), jax.tree.map(np.asarray, seen)


def main(args):
    start = time.perf_counter()
    rewards, seen = play(args.socialjax, args.actor, args.games, args.first_seed, args.record)
    seconds = time.perf_counter() - start
    returns = rewards.sum(axis=1)
    out = Path(args.out)
    summary = {
        "game": "SocialJax Clean Up: 7 agents, 1,000 steps, individual rewards",
        "socialjax": {"repo": fetch.PINS["repo"], "commit": fetch.PINS["commit"]},
        "actor": args.actor if args.actor == "teacher" else str(Path(args.actor).resolve()),
        "games": args.games,
        "seeds": f"{args.first_seed} to {args.first_seed + args.games - 1}",
        "returns": [float(r) for r in returns],
        "return_mean": float(returns.mean()),
        "return_std": float(returns.std(ddof=1)) if args.games > 1 else None,
        "recorded": None,
        "seconds": seconds,
        "versions": {"python": platform.python_version(), "jax": jax.__version__,
                     "numpy": np.__version__},
        "device": str(jax.devices()[0]),
    }
    if args.record:
        obs, taught, whole = seen
        if not whole.all():
            sys.exit("an observation held a value that is not a whole number from 0 to 255")
        # (games, steps, agents, ...) to one row an observation.
        obs = obs.reshape(-1, *obs.shape[3:])
        taught = taught.reshape(-1, taught.shape[-1]).astype(np.float32)
        np.savez(out.with_suffix(".npz"), observations=obs, logits=taught)
        summary["recorded"] = {"file": out.with_suffix(".npz").name, "observations": len(obs)}
    out.with_suffix(".json").write_text(json.dumps(summary, indent=2) + "\n", newline="\n")
    print(f"{summary['actor']}: {args.games} games from seed {args.first_seed}, return "
          f"{summary['return_mean']:.1f} (sd {summary['return_std']:.1f}), {seconds:.1f} s")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("socialjax")
    parser.add_argument("actor")
    parser.add_argument("games", type=int)
    parser.add_argument("first_seed", type=int)
    parser.add_argument("out")
    parser.add_argument("--record", action="store_true")
    main(parser.parse_args())
