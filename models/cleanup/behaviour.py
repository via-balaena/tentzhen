"""Whether a student plays Clean Up as its teacher does, measure by measure (docs/littlebit.md,
"How small the policy can get"). From this folder:

    uv run --locked python behaviour.py <teacher.json> <other.json>...

Each file is what `rollouts.py` writes for 1,000 games on the same seeds. The bar is the trained
agents' behaviour, not their score, because the chip is to simulate the agents it was given: a
compressed policy that scores higher by acting differently is a different agent (Jon, 2026-11-24).
The measures adapt new-melting-pot's (sims/jaxtown/coop_report.py and compare.py, at 480062a,
Apache-2.0: apples, cleaning and zaps per 1,000 steps, and how many players did at least a tenth of
the cleaning), to what SocialJax's game records. It records no dirt cleaned per player, so a
player's cleaning here is the clean beams it fired. Each is taken per game of 1,000 steps:

- the town's return, all 7 agents' rewards;
- each player's apples and clean beams, ranked within the game, most first: since the 7 share one
  policy and are placed at random, a player is its rank, and who does the work shows there;
- zaps fired and times zapped, all 7;
- how many players fired at least a tenth of the game's clean beams (none if no beam was fired);
- the dirt the beams removed, all 7 together, since beams fired count tries, not dirt cleared
  (new-melting-pot's caution, 2026-11-24);
- the river's dirt and the apples on the map, each step on average;
- how often each of the 9 actions was taken, as a share of all 7,000.

The rule, set before any student was compared: a student matches the teacher on a measure if their
means differ by no more than 3 standard errors of the difference between two independent 1,000-game
means of the teacher's, 3 x sqrt(2) x the teacher's standard deviation / sqrt(1,000). It plays as the
teacher does if it matches on every measure. The rule is checked first on the teacher against
itself, on a second 1,000 games: with 30 measures, a policy identical to the teacher fails one by
chance about 8 % of the time. It also prints, for each, how closely a game's clean beams follow the
dirt they removed (their correlation over the games), to show whether beams stand for cleaning.
"""

import json
import math
import sys
from pathlib import Path

import numpy as np

CLEANER = 0.1
TOLERANCE = 3


def measures(path):
    """Each measure's value in each game, by name."""
    s = json.loads(Path(path).read_text())
    p = s["played"]
    names = p["actions_order"]
    apples = np.asarray(p["apples"], dtype=float)  # games x agents
    actions = np.asarray(p["actions"], dtype=float)  # games x agents x 9
    clean = actions[:, :, names.index("zap_clean")]
    zaps = actions[:, :, names.index("zap_forward")]
    total_clean = clean.sum(axis=1, keepdims=True)
    share = np.divide(clean, total_clean, out=np.zeros_like(clean), where=total_clean > 0)
    out = {"return": apples.sum(axis=1)}
    for k, a in enumerate(-np.sort(-apples, axis=1).T):
        out[f"apples, player {k + 1}"] = a
    for k, c in enumerate(-np.sort(-clean, axis=1).T):
        out[f"clean beams, player {k + 1}"] = c
    out["zaps fired"] = zaps.sum(axis=1)
    out["times zapped"] = np.asarray(p["zapped"], dtype=float).sum(axis=1)
    out["cleaners"] = (share >= CLEANER).sum(axis=1).astype(float)
    counted = np.asarray(p["counted_steps"], dtype=float)
    out["dirt removed"] = np.asarray(p["dirt_removed"], dtype=float)
    out["dirt a step"] = np.asarray(p["dirt"], dtype=float) / counted
    out["apples on the map a step"] = np.asarray(p["apples_on_map"], dtype=float) / counted
    moves = actions.sum(axis=1)
    for i, name in enumerate(names):
        out[f"share {name}"] = moves[:, i] / moves.sum(axis=1)
    beams = clean.sum(axis=1)
    out_check = {"beams and dirt removed, correlation": float(np.corrcoef(beams, out["dirt removed"])[0, 1]),
                 "dirt removed a beam": float(out["dirt removed"].sum() / max(beams.sum(), 1)),
                 "counted steps": sorted(set(counted.astype(int).tolist()))}
    return s["seeds"], out, out_check


def compare(teacher, other):
    """Each measure: the teacher's mean, the other's, the tolerance, and whether they match."""
    rows = []
    for name, t in teacher.items():
        o = other[name]
        tolerance = TOLERANCE * math.sqrt(2) * t.std(ddof=1) / math.sqrt(len(t))
        rows.append((name, t.mean(), o.mean(), tolerance, abs(o.mean() - t.mean()) <= tolerance))
    return rows


def main(teacher_path, others):
    seeds, teacher, check = measures(teacher_path)
    print(f"the teacher ({seeds}): {check}")
    for path in others:
        other_seeds, other, check = measures(path)
        rows = compare(teacher, other)
        failed = [r for r in rows if not r[4]]
        label = Path(path).stem
        print(f"\n{label} ({other_seeds}) against the teacher ({seeds}): "
              f"{len(rows) - len(failed)} of {len(rows)} measures match"
              + ("" if failed else ", so it plays as the teacher does"))
        print(f"  {check}")
        for name, t, o, tol, ok in rows:
            mark = "  " if ok else "X "
            print(f"  {mark}{name:28s} teacher {t:10.4f}  this {o:10.4f}  "
                  f"difference {o - t:+10.4f}  tolerance {tol:9.4f}")


if __name__ == "__main__":
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    main(sys.argv[1], sys.argv[2:])
