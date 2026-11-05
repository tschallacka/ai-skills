import type { Register } from 'claude-code'
import { update } from 'claude-code'

// A read-only pane showing one tailpipe stream's recent lines, with a
// streams-picker button to switch which one is shown. Polls the client
// binary's `read` subcommand on a timer rather than holding its `tail`
// subcommand's own long-poll connection open -- the same choice chat-board
// makes against chat's own live tail, and for the same reason: a pane
// redraws on its own schedule regardless of which consumption model the
// CLI's tail subcommand uses. Unlike chat-board, there is no on-disk
// fallback: a tailpipe stream lives only in the server's memory (or, once
// evicted, as a gzip file this pane does not read), so every render shells
// out to the client against a live server.

const PANE = 'tailpipe-board'
const TOOL = 'show_tailpipe_board'
const READ = 'read_tailpipe_board'
const SHOWN = 20
const picked = { plugin: 'tailpipe-board', key: 'stream' } as const
const listing = { plugin: 'tailpipe-board', key: 'listing' } as const

// The part of `$.process.run` the resolution and reads need.
type ProcessRun = (argv: readonly string[]) => Promise<{ exitCode: number; stdout: string; stderr: string }>

type Line = { id: number; text: string }

// The tailpipe client binary's own path, in the shared bin the installer
// puts it in -- the same resolution chat-board's clientPath/todo-board's
// todoBin use for their own binaries.
function tailpipeClientPath(home: string, xdg: string): string {
  const bin = xdg ? `${xdg}/tsch-ai-skills/bin` : `${home}/.config/tsch-ai-skills/bin`
  return `${bin}/tailpipe-client-rs`
}

// The server endpoint tailpipe-server-rs binds by default (TAILPIPE_HOME,
// or XDG_CONFIG_HOME/tsch-ai-skills/tailpipe, or ~/.config/tsch-ai-skills/
// tailpipe), mirroring tailpipe-mcp's own resolve_endpoint default exactly.
function tailpipeEndpointOf(home: string, xdg: string): string {
  return `${xdg || `${home}/.config`}/tsch-ai-skills/tailpipe/tailpipe.sock`
}

// `list`'s stdout, one stream name per line.
function streamsOf(output: string): string[] {
  return output.split('\n').map(line => line.trim()).filter(line => line !== '')
}

// `read`'s stdout: one `<id>\t<text>` line per line, matching
// tailpipe-client-rs's own print_response format for Response::Lines.
function linesOf(output: string): Line[] {
  return output
    .split('\n')
    .map(line => /^(\d+)\t(.*)$/.exec(line))
    .filter((match): match is RegExpExecArray => match !== null)
    .map(match => ({ id: Number(match[1]), text: match[2] ?? '' }))
}

// Exposes the pure functions above for a unit test to call directly, without
// spawning the binary or driving the mod runtime -- nothing here is read by
// Claude Code itself, which only ever reads the `register` export below.
export const __test = {
  tailpipeClientPath,
  tailpipeEndpointOf,
  streamsOf,
  linesOf,
}

// The redraw timer's cancel handle, kept so a re-registered session replaces
// the timer rather than adding a second one.
let tick: { cancel: () => void } | undefined

export const register: Register = (on, options) => {
  if (options.enabled === false) return

  on('session.start', async ($, e, next) => {
    await $.command.register({
      name: 'tailpipe-board',
      description: 'Show a tailpipe stream’s recent lines in a pane; `/tailpipe-board close` hides it',
    })
    await $.tool.register({
      name: TOOL,
      description: 'Show a tailpipe stream’s recent lines to the person, in a pane, and return them as text.',
      inputSchema: { type: 'object', properties: {} },
    })
    await $.tool.register({
      name: READ,
      description: 'Read the tailpipe stream the board shows and its recent lines, as text, without opening the pane.',
      inputSchema: { type: 'object', properties: {} },
    })
    tick?.cancel()
    // Every five seconds, matching chat-board's own polling interval: often
    // enough to notice a new line, rarely enough that the pane does not
    // flicker.
    tick = $.clock.every(5000, () => $.ui.invalidate('ui.render'))
    return next(e)
  })

  on('command.run', { command: 'tailpipe-board' }, async ($, e) => {
    if (e.args.trim().toLowerCase() === 'close') {
      await $.ui.close({ id: PANE })
      return { text: 'Tailpipe board closed.' }
    }
    await $.ui.open({ id: PANE, title: 'Tailpipe' })
    return { text: 'Tailpipe board opened.' }
  })

  async function recentLines(
    run: ProcessRun,
    home: string,
    xdg: string,
    stream: string,
  ): Promise<Line[]> {
    const endpoint = tailpipeEndpointOf(home, xdg)
    const result = await run([
      tailpipeClientPath(home, xdg),
      'read',
      '--endpoint',
      endpoint,
      '--stream',
      stream,
      '--from',
      '1',
      '--to',
      String(Number.MAX_SAFE_INTEGER),
    ]).catch(() => ({ exitCode: 1, stdout: '', stderr: '' }))
    return linesOf(result.stdout).slice(-SHOWN)
  }

  async function activeStreams(run: ProcessRun, home: string, xdg: string): Promise<string[]> {
    const endpoint = tailpipeEndpointOf(home, xdg)
    const result = await run([tailpipeClientPath(home, xdg), 'list', '--endpoint', endpoint]).catch(() => ({
      exitCode: 1,
      stdout: '',
      stderr: '',
    }))
    return streamsOf(result.stdout)
  }

  on('tool.call', { tool: `mcp__tailpipe-board__${TOOL}` }, async $ => {
    const home = (await $.env.get('HOME')) ?? ''
    const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
    const run: ProcessRun = argv => $.process.run(argv)
    const streams = await activeStreams(run, home, xdg)
    const { value: picked_ } = await $.state.get(picked)
    const shown = picked_ && streams.includes(picked_) ? picked_ : streams[0]
    await $.ui.open({ id: PANE, title: shown ? `Tailpipe: ${shown}` : 'Tailpipe' })

    if (!shown) {
      return { result: 'No streams yet.' }
    }
    const lines = await recentLines(run, home, xdg, shown)
    return {
      result: [`Latest lines in ${shown}:`, ...lines.map(line => `${line.id}\t${line.text}`)].join('\n'),
    }
  })

  // The agent's read of the stream the board shows. Read-only; the pane is
  // not opened.
  on('tool.call', { tool: `mcp__tailpipe-board__${READ}` }, async $ => {
    const home = (await $.env.get('HOME')) ?? ''
    const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
    const run: ProcessRun = argv => $.process.run(argv)
    const streams = await activeStreams(run, home, xdg)
    const { value: picked_ } = await $.state.get(picked)
    const shown = picked_ && streams.includes(picked_) ? picked_ : streams[0]

    if (!shown) {
      return { result: 'No streams yet.' }
    }
    const lines = await recentLines(run, home, xdg, shown)
    return {
      result: [`Latest lines in ${shown}:`, ...lines.map(line => `${line.id}\t${line.text}`)].join('\n'),
    }
  })

  on('ui.render', { component: 'Pane', requestId: PANE }, async ($, e) => {
    const { Box, Button, Text } = $.ui.resolve(e)
    const home = (await $.env.get('HOME')) ?? ''
    const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
    const run: ProcessRun = argv => $.process.run(argv)
    const { value: choice } = await $.state.get(picked)
    const { value: browsing } = await $.state.get(listing)
    const streams = await activeStreams(run, home, xdg)
    const current = choice && streams.includes(choice) ? choice : streams[0]
    const lines = current ? await recentLines(run, home, xdg, current) : []

    return (
      <Box flexDirection="column" height="100%">
        <Box flexDirection="row" justifyContent="space-between">
          <Text bold inverse>
            {browsing ? 'Streams' : current ? `Tailpipe: ${current}` : 'Tailpipe'}
          </Text>
          <Box flexDirection="row">
            <Button
              variant={browsing ? 'primary' : undefined}
              onPress={() => update($, listing, () => !(browsing ?? false))}
            >
              {browsing ? 'back' : 'streams'}
            </Button>
            <Text> </Text>
            <Button role="dismiss" onPress={() => $.ui.close({ id: PANE })}>
              Close
            </Button>
          </Box>
        </Box>
        <Text> </Text>
        {browsing ? (
          <Box flexDirection="column">
            {streams.length === 0 && <Text dimColor>No streams yet.</Text>}
            {streams.map(name => (
              <Button
                key={name}
                variant={name === current ? 'primary' : undefined}
                onPress={() => {
                  update($, picked, () => name)
                  update($, listing, () => false)
                }}
              >
                {name}
              </Button>
            ))}
          </Box>
        ) : (
          <Box flexDirection="column" height="100%">
            {!current && <Text dimColor>No streams yet.</Text>}
            {current && lines.length === 0 && <Text dimColor>No lines yet.</Text>}
            {lines.map(line => (
              <Text key={line.id}>
                <Text dimColor>{`${line.id}\t`}</Text>
                {line.text}
              </Text>
            ))}
          </Box>
        )}
      </Box>
    )
  })
}
