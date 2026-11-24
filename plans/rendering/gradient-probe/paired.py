"""Run uninstrumented phase1 binaries in counterbalanced pairs on Linux."""
import argparse
import os
import subprocess
from pathlib import Path


def run(binary, output, temporary, cores, workload):
    for attempt in range(3):
        with output.with_suffix(".log").open("w") as log:
            result = subprocess.run(
                ["taskset", "-c", cores, binary],
                env=dict(os.environ, PROBE_ONLY=workload, PROBE_TMP=str(temporary),
                         PROBE_TSV=str(output)),
                stdout=log, stderr=subprocess.STDOUT,
            )
        if result.returncode == 0:
            return
        print(f"{output.name}: attempt {attempt+1} exited {result.returncode}", flush=True)
    raise RuntimeError(f"scenario did not finish: {output}")


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("reference", type=Path)
parser.add_argument("candidate", type=Path)
parser.add_argument("output", type=Path)
parser.add_argument("--cores", default="6,8")
parser.add_argument("--workload", choices=["mixer-diagram", "zoom-present"], default="mixer-diagram")
args = parser.parse_args()
out = args.output.resolve()
out.mkdir(parents=True, exist_ok=True)
tmp = out / "tmp"
tmp.mkdir(exist_ok=True)
for pair in range(10):
    samples = [("a", args.reference), ("b", args.candidate)]
    if pair % 2:
        samples.reverse()
    for label, exe in samples:
        print(pair+1, label, Path("/proc/loadavg").read_text().strip(), flush=True)
        run(exe.resolve(), out / f"{label}-{pair+1}.tsv", tmp, args.cores, args.workload)
