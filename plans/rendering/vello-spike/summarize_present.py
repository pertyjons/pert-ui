#!/usr/bin/env python3
"""Summarizes results/raw-present-*.md: per workload, scale and renderer,
the median over runs of the UI-frame median (with the range over runs) and
p95, each phase's median and the GPU completion median.

    python3 -I plans/rendering/vello-spike/summarize_present.py results/raw-present-*.md
"""
import re
import statistics
import sys

rows, order = {}, []
for path in sys.argv[1:]:
    for line in open(path, encoding="utf-8"):
        if not line.startswith("| ") or line.startswith("| Workload"):
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) != 7 or "/" not in cells[3]:
            continue
        key = (cells[0], cells[1], cells[2])
        med, p95 = (float(v) for v in cells[3].split("/"))
        phases = [(p.rsplit(":", 1)[0].strip(), float(p.rsplit(":", 1)[1].split("/")[0])) for p in cells[4].split("<br>")]
        gpu = cells[5]
        gpu = (float(gpu.split("/")[0]), float(gpu.split("/")[1])) if "/" in gpu else None
        if key not in rows:
            rows[key] = []
            order.append(key)
        rows[key].append((med, p95, phases, gpu, int(cells[6])))

print("| Workload | Scale | Renderer | UI frame median (range over runs) | p95 | Phases (median) | GPU completion median / p95 | Frames not presented (all runs) |")
print("|---|---|---|---|---|---|---|---|")
for key in sorted(order, key=lambda k: (float(k[1]), order.index(k))):
    runs = rows[key]
    meds = [r[0] for r in runs]
    names = [n for n, _ in runs[0][2]]
    phases = "<br>".join(f"{n}: {statistics.median(r[2][i][1] for r in runs):.3f}" for i, n in enumerate(names))
    gpus = [r[3] for r in runs if r[3]]
    gpu = f"{statistics.median(g[0] for g in gpus):.3f} / {statistics.median(g[1] for g in gpus):.3f}" if gpus else "not reachable through the public API"
    print(f"| {key[0]} | {key[1]} | {key[2]} | {statistics.median(meds):.3f} ({min(meds):.3f}–{max(meds):.3f}) | {statistics.median(r[1] for r in runs):.3f} | {phases} | {gpu} | {sum(r[4] for r in runs)} |")
print(f"\n{len(sys.argv) - 1} runs.")
