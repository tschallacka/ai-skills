import type { Register } from 'claude-code'
import { update } from 'claude-code'

// The latest messages of one chat channel, read from the local chat log with the
// chat client's own `read --local`, so no server has to be running. The pane
// redraws every two seconds. A channels button lists the channels on disk; a press
// on one shows it. Toggled by `enabled` and set by `channel` (the first one shown) and
// `nick` in settings.json pluginConfigs["chat-board"].options.

const PANE = 'chat-board'
// The input is its own pane, under the messages, so scrolling the messages never moves it.
const SAY = 'chat-board-say'
const TOOL = 'show_chat_board'
const READ = 'read_chat_board'
const SHOWN = 20
const picked = { plugin: 'chat-board', key: 'channel' } as const
const listing = { plugin: 'chat-board', key: 'listing' } as const
// How many messages beyond the last SHOWN the pane has been asked to load, ten at a press.
const extra = { plugin: 'chat-board', key: 'extra' } as const
const MORE = 10

type Message = { time: string; nick: string; text: string; at: number }

// How long before today a message was sent, in days, months or years: "1 day ago",
// "2 months ago". Nothing for a message from today, which shows its time alone.
function agoOf(at: number, nowSeconds: number): string | null {
  const day = 86400
  const days = Math.floor(nowSeconds / day) - Math.floor(at / day)
  if (!Number.isFinite(days) || days < 1) return null
  const plural = (count: number, unit: string) => `${count} ${unit}${count === 1 ? '' : 's'} ago`
  if (days < 30) return plural(days, 'day')
  if (days < 365) return plural(Math.floor(days / 30), 'month')
  return plural(Math.floor(days / 365), 'year')
}

// One `MSG #chan <id> <unix-time> <nick> :<text>` line of the chat client's output.
function messagesOf(output: string): Message[] {
  return output
    .split('\n')
    .map(line => /^MSG (\S+) (\d+) (\d+) (\S+) :(.*)$/.exec(line))
    .filter((match): match is RegExpExecArray => match !== null)
    .map(match => {
      const when = new Date(Number(match[3]) * 1000)
      const time = `${String(when.getUTCHours()).padStart(2, '0')}:${String(when.getUTCMinutes()).padStart(2, '0')}`
      return { time, nick: match[4] ?? '', text: match[5] ?? '', at: Number(match[3]) }
    })
}

// Each nick's colour: the same nick always gets the same one, picked from the palette
// by a hash of its name, so the colours look random but need no stored state.
// Thirty-two hex colours, evenly spread round the hue wheel at one lightness, so each is
// distinct and all are readable on a dark ground.
const NICK_COLORS = ["#eb7070","#eb8770","#eb9e70","#ebb570","#ebcc70","#ebe370","#dbeb70","#c4eb70","#adeb70","#96eb70","#80eb70","#70eb78","#70eb8f","#70eba6","#70ebbd","#70ebd4","#70ebeb","#70d4eb","#70bdeb","#70a6eb","#708feb","#7078eb","#8070eb","#9670eb","#ad70eb","#c470eb","#db70eb","#eb70e3","#eb70cc","#eb70b5","#eb709e","#eb7087"]
function colorOf(nick: string): string {
  let hash = 0
  for (const ch of nick) hash = (hash * 31 + ch.charCodeAt(0)) >>> 0
  return NICK_COLORS[hash % NICK_COLORS.length] ?? 'cyan'
}

// Every nick in view, posting or mentioned, in the order first seen.
function nicksIn(messages: Message[]): string[] {
  const seen = new Set<string>()
  for (const message of messages) {
    seen.add(message.nick)
    for (const piece of piecesOf(message.text)) if (piece.nick) seen.add(piece.nick)
  }
  return [...seen]
}

// A message's text as pieces: a mention (`@nick`) carries its nick, so it is drawn in
// that nick's colour; the rest is plain.
function piecesOf(text: string): { text: string; nick?: string }[] {
  const pieces: { text: string; nick?: string }[] = []
  let last = 0
  for (const match of text.matchAll(/@([A-Za-z0-9_.-]+)/g)) {
    const at = match.index ?? 0
    if (at > last) pieces.push({ text: text.slice(last, at) })
    pieces.push({ text: match[0], nick: match[1] })
    last = at + match[0].length
  }
  if (last < text.length) pieces.push({ text: text.slice(last) })
  return pieces
}

// The chat client's binary in the shared bin the installer puts it in.
function clientPath(home: string, xdg: string): string {
  const bin = xdg ? `${xdg}/tsch-ai-skills/bin` : `${home}/.config/tsch-ai-skills/bin`
  return `${bin}/chat-client-rs`
}

// The chat home: the channel logs and the server's port live here.
function chatHomeOf(home: string, xdg: string): string {
  return `${xdg || `${home}/.config`}/tsch-ai-skills/chat`
}

// The server the chat client's session names, as `host:port`, or null.
function serverOf(output: string): string | null {
  const line = output.split('\n').find(text => text.startsWith('server='))
  return line ? line.slice('server='.length).trim() : null
}

// The server this machine runs, from the port it wrote to the chat home. The plugin's own
// process has no session id, so `session show` would name the shared session's server
// instead; that is the fallback only.
async function addressOf(
  $: { fs: { exists: (path: string) => Promise<boolean>; read: (path: string) => Promise<string> }; process: { run: (argv: string[]) => Promise<{ exitCode: number; stdout: string; stderr: string }> } },
  home: string,
  xdg: string,
): Promise<string | null> {
  const portFile = `${chatHomeOf(home, xdg)}/server.port`
  if (await $.fs.exists(portFile)) {
    const port = (await $.fs.read(portFile)).trim()
    if (port !== '') return `127.0.0.1:${port}`
  }
  return serverOf((await $.process.run([clientPath(home, xdg), 'session', 'show'])).stdout)
}

// Whether something accepts a TCP connection at `address`. One second at most, so an
// offline server costs the pane a second; the client's own probe retries for far longer.
async function reachable(address: string, run: (argv: string[]) => Promise<{ exitCode: number }>): Promise<boolean> {
  const colon = address.lastIndexOf(':')
  const host = address.slice(0, colon)
  const port = address.slice(colon + 1)
  const result = await run(['timeout', '1', 'bash', '-c', 'exec 3<>"/dev/tcp/$1/$2"', 'probe', host, port])
  return result.exitCode === 0
}

// The channels with a log on disk, by name, sorted: the channel logs are `<chan>.log`.
async function channelsOf(
  $: { fs: { exists: (path: string) => Promise<boolean>; list: (path?: string) => Promise<{ name: string; kind: string }[]> } },
  dir: string,
): Promise<string[]> {
  if (!(await $.fs.exists(dir))) return []
  return (await $.fs.list(dir))
    .filter(entry => entry.kind === 'file' && entry.name.endsWith('.log'))
    .map(entry => entry.name.replace(/\.log$/, ''))
    .sort()
}

// The redraw timer's cancel handle, kept so a re-registered session replaces the
// timer rather than adding a second one.
let tick: { cancel: () => void } | undefined

export const register: Register = (on, options) => {
  if (options.enabled === false) return
  const channel = String(options.channel ?? '') || '#ops'
  // The nick the person's own lines go out under, from the `nick` option.
  const nick = String(options.nick ?? '') || 'tschallacka'

  on('session.start', async ($, e, next) => {
    await $.command.register({
      name: 'chat-board',
      description: 'Show the latest messages of the chat channel in a pane; `/chat-board close` hides it',
    })
    await $.tool.register({
      name: TOOL,
      description: 'Show the latest messages of the chat channel to the person, in a pane, and return them as text.',
      inputSchema: { type: 'object', properties: {} },
    })
    await $.tool.register({
      name: READ,
      description: 'Read the channel the chat board shows and its latest messages, as text, without opening the pane.',
      inputSchema: { type: 'object', properties: {} },
    })
    tick?.cancel()
    // Every five seconds: often enough for the status line to notice a server coming
    // online, rarely enough that the pane does not flicker.
    tick = $.clock.every(5000, () => $.ui.invalidate('ui.render'))
    return next(e)
  })

  on('command.run', { command: 'chat-board' }, async ($, e) => {
    if (e.args.trim().toLowerCase() === 'close') {
      await $.ui.close({ id: SAY })
      await $.ui.close({ id: PANE })
      return { text: 'Chat board closed.' }
    }
    await $.ui.open({ id: PANE, title: `Chat ${channel}` })
    await $.ui.open({ id: SAY, title: 'Say', rows: 3 })
    return { text: 'Chat board opened.' }
  })

  on('tool.call', { tool: `mcp__chat-board__${TOOL}` }, async ($) => {
    const home = (await $.env.get('HOME')) ?? ''
    const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
    const { value: current } = await $.state.get(picked)
    const shown = current ?? channel
    const run = await $.process.run([clientPath(home, xdg), 'read', '--local', '--chan', shown, '--since', '0'])
    const recent = messagesOf(run.stdout).slice(-SHOWN)
    await $.ui.open({ id: PANE, title: `Chat ${shown}` })
    await $.ui.open({ id: SAY, title: 'Say', rows: 3 })

    return {
      result: [
        `Latest messages in ${shown}:`,
        ...recent.map(message => `${message.time} ${message.nick}: ${message.text}`),
      ].join('\n'),
    }
  })

  // The agent's read of the channel the board shows. Read-only; the pane is not opened.
  on('tool.call', { tool: `mcp__chat-board__${READ}` }, async $ => {
    const home = (await $.env.get('HOME')) ?? ''
    const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
    const { value: current } = await $.state.get(picked)
    const shown = current ?? channel
    const run = await $.process.run([clientPath(home, xdg), 'read', '--local', '--chan', shown, '--since', '0'])
    const recent = messagesOf(run.stdout).slice(-SHOWN)
    // The text the person has highlighted in the pane, when they have.
    const live = await $.ui.selection().catch(() => undefined)
    const address = await addressOf($, home, xdg)
    const online = address ? await reachable(address, argv => $.process.run(argv)) : false

    return {
      result: [
        `Server modified IRC, ${address ?? 'no server set'} ${online ? 'online' : 'offline'}`,
        `Channel shown: ${shown}`,
        `Highlighted in the board: ${live?.text ?? 'none'}`,
        `Latest messages in ${shown}:`,
        ...recent.map(message => `${message.time} ${message.nick}: ${message.text}`),
      ].join('\n'),
    }
  })

  on('ui.render', { component: 'Pane', requestId: PANE }, async ($, e) => {
    const { Box, Button, Text } = $.ui.resolve(e)
    const home = (await $.env.get('HOME')) ?? ''
    const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
    const { value: choice } = await $.state.get(picked)
    const { value: browsing } = await $.state.get(listing)
    const { value: loaded } = await $.state.get(extra)
    const current = choice ?? channel
    const run = await $.process.run([clientPath(home, xdg), 'read', '--local', '--chan', current, '--since', '0'])
    const all = messagesOf(run.stdout)
    const shownCount = SHOWN + (loaded ?? 0)
    const recent = all.slice(-shownCount)
    const earlier = all.length > recent.length
    // A nick's colour is its place in the channel's log, the order it first speaks or is
    // mentioned: the log only grows at the end, so no nick ever moves to another colour.
    const order = nicksIn(all)
    const colorOfNick = (nick: string) => {
      const place = order.indexOf(nick)
      // Stepping eleven places round the palette (coprime with its 32) puts nicks that first
      // appear one after another far apart on the hue wheel, rather than side by side.
      return place >= 0 ? NICK_COLORS[(place * 11) % NICK_COLORS.length] ?? 'cyan' : colorOf(nick)
    }
    // The clock comes from the system, as the CI board's does: the plugin runtime's own
    // Date.now is not trusted for this.
    const nowSeconds = Number((await $.process.run(['date', '+%s'])).stdout.trim())
    const address = await addressOf($, home, xdg)
    const online = address ? await reachable(address, argv => $.process.run(argv)) : false
    const channels = await channelsOf($, `${chatHomeOf(home, xdg)}/channels`)

    return (
      <Box flexDirection="column" height="100%">
        <Box flexDirection="row" justifyContent="space-between">
          <Text bold inverse>
            {browsing ? 'Channels' : `Chat ${current}`}
          </Text>
          <Box flexDirection="row">
            <Button variant={browsing ? 'primary' : undefined} onPress={() => update($, listing, () => !(browsing ?? false))}>
              {browsing ? 'back to chat' : 'channels'}
            </Button>
            <Text> </Text>
            <Button
              role="dismiss"
              onPress={async () => {
                await $.ui.close({ id: SAY })
                await $.ui.close({ id: PANE })
              }}
            >
              Close
            </Button>
          </Box>
        </Box>
        <Text> </Text>
        {browsing ? (
          <Box flexDirection="column">
            {channels.length === 0 && <Text dimColor>No channels on this machine yet.</Text>}
            {channels.map(name => (
              <Button
                key={name}
                variant={name === current ? 'primary' : undefined}
                onPress={() => {
                  update($, picked, () => name)
                  update($, listing, () => false)
                  update($, extra, () => 0)
                }}
              >
                {name}
              </Button>
            ))}
          </Box>
        ) : (
          <Box flexDirection="column" height="100%">
            {/* The messages take the room the input leaves, and clip to it, so the input stays
                in view however long the log is. */}
            <Box flexDirection="column" flexGrow={1} overflow="hidden">
            <Text>
              {`Server modified IRC, ${address ?? 'no server set'} `}
              <Text color={online ? 'green' : 'red'}>{online ? '[online]' : '[offline]'}</Text>
            </Text>
            <Text> </Text>
            <Button
              variant={earlier ? 'primary' : undefined}
              onPress={() => {
                if (earlier) update($, extra, () => (loaded ?? 0) + MORE)
              }}
            >
              {earlier ? `load ${MORE} earlier messages` : 'no earlier messages'}
            </Button>
            {recent.length === 0 && <Text dimColor>No messages in this channel yet.</Text>}
            {recent.map((message, index) => (
              <Text key={index}>
                <Text dimColor>{`${message.time} `}</Text>
                {Number.isFinite(nowSeconds) && agoOf(message.at, nowSeconds) && (
                  <Text dimColor italic>{`${agoOf(message.at, nowSeconds)}  `}</Text>
                )}
                <Text bold color={colorOfNick(message.nick)}>{`<${message.nick}> `}</Text>
                {piecesOf(message.text).map((piece, part) => (
                  <Text key={part} bold={piece.nick !== undefined} color={piece.nick ? colorOfNick(piece.nick) : undefined}>
                    {piece.text}
                  </Text>
                ))}
              </Text>
            ))}
            </Box>
          </Box>
        )}
      </Box>
    )
  })

  // The input pane: the line the person types goes to the channel under their own nick.
  on('ui.render', { component: 'Pane', requestId: SAY }, async ($, e) => {
    const { Box, Input } = $.ui.resolve(e)
    const home = (await $.env.get('HOME')) ?? ''
    const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
    const { value: choice } = await $.state.get(picked)
    const current = choice ?? channel
    const address = await addressOf($, home, xdg)
    const online = address ? await reachable(address, argv => $.process.run(argv)) : false

    const say = (text: string) => {
      const line = text.trim()
      if (line === '') return
      $.process
        .run([clientPath(home, xdg), 'send', '--chan', current, '--nick', nick, '--text', line])
        .then(() => $.ui.invalidate('ui.render'))
        .catch(() => $.ui.invalidate('ui.render'))
    }

    return (
      <Box borderStyle="round" borderColor={online ? 'green' : 'gray'} paddingX={1}>
        <Input
          key="chat-say"
          label={`${nick} > `}
          placeholder={`message ${current}`}
          submitLabel="send"
          onSubmit={say}
        />
      </Box>
    )
  })
}
