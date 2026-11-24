"""Order statistics for ten counterbalanced phase1 measurement pairs."""
import argparse
import statistics
from pathlib import Path


def sample(path):
    metrics = {"cpu_median_ms", "present_median_ms", "gpu_median_ms"}
    values = {}
    for line in path.read_text().splitlines():
        scene, metric, value = line.split("\t")
        if metric in metrics:
            values[scene, metric] = float(value)
    return values


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("directory", type=Path)
parser.add_argument("--consecutive", action="store_true", help="read legacy aa-1..aa-20 samples")
args = parser.parse_args()
ratios = {}
for pair in range(1, 11):
    files = ([f"aa-{2*pair-1}.tsv", f"aa-{2*pair}.tsv"] if args.consecutive
             else [f"a-{pair}.tsv", f"b-{pair}.tsv"])
    before, after = (sample(args.directory / file) for file in files)
    if not before or before.keys() != after.keys():
        raise ValueError(f"missing or unmatched metrics for pair {pair}")
    for key in before:
        ratios.setdefault(key, []).append(after[key] / before[key])
print("scenario\tmetric\tsecond\tmedian\tninth")
for (scene, metric), values in sorted(ratios.items()):
    ordered = sorted(values)
    print(scene, metric, *(f"{v:.4f}" for v in
          (ordered[1], statistics.median(ordered), ordered[8])), sep="\t")
