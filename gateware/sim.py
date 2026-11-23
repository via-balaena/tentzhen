"""Running cocotb tests against Amaranth gateware on Verilator, from a pytest test: the gateware
becomes Verilog, Verilator builds it, and cocotb runs the named tests on it. Under pytest, cocotb's
runner fails the pytest test if a cocotb test fails, but passes one in which no test ran at all, so
`run` also checks which ran.

It reads cocotb's results file itself. A test the name filter leaves out is in that file too,
marked skipped (`_record_test_excluded`, cocotb/regression.py at 41564633, the cocotb in OSS CAD
Suite 2026-11-21), and `cocotb_tools.check_results.get_results` counts it among the tests."""

from xml.etree import ElementTree

from amaranth.back import verilog
from cocotb_tools.runner import get_runner


def ran_and_failed(results):
    """The names of the cocotb tests in the results file `results` that ran, and of those that
    failed. A test marked skipped did not run."""
    ran, failed = [], []
    for case in ElementTree.parse(results).getroot().iter("testcase"):
        if case.find("skipped") is not None:
            continue
        ran.append(case.get("name"))
        if case.find("failure") is not None or case.find("error") is not None:
            failed.append(case.get("name"))
    return ran, failed


def run(top, ports, name, test_module, tests, build_dir, platform=None, env=None):
    """Builds `top`, with `ports` as its toplevel module `name`'s ports, in `build_dir`, runs the
    cocotb tests named in `tests` from `test_module` on it, with the environment variables `env`
    added, and fails unless just those ran, each once, and passed."""
    source = build_dir / f"{name}.v"
    source.write_text(verilog.convert(top, name=name, ports=ports, platform=platform))
    runner = get_runner("verilator")
    runner.build(
        sources=[source],
        hdl_toplevel=name,
        build_dir=build_dir,
        build_args=["-Wno-fatal"],
        timescale=("1ns", "1ps"),
        always=True,
    )
    results = runner.test(
        test_module=test_module,
        testcase=tests,
        hdl_toplevel=name,
        build_dir=build_dir,
        extra_env=env or {},
    )
    ran, failed = ran_and_failed(results)
    assert (sorted(ran), failed) == (sorted(tests), []), (
        f"{test_module}: of the cocotb tests named, {tests}, these ran: {ran}; and these failed: "
        f"{failed}"
    )
