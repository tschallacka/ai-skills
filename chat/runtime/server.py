#!/usr/bin/env python3
# MODE: PROD
# server.py - chat runtime: line protocol on TCP, log storage under AI_CHAT_HOME.
# Chosen by chat-server.sh when python3 is present; not run by hand.
import os
import socket
import socketserver
import sys
import threading
import time

HOME = os.environ["AI_CHAT_HOME"]
CHAN_DIR = os.path.join(HOME, "channels")
LOCK_TRIES = 200


def chan_path(chan):
    return os.path.join(CHAN_DIR, chan + ".log")


def valid_chan(chan):
    return len(chan) > 1 and chan[0] == "#" and all(c.islower() or c.isdigit() or c in "_-" for c in chan[1:]) and len(chan) <= 33


def valid_nick(nick):
    return 1 <= len(nick) <= 32 and all(c.isalnum() or c in "_-" for c in nick)


class Lock:
    def __enter__(self):
        for _ in range(LOCK_TRIES):
            try:
                os.mkdir(self.path)
                return self
            except FileExistsError:
                time.sleep(0.05)
        raise RuntimeError("lock timeout " + self.path)

    def __exit__(self, *a):
        os.rmdir(self.path)

    def __init__(self, path):
        self.path = path


class Hub:
    """One lock around id allocation, append and the subscriber lists."""

    def __init__(self):
        self.mutex = threading.Lock()
        self.subs = {}  # conn -> set of chans

    def register(self, chan):
        if not valid_chan(chan):
            return None, "ERR invalid channel"
        with Lock(os.path.join(CHAN_DIR, chan + ".lock")):
            open(chan_path(chan), "a").close()
        return "OK register %s" % chan, None

    def append(self, chan, nick, text):
        with Lock(os.path.join(CHAN_DIR, chan + ".lock")):
            # B56/B57: only well-formed MSG lines count, and the next id is
            # highest+1 — a restored or hand-edited log may be out of order,
            # and an id below one already stored would be invisible to any
            # --since cursor past it.
            last = 0
            try:
                with open(chan_path(chan), "rb") as fh:
                    for line in fh:
                        if not line.startswith(b"MSG "):
                            continue
                        parts = line.split(b" ")
                        if len(parts) < 4:
                            continue
                        try:
                            mid = int(parts[2])
                        except ValueError:
                            continue
                        if mid > last:
                            last = mid
            except FileNotFoundError:
                pass
            mid = last + 1
            ts = int(time.time())
            line = "MSG %s %d %d %s :%s\n" % (chan, mid, ts, nick, text)
            with open(chan_path(chan), "a") as fh:
                fh.write(line)
        return line.rstrip("\n")

    def fetch(self, chan, since):
        out = []
        try:
            with open(chan_path(chan), encoding="utf-8") as fh:
                for line in fh:
                    parts = line.split(" ", 4)
                    if len(parts) > 3 and int(parts[2]) > since:
                        out.append(line.rstrip("\n"))
        except FileNotFoundError:
            pass
        return out

    def broadcast(self, line, chans, origin=None):
        with self.mutex:
            for conn in list(self.subs):
                if origin is not None and conn is origin:
                    continue
                if self.subs[conn] & chans:
                    try:
                        conn.sendall((line + "\n").encode())
                    except OSError:
                        pass


HUB = Hub()


class Handler(socketserver.StreamRequestHandler):
    def reply(self, text):
        self.wfile.write((text + "\n").encode())

    def handle(self):  # noqa: C901 - one verb per branch, deliberately flat
        HUB.mutex.acquire()
        HUB.subs[self.request] = set()
        HUB.mutex.release()
        nick = "anon-%d" % threading.get_ident()
        try:
            for raw in self.rfile:
                line = raw.decode(errors="replace").rstrip("\r\n")
                if not line:
                    continue
                verb = line.split(" ", 1)[0]
                arg = line[len(verb):].strip()
                if verb == "NICK":
                    if not valid_nick(arg):
                        self.reply("ERR invalid nick")
                        continue
                    nick = arg
                    self.reply("OK nick %s" % nick)
                elif verb == "REGISTER":
                    ok, err = HUB.register(arg)
                    self.reply(err or ok)
                elif verb == "JOIN":
                    # B66: JOIN may carry a since-id; the backlog from that id
                    # is replayed after subscribing, so a socket tail starts
                    # from the requested point instead of only new traffic.
                    # Subscribing first trades a possible duplicate (a line
                    # broadcast between snapshot and replay) for zero loss.
                    parts = arg.split(None, 1)
                    chan = parts[0]
                    since = "0"
                    if len(parts) > 1:
                        since = parts[1]
                    if not valid_chan(chan) or not since.isdigit():
                        self.reply("ERR invalid channel")
                        continue
                    HUB.mutex.acquire()
                    HUB.subs[self.request].add(chan)
                    HUB.mutex.release()
                    self.reply("OK join %s" % chan)
                    try:
                        with open(chan_path(chan), "rb") as fh:
                            for line in fh:
                                if not line.startswith(b"MSG "):
                                    continue
                                fields = line.split(b" ", 3)
                                if len(fields) < 4:
                                    continue
                                try:
                                    mid = int(fields[2])
                                except ValueError:
                                    continue
                                if mid >= int(since):
                                    self.reply(line.decode().rstrip("\n")) 
                    except FileNotFoundError:
                        pass
                elif verb == "LEAVE":
                    HUB.mutex.acquire()
                    HUB.subs[self.request].discard(arg)
                    HUB.mutex.release()
                    self.reply("OK leave %s" % arg)
                elif verb == "PRIVMSG":
                    chan, _, text = arg.partition(" :")
                    # B59: name the argument that was wrong, not a generic
                    # usage hint — perl and the bash handler already do.
                    if not valid_chan(chan):
                        self.reply("ERR invalid channel: %s" % chan)
                    elif not text:
                        self.reply("ERR usage: PRIVMSG #chan :text")
                        continue
                    # B74: the newline is the frame delimiter, so a handler
                    # can never see an injected command — but a bare CR would
                    # still reach the log and corrupt the stored line. Strip
                    # both, as the bash rung and chat-send.sh do.
                    stored = HUB.append(chan, nick, text.replace("\n", " ").replace("\r", " "))
                    self.reply(stored)
                    HUB.broadcast(stored, {chan}, origin=self.request)
                elif verb == "FETCH":
                    chan, _, since = arg.partition(" ")
                    if not valid_chan(chan) or not since.isdigit():
                        self.reply("ERR usage: FETCH #chan <since-id>")
                        continue
                    for row in HUB.fetch(chan, int(since)):
                        self.reply(row)
                    self.reply("OK fetch end")
                elif verb == "PING":
                    self.reply("PONG")
                elif verb == "QUIT":
                    self.reply("OK bye")
                    break
                else:
                    self.reply("ERR unknown verb %s" % verb)
        except (ConnectionResetError, BrokenPipeError):
            pass
        finally:
            HUB.mutex.acquire()
            HUB.subs.pop(self.request, None)
            HUB.mutex.release()


class Server(socketserver.ThreadingTCPServer):
    allow_reuse_address = True
    daemon_threads = True


if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 0
    os.makedirs(HOME, exist_ok=True)
    os.makedirs(CHAN_DIR, exist_ok=True)
    srv = Server((os.environ.get("AI_CHAT_BIND", "127.0.0.1"), port), Handler)
    with open(os.path.join(HOME, "server.port"), "w") as fh:
        fh.write("%d\n" % srv.server_address[1])
    srv.serve_forever()
