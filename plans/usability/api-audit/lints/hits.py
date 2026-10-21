#!/usr/bin/env python3
"""Reads cargo's JSON messages on stdin and prints one line per warning:
lint, file:line, target kind and message, without duplicates."""
import json
import sys

seen = set()
for line in sys.stdin:
    try:
        m = json.loads(line)
    except ValueError:
        continue
    if m.get("reason") != "compiler-message":
        continue
    msg = m["message"]
    code = (msg.get("code") or {}).get("code")
    if not code or msg["level"] != "warning":
        continue
    span = next((s for s in msg["spans"] if s["is_primary"]), None)
    if span is None:
        continue
    kinds = ",".join(m["target"]["kind"])
    row = f"{code}\t{span['file_name']}:{span['line_start']}\t{kinds}\t{msg['message']}"
    if row not in seen:
        seen.add(row)
        print(row)
