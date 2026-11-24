"""SocialJax at the commit socialjax.toml pins, fetched into a folder named on the command line
and checked: the commit checked out is the pin, and the Clean Up checkpoint's sha256 is its pin.
Its files are not committed here; the folder goes on PYTHONPATH for play.py. From this folder:

    uv run --locked python fetch.py <folder>
"""

import hashlib
import subprocess
import sys
import tomllib
from pathlib import Path

PINS = tomllib.loads(Path(__file__).with_name("socialjax.toml").read_text())


def check(folder):
    """Refuses a checkout whose commit or checkpoint is not the pin. The commit is read from
    .git/HEAD, which `fetch` leaves holding the pinned commit itself (a detached checkout), so the
    check needs no git: git in WSL refuses a repo cloned from Windows, as another user's."""
    folder = Path(folder)
    head = (folder / ".git" / "HEAD").read_text().strip()
    if head != PINS["commit"]:
        raise ValueError(f"{folder}: at {head}, not the pinned {PINS['commit']}")
    checkpoint = folder / PINS["checkpoint"]["path"]
    got = hashlib.sha256(checkpoint.read_bytes()).hexdigest()
    if got != PINS["checkpoint"]["sha256"]:
        raise ValueError(f"{checkpoint.name}: its sha256 is {got}, not the pinned one")


def fetch(folder):
    """Clones SocialJax into `folder` unless it is there, checks out the pin, and checks it."""
    folder = Path(folder)
    if not folder.exists():
        subprocess.run(["git", "clone", "--quiet", PINS["repo"], str(folder)], check=True)
    subprocess.run(
        ["git", "-c", "advice.detachedHead=false", "checkout", "--quiet", PINS["commit"]],
        cwd=folder,
        check=True,
    )
    check(folder)


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    fetch(sys.argv[1])
    print(f"{sys.argv[1]}: SocialJax at {PINS['commit'][:12]}, its Clean Up checkpoint as pinned")
