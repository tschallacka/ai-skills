// MODE: PROD
// server.js - chat runtime chosen when node is present; same protocol and
// storage layout as server.py. Not run by hand: chat-server.sh starts it.
"use strict";
const net = require("net");
const fs = require("fs");
const path = require("path");

const HOME = process.env.AI_CHAT_HOME;
const CHAN_DIR = path.join(HOME, "channels");
const LOCK_TRIES = 200;
const subs = new Map(); // socket -> Set(chan)

const chanPath = (c) => path.join(CHAN_DIR, c + ".log");
const validChan = (c) =>
  /^#[a-z0-9_-]{1,32}$/.test(c);
const validNick = (n) => /^[A-Za-z0-9_-]{1,32}$/.test(n);

function withLock(chan, fn) {
  const lockPath = path.join(CHAN_DIR, chan + ".lock");
  for (let i = 0; i < LOCK_TRIES; i++) {
    try {
      fs.mkdirSync(lockPath);
      try {
        return fn();
      } finally {
        fs.rmdirSync(lockPath);
      }
    } catch (e) {
      if (e.code !== "EEXIST") throw e;
      Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 50);
    }
  }
  throw new Error("lock timeout " + lockPath);
}

function register(chan) {
  withLock(chan, () => fs.closeSync(fs.openSync(chanPath(chan), "a")));
  return "OK register " + chan;
}

function append(chan, nick, text) {
  return withLock(chan, () => {
    let last = 0;
    let raw = "";
    try {
      raw = fs.readFileSync(chanPath(chan), "utf8");
    } catch (e) {}
    for (const line of raw.split("\n")) {
      // B56: highest, not the final line's — restored logs may be unordered.
      if (line.startsWith("MSG ")) { const id = parseInt(line.split(" ")[2], 10); if (id > last) last = id; }
    }
    const line =
      "MSG " + chan + " " + (last + 1) + " " + Math.floor(Date.now() / 1000) +
      " " + nick + " :" + text;
    fs.appendFileSync(chanPath(chan), line + "\n");
    return line;
  });
}

function fetchLines(chan, since) {
  const out = [];
  let raw = "";
  try {
    raw = fs.readFileSync(chanPath(chan), "utf8");
  } catch (e) {
    return out;
  }
  for (const line of raw.split("\n")) {
    if (!line.startsWith("MSG ")) continue;
    const id = parseInt(line.split(" ")[2], 10);
    if (id > since) out.push(line);
  }
  return out;
}

function broadcast(line, chan, origin) {
  for (const [sock, chans] of subs) {
    if (sock === origin || !chans.has(chan)) continue;
    try {
      sock.write(line + "\n");
    } catch (e) {}
  }
}

const server = net.createServer((sock) => {
  sock.setEncoding("utf8");
  subs.set(sock, new Set());
  const nick = () => (sock.nick ? sock.nick : "anon-" + sock.remotePort);
  let buf = "";
  sock.on("data", (chunk) => {
    buf += chunk;
    let idx;
    while ((idx = buf.indexOf("\n")) >= 0) {
      const line = buf.slice(0, idx).replace(/\r$/, "");
      buf = buf.slice(idx + 1);
      if (line) handle(sock, nick, line);
    }
  });
  sock.on("error", () => {});
  sock.on("close", () => subs.delete(sock));
});

function handle(sock, nickOf, line) {
  const sp = line.indexOf(" ");
  const verb = sp < 0 ? line : line.slice(0, sp);
  const arg = sp < 0 ? "" : line.slice(sp + 1).trim();
  const say = (t) => sock.write(t + "\n");
  switch (verb) {
    case "NICK":
      if (!validNick(arg)) return say("ERR invalid nick");
      sock.nick = arg;
      say("OK nick " + arg);
      break;
    case "REGISTER":
      if (!validChan(arg)) return say("ERR invalid channel");
      say(register(arg));
      break;
    case "JOIN": {
      // B66: JOIN may carry a since-id; the backlog from that id is
      // replayed after subscribing. Subscribing first trades a possible
      // duplicate for zero loss.
      const parts = arg.split(/\s+/);
      const chan = parts[0];
      const since = (parts[1] && /^\d+$/.test(parts[1])) ? parseInt(parts[1], 10) : 0;
      if (!validChan(chan)) return say("ERR invalid channel");
      subs.get(sock).add(chan);
      say("OK join " + chan);
      let raw = "";
      try { raw = fs.readFileSync(chanPath(chan), "utf8"); } catch (e) {}
      for (const line of raw.split("\n")) {
        if (!line.startsWith("MSG ")) continue;
        const f = line.split(" ");
        const id = parseInt(f[2], 10);
        if (!isNaN(id) && id >= since && f[1] === chan) say(line);
      }
      break;
    }
    case "LEAVE":
      subs.get(sock).delete(arg);
      say("OK leave " + arg);
      break;
    case "PRIVMSG": {
      const sep = arg.indexOf(" :");
      if (sep < 0) return say("ERR usage: PRIVMSG #chan :text");
      const chan = arg.slice(0, sep);
      // B74: CR as well as LF — the newline is the frame delimiter so no
      // command can be injected, but a bare CR would corrupt a stored line.
      const text = arg.slice(sep + 2).replace(/[\n\r]/g, " ");
      if (!validChan(chan)) return say("ERR invalid channel: " + chan);
      if (!text) return say("ERR usage: PRIVMSG #chan :text");
      const stored = append(chan, nickOf(), text);
      say(stored);
      broadcast(stored, chan, sock);
      break;
    }
    case "FETCH": {
      const [chan, since] = arg.split(/\s+/);
      if (!validChan(chan) || !/^\d+$/.test(since || ""))
        return say("ERR usage: FETCH #chan <since-id>");
      for (const row of fetchLines(chan, parseInt(since, 10))) say(row);
      say("OK fetch end");
      break;
    }
    case "PING":
      say("PONG");
      break;
    case "QUIT":
      say("OK bye");
      sock.end();
      break;
    default:
      say("ERR unknown verb " + verb);
  }
}

const port = parseInt(process.argv[2] || "0", 10);
fs.mkdirSync(CHAN_DIR, { recursive: true });
const bind = process.env.AI_CHAT_BIND || "127.0.0.1";
server.listen(port, bind, () => {
  fs.writeFileSync(path.join(HOME, "server.port"), server.address().port + "\n");
});
