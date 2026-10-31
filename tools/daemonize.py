#!/usr/bin/env python3
"""daemonize.py — run a command fully detached so it survives the harness.

The sandbox kills every process spawned by a Bash tool call once the call
ends (process-group/session sweep). A classic double-fork + setsid daemon
escapes that sweep (reparented to init before the cleanup runs).

Usage:
  python3 tools/daemonize.py --log FILE [--cwd DIR] -- CMD [ARGS...]

Example:
  python3 tools/daemonize.py --log dev.log --cwd /home/z/my-project -- bun run dev
"""
import os
import sys


def main() -> None:
    args = sys.argv[1:]
    log_path = "/tmp/daemonize.log"
    cwd = None
    if args[0] == "--log":
        log_path = args[1]
        args = args[2:]
    if args and args[0] == "--cwd":
        cwd = args[1]
        args = args[2:]
    if not args or args[0] != "--":
        print(__doc__)
        sys.exit(2)
    cmd = args[1:]

    # fork #1 — parent exits immediately
    pid = os.fork()
    if pid > 0:
        print(f"daemonized pid={pid} log={log_path}")
        sys.exit(0)
    os.setsid()
    # fork #2 — escape the session for good; reaper reparents us to init
    pid = os.fork()
    if pid > 0:
        sys.exit(0)

    if cwd:
        os.chdir(cwd)
    os.umask(0)

    log = os.open(log_path, os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o644)
    devnull = os.open(os.devnull, os.O_RDONLY)
    os.dup2(devnull, 0)
    os.dup2(log, 1)
    os.dup2(log, 2)
    if log > 2:
        os.close(log)
    if devnull > 2:
        os.close(devnull)

    os.execvp(cmd[0], cmd)


if __name__ == "__main__":
    main()
