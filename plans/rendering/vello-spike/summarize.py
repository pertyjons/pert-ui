#!/usr/bin/env python3
"""Summarizes results/raw-workloads-*.md: per workload, scale and renderer,
the median over runs of each run's frame-total median and p95, the range of
the run medians, and the median over runs of each phase's median.

    python3 -I plans/rendering/vello-spike/summarize.py results/raw-workloads-*.md
"""
import re
import statistics
import sys

rows = {}  # (workload, scale, renderer) -> list of (total_med, total_p95, phases)
order = []
for path in sys.argv[1:]:
    workload = None
    for line in open(path, encoding="utf-8"):
        m = re.match(r"### (.+) — scale (\S+)", line)
        if m:
            workload = (m.group(1), m.group(2))
            continue
        if not workload or not line.startswith("| ") or line.startswith("| Renderer"):
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) != 5 or "/" not in cells[1]:
            continue
        renderer = cells[0]
        med, p95 = (float(v) for v in cells[1].split("/"))
        phases = []
        for part in cells[2].split("<br>"):
            name, value = part.rsplit(":", 1)
            phases.append((name.strip(), float(value.split("/")[0])))
        key = (*workload, renderer)
        if key not in rows:
            rows[key] = []
            order.append(key)
        rows[key].append((med, p95, phases))

current = None
for key in order:
    workload, scale, renderer = key
    if (workload, scale) != current:
        current = (workload, scale)
        print(f"\n### {workload} — scale {scale}\n")
        print("| Renderer | Frame total median (range over runs) | p95 | Phases (median) |")
        print("|---|---|---|---|")
    runs = rows[key]
    meds = [r[0] for r in runs]
    p95 = statistics.median(r[1] for r in runs)
    names = [n for n, _ in runs[0][2]]
    phases = "<br>".join(
        f"{n}: {statistics.median(r[2][i][1] for r in runs):.3f}" for i, n in enumerate(names)
    )
    print(
        f"| {renderer} | {statistics.median(meds):.3f} ({min(meds):.3f}–{max(meds):.3f}) | {p95:.3f} | {phases} |"
    )
print(f"\n{len(sys.argv) - 1} runs.")
