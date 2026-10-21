#!/usr/bin/python3 -I
"""The tester's `cargo`: runs the real cargo with its streams unchanged.

Every invocation appends a start event (time, argv, cwd) to
/log/builds.jsonl before cargo runs, copies stdout and stderr to
/log/builds/<id>.stdout and .stderr as they stream, and appends an end
event with the exit status. A build killed by the run's timeout keeps its
start event and partial output and has no end event.
"""

import json
import os
import subprocess
import sys
import threading
import time

REAL = "/opt/cargo-bin/cargo"
LOG = "/log/builds.jsonl"
build_id = f"{time.time_ns()}-{os.getpid()}"


def event(**fields):
    with open(LOG, "a") as f:
        f.write(json.dumps({"id": build_id, **fields}) + "\n")


forward_failed = []


def copy(src, dst, path, name):
    """Forwards a stream and saves it. If the reader of the forwarded stream
    goes away (`cargo metadata | head`), forwarding stops but the stream is
    still drained into the file, so cargo never blocks on a full pipe."""
    forwarding = True
    with open(path, "ab") as saved:
        for chunk in iter(lambda: src.read1(65536), b""):
            saved.write(chunk)
            saved.flush()
            if forwarding:
                try:
                    dst.write(chunk)
                    dst.flush()
                except (BrokenPipeError, OSError):
                    forwarding = False
                    forward_failed.append(name)


event(event="start", time=time.time(), argv=sys.argv[1:], cwd=os.getcwd())
proc = subprocess.Popen([REAL, *sys.argv[1:]], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
threads = [
    threading.Thread(target=copy, args=(proc.stdout, sys.stdout.buffer, f"/log/builds/{build_id}.stdout", "stdout")),
    threading.Thread(target=copy, args=(proc.stderr, sys.stderr.buffer, f"/log/builds/{build_id}.stderr", "stderr")),
]
for t in threads:
    t.start()
try:
    status = proc.wait()
except KeyboardInterrupt:
    proc.send_signal(2)
    status = proc.wait()
for t in threads:
    t.join()
event(event="end", time=time.time(), status=status, forward_failed=forward_failed)
for stream in (sys.stdout, sys.stderr):
    try:
        stream.close()
    except OSError:
        pass
sys.exit(status)
