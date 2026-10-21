"""In-sandbox bridge: forwards 127.0.0.1:3128 to the host proxy's Unix socket.

The sandbox's network namespace has only loopback, so this bridge and the
host proxy behind it are the only way out, and the proxy admits only the
model hosts.
"""

import socket
import threading


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
    upstream = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    try:
        upstream.connect("/run/proxy.sock")
    except OSError:
        client.close()
        return
    t = threading.Thread(target=pipe, args=(upstream, client), daemon=True)
    t.start()
    pipe(client, upstream)
    t.join()
    client.close()
    upstream.close()


server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
server.bind(("127.0.0.1", 3128))
server.listen(64)
while True:
    conn, _ = server.accept()
    threading.Thread(target=handle, args=(conn,), daemon=True).start()
