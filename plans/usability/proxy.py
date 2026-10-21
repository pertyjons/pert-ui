"""Host-side HTTPS CONNECT proxy for the blind-test sandbox.

Usage: proxy.py <unix-socket> <comma-separated allowed hosts> <log-file>

Listens on a Unix socket bound into the sandbox, accepts only
`CONNECT <allowed host>:443` and logs every request with its decision. The
sandbox has no other network path.
"""

import datetime
import os
import socket
import sys
import threading

SOCKET, ALLOWED, LOG = sys.argv[1], set(sys.argv[2].split(",")), sys.argv[3]
log_lock = threading.Lock()


def log(line):
    with log_lock, open(LOG, "a") as f:
        f.write(f"{datetime.datetime.now().isoformat(timespec='seconds')} {line}\n")


def pipe(src, dst):
    try:
        while data := src.recv(65536):
            dst.sendall(data)
    except OSError:
        pass
    finally:
        for s in (src, dst):
            try:
                s.shutdown(socket.SHUT_RDWR)
            except OSError:
                pass


def handle(client):
    try:
        head = b""
        while b"\r\n\r\n" not in head and len(head) < 65536:
            chunk = client.recv(4096)
            if not chunk:
                return
            head += chunk
        request = head.split(b"\r\n", 1)[0].decode("latin-1")
        parts = request.split()
        host, _, port = (parts[1].rpartition(":") if len(parts) == 3 else ("", "", ""))
        if parts[:1] != ["CONNECT"] or host not in ALLOWED or port != "443":
            log(f"DENY {request}")
            client.sendall(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n")
            return
        upstream = socket.create_connection((host, 443), timeout=30)
        upstream.settimeout(None)
        log(f"ALLOW {request}")
        client.sendall(b"HTTP/1.1 200 Connection established\r\n\r\n")
        rest = head.split(b"\r\n\r\n", 1)[1]
        if rest:
            upstream.sendall(rest)
        t = threading.Thread(target=pipe, args=(upstream, client), daemon=True)
        t.start()
        pipe(client, upstream)
        t.join()
    except OSError as e:
        log(f"ERROR {e}")
    finally:
        client.close()


def main():
    if os.path.exists(SOCKET):
        os.unlink(SOCKET)
    server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    server.bind(SOCKET)
    server.listen(64)
    while True:
        client, _ = server.accept()
        threading.Thread(target=handle, args=(client,), daemon=True).start()


main()
