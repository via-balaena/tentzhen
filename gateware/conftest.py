"""pytest's setup for the gateware's cocotb tests: the repo root goes on the import path, so a test
can import `gateware.*`, and so can the simulator's Python, which cocotb's runner gives this path."""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
