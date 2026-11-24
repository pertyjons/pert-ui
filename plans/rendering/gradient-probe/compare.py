"""Compare production solid-scene queue writes, keyed by scenario and frame."""
import ast
import collections
import sys
from pathlib import Path


def uploads(path):
    frames = {}
    for line in Path(path).read_text().splitlines():
        if line.startswith("upload\t"):
            _, scene, frame, data = line.split("\t")
            key = scene, int(frame)
            if key in frames:
                raise ValueError(f"duplicate frame {key}")
            writes = ast.literal_eval(data)
            if len(writes) == 4:
                writes.append((0, 0))
            frames[key] = writes
    if not frames:
        raise ValueError(f"no writes in {path}")
    return frames


before, after = map(uploads, sys.argv[1:])
if before.keys() != after.keys():
    sys.exit("frame identities differ")
for scene in sorted({key[0] for key in before}):
    keys = [key for key in before if key[0] == scene]
    totals = [sum(before[key][i][0] for key in keys) for i in range(5)]
    differences = collections.Counter(i for key in keys for i in range(5)
                                      if before[key][i] != after[key][i])
    print(scene, len(keys), totals, dict(differences), sep="\t")
sys.exit(before != after)
