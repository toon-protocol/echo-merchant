#!/usr/bin/env python3
"""Run a command in the provider VM through the QEMU guest agent.

The VM has no forwarded port, so there is no SSH into it. The guest agent speaks
over a virtio serial channel to a unix socket on the host, which is not a network.

  qga.py SOCKET ping
  qga.py SOCKET exec [--as USER] [--timeout SECONDS] -- COMMAND

`exec` runs COMMAND with bash -lc, prints its stdout and stderr, and exits with
its exit code.
"""
import base64, json, os, random, socket, sys, time


class Agent:
    def __init__(self, path, timeout=5):
        self.s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.s.settimeout(timeout)
        self.s.connect(path)
        self.buf = b""
        self.sync()

    def _send(self, msg):
        self.s.sendall(json.dumps(msg).encode() + b"\n")

    def _recv(self):
        while b"\n" not in self.buf:
            chunk = self.s.recv(65536)
            if not chunk:
                raise ConnectionError("guest agent closed the socket")
            self.buf += chunk
        line, self.buf = self.buf.split(b"\n", 1)
        return json.loads(line.lstrip(b"\xff"))

    def sync(self):
        n = random.randint(1, 2**31)
        self.s.sendall(b"\xff")
        self._send({"execute": "guest-sync-delimited", "arguments": {"id": n}})
        while True:
            r = self._recv()
            if r.get("return") == n:
                return

    def call(self, cmd, args=None):
        self._send({"execute": cmd, "arguments": args or {}})
        r = self._recv()
        if "error" in r:
            raise RuntimeError(r["error"].get("desc", r["error"]))
        return r["return"]


def main():
    sock, action = sys.argv[1], sys.argv[2]
    if action == "ping":
        try:
            Agent(sock, timeout=3).call("guest-ping")
        except Exception:
            sys.exit(1)
        return
    rest = sys.argv[3:]
    user, timeout = None, 3600
    while rest and rest[0] != "--":
        if rest[0] == "--as":
            user, rest = rest[1], rest[2:]
        elif rest[0] == "--timeout":
            timeout, rest = int(rest[1]), rest[2:]
        else:
            sys.exit(f"qga.py: unknown option {rest[0]}")
    command = " ".join(rest[1:])
    if user:
        argv = ["/usr/sbin/runuser", "-u", user, "--", "/usr/bin/env",
                "XDG_RUNTIME_DIR=/run/user/1000", f"HOME=/home/{user}",
                "/bin/bash", "-lc", command]
    else:
        argv = ["/bin/bash", "-lc", command]
    a = Agent(sock, timeout=30)
    pid = a.call("guest-exec", {"path": argv[0], "arg": argv[1:], "capture-output": True})["pid"]
    deadline = time.time() + timeout
    while True:
        st = a.call("guest-exec-status", {"pid": pid})
        if st.get("exited"):
            break
        if time.time() > deadline:
            sys.exit(f"qga.py: timed out after {timeout}s")
        time.sleep(0.5)
    sys.stdout.write(base64.b64decode(st.get("out-data", "")).decode(errors="replace"))
    sys.stderr.write(base64.b64decode(st.get("err-data", "")).decode(errors="replace"))
    sys.stdout.flush()
    sys.exit(st.get("exitcode", 1 if st.get("signal") else 0))


if __name__ == "__main__":
    main()
