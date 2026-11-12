#!/usr/bin/env python3
"""Summarizes the phase 1 probe's runs (plans/rendering/phase1-plan.md).

    python3 -I plans/rendering/phase1-probe/summarize.py RESULTS_DIR [BEFORE AFTER]

Reads RESULTS_DIR/LABEL/run-N.tsv for every label and prints, per scenario
and metric, the median over the runs and the range of the per-run values.
It refuses incomplete evidence: a row repeated within a run or missing from
one, a value that is not finite and, for a comparison, fewer than five runs
or a different number on each side.

With BEFORE and AFTER labels it adds the plan's slowdown rule for the time
metrics (`*_ms`): a scenario counts as slower when the median of its AFTER
runs exceeds the largest BEFORE run by more than 5 % and by more than
0.05 ms. A smaller excess is marked "within noise", and a median below the
smallest BEFORE run by the same margins "faster". Counts (masks, commands)
are deterministic and are compared as values.
"""
import collections
import glob
import math
import os
import statistics
import sys


def load(results, label):
    """Every run's rows. Refuses evidence that is not complete: a run with
    a repeated row, a row missing from some run, or a value that is not
    finite."""
    runs = sorted(glob.glob(os.path.join(results, label, "run-*.tsv")))
    if not runs:
        sys.exit(f"{label}: no runs in {results}")
    values = collections.defaultdict(list)
    for path in runs:
        seen = set()
        with open(path) as f:
            for line in f:
                scenario, metric, value = line.rstrip("\n").split("\t")
                key = (scenario, metric)
                if key in seen:
                    sys.exit(f"{path}: {scenario} / {metric} appears twice")
                seen.add(key)
                number = float(value)
                if not math.isfinite(number):
                    sys.exit(f"{path}: {scenario} / {metric} is {value}")
                values[key].append(number)
    for (scenario, metric), v in values.items():
        if len(v) != len(runs):
            sys.exit(f"{label}: {scenario} / {metric} is in {len(v)} of {len(runs)} runs")
    return values, len(runs)


def fmt(v):
    return f"{v:.3f}" if abs(v) < 100 else f"{v:.0f}"


RELATIVE = 0.05
ABSOLUTE_MS = 0.05


def judge(before, after_median):
    high, low = max(before), min(before)
    if after_median > high:
        excess = after_median - high
        if excess > RELATIVE * high and excess > ABSOLUTE_MS:
            return "slower"
        return "within noise"
    if after_median < low:
        gain = low - after_median
        if gain > RELATIVE * low and gain > ABSOLUTE_MS:
            return "faster"
    return ""


def main():
    if len(sys.argv) not in (2, 4):
        sys.exit(__doc__)
    results = sys.argv[1]
    labels = sorted(
        d for d in os.listdir(results) if os.path.isdir(os.path.join(results, d))
    )
    if len(sys.argv) == 4:
        before, after = sys.argv[2], sys.argv[3]
        b, nb = load(results, before)
        a, na = load(results, after)
        if nb != na or nb < 5:
            sys.exit(f"the rule needs at least five paired runs; found {nb} and {na}")
        print(f"| Scenario | Metric | {before} median (max), {nb} runs | {after} median, {na} runs | Verdict |")
        print("|---|---|---|---|---|")
        for key in sorted(set(b) | set(a)):
            scenario, metric = key
            bv, av = b.get(key), a.get(key)
            if not bv or not av:
                print(f"| {scenario} | {metric} | {fmt(statistics.median(bv)) if bv else '–'} | {fmt(statistics.median(av)) if av else '–'} | only one side |")
                continue
            bm, am = statistics.median(bv), statistics.median(av)
            if metric.endswith("_ms"):
                verdict = judge(bv, am)
            else:
                verdict = "changed" if am != bm else ""
            print(f"| {scenario} | {metric} | {fmt(bm)} ({fmt(max(bv))}) | {fmt(am)} | {verdict} |")
        return
    for label in labels:
        values, n = load(results, label)
        print(f"\n## {label}: {n} runs\n")
        print("| Scenario | Metric | Median | Min–max |")
        print("|---|---|---|---|")
        for (scenario, metric), v in sorted(values.items()):
            print(f"| {scenario} | {metric} | {fmt(statistics.median(v))} | {fmt(min(v))}–{fmt(max(v))} |")


if __name__ == "__main__":
    main()
