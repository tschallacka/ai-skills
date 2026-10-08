import type { Register } from 'claude-code'

// The project's own open bugs (BUGS.json), most severe first. The register is
// read on every draw, so the pane always shows what is on disk. Open means not
// closed: a bug closed as fixed, not-a-defect, wont-fix or obsolete is left
// out. Split out of register-board, which showed bugs and tasks together in
// one pane; this one is bugs only, todo-board is tasks only. Toggled by
// `enabled` in settings.json pluginConfigs["bugs-board"].options.

const PANE = 'bugs-board'
const TOOL = 'show_bugs_board'
const READ = 'read_bugs_board'

// The part of `$.fs` the reading needs.
type Fs = {
  exists: (path: string) => Promise<boolean>
  read: (path: string) => Promise<string>
}

// The part of `$.process.run` the resolution needs.
type ProcessRun = (argv: readonly string[]) => Promise<{ exitCode: number; stdout: string; stderr: string }>

type Bug = { id: string; title: string; status: string; severity?: string }

const SEVERITY_ORDER = ['blocking', 'major', 'minor', 'cosmetic']
const BUG_CLOSED = ['fixed', 'not-a-defect', 'wont-fix', 'obsolete']

function rank(order: string[], value: string | undefined): number {
  const index = order.indexOf(value ?? '')
  return index === -1 ? order.length : index
}

// The bugs binary's own path, in the shared bin the installer puts it in --
// the same resolution register-board's own bugsBin uses.
function bugsBin(home: string, xdg: string): string {
  const bin = xdg ? `${xdg}/tsch-ai-skills/bin` : `${home}/.config/tsch-ai-skills/bin`
  return `${bin}/bugs`
}

// The directory `bugs` actually reads and writes BUGS.json in, resolved once
// via `bugs resolve-path`. `resolve-path` prints a bare filename with no
// directory component for today's unresolved/session-root case, so splitting
// off the last path segment naturally falls back to `root` there too; a
// process-run failure (the binary missing, for example) falls back to the
// session root the same way.
async function registersDir(run: ProcessRun, home: string, xdg: string, root: string): Promise<string> {
  const result = await run([bugsBin(home, xdg), 'resolve-path']).catch(() => undefined)
  if (!result || result.exitCode !== 0) return root
  const file = result.stdout.trim()
  const slash = file.lastIndexOf('/')
  return slash === -1 ? root : file.slice(0, slash)
}

// The open bugs of the project, read from its own register.
async function openBugs(fs: Fs, run: ProcessRun, home: string, xdg: string, root: string): Promise<Bug[]> {
  const dir = await registersDir(run, home, xdg, root)
  const bugs: Bug[] = []
  const bugFile = `${dir}/BUGS.json`
  if (await fs.exists(bugFile)) {
    const register = JSON.parse(await fs.read(bugFile)) as { bugs?: Bug[] }
    for (const bug of register.bugs ?? []) {
      if (!BUG_CLOSED.includes(bug.status)) bugs.push(bug)
    }
  }
  bugs.sort((a, b) => rank(SEVERITY_ORDER, a.severity) - rank(SEVERITY_ORDER, b.severity))
  return bugs
}

// Exposes the pure functions above for a unit test to call directly, without
// spawning the binary or driving the mod runtime -- nothing here is read by
// Claude Code itself, which only ever reads the `register` export below.
export const __test = {
  rank,
  bugsBin,
  registersDir,
  openBugs,
}

export const register: Register = (on, options) => {
  if (options.enabled === false) return

  on('session.start', async ($, e, next) => {
    await $.command.register({
      name: 'bugs-board',
      description: 'Show the project’s open bugs in a pane; `/bugs-board close` hides it',
    })
    await $.tool.register({
      name: TOOL,
      description: 'Show the project’s open bugs to the person, in a pane, and return them as text for the chat.',
      inputSchema: { type: 'object', properties: {} },
    })
    await $.tool.register({
      name: READ,
      description: 'Read the project’s open bugs as text, without opening the pane.',
      inputSchema: { type: 'object', properties: {} },
    })
    return next(e)
  })

  on('command.run', { command: 'bugs-board' }, async ($, e) => {
    if (e.args.trim().toLowerCase() === 'close') {
      await $.ui.close({ id: PANE })
      return { text: 'Bugs board closed.' }
    }
    await $.ui.open({ id: PANE, title: 'Open bugs' })

    return { text: 'Bugs board opened.' }
  })

  on('tool.call', { tool: `mcp__bugs-board__${TOOL}` }, async $ => {
    const root = await $.session.root()
    const home = (await $.env.get('HOME')) ?? ''
    const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
    const fs: Fs = {
      exists: path => $.fs.exists(path),
      read: path => $.fs.read(path),
    }
    const bugs = await openBugs(fs, argv => $.process.run(argv), home, xdg, root)
    await $.ui.open({ id: PANE, title: 'Open bugs' })

    return {
      result: [
        `Open bugs: ${bugs.length}`,
        ...bugs.map(bug => `${bug.id} [${bug.severity ?? 'major'}] ${bug.title}`),
      ].join('\n'),
    }
  })

  // The agent's read of the open bugs. Read-only; the pane is not opened.
  on('tool.call', { tool: `mcp__bugs-board__${READ}` }, async $ => {
    const root = await $.session.root()
    const home = (await $.env.get('HOME')) ?? ''
    const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
    const fs: Fs = {
      exists: path => $.fs.exists(path),
      read: path => $.fs.read(path),
    }
    const bugs = await openBugs(fs, argv => $.process.run(argv), home, xdg, root)
    const live = await $.ui.selection().catch(() => undefined)

    return {
      result: [
        `Open bugs: ${bugs.length}`,
        ...bugs.map(bug => `${bug.id} [${bug.severity ?? 'major'}] ${bug.title}`),
        `Highlighted in the board: ${live?.text ?? 'none'}`,
      ].join('\n'),
    }
  })

  on('ui.render', { component: 'Pane', requestId: PANE }, async ($, e) => {
    const { Box, Button, Text } = $.ui.resolve(e)
    const root = await $.session.root()
    const home = (await $.env.get('HOME')) ?? ''
    const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
    const fs: Fs = {
      exists: path => $.fs.exists(path),
      read: path => $.fs.read(path),
    }
    const bugs = await openBugs(fs, argv => $.process.run(argv), home, xdg, root)

    return (
      <Box flexDirection="column">
        <Box flexDirection="row" justifyContent="space-between">
          <Text bold inverse>
            Open bugs
          </Text>
          <Button role="dismiss" onPress={() => $.ui.close({ id: PANE })}>
            Close
          </Button>
        </Box>
        <Text> </Text>
        {bugs.length === 0 && <Text dimColor>No open bugs.</Text>}
        {bugs.map(bug => (
          <Text key={bug.id} color={bug.severity === 'blocking' || bug.severity === 'major' ? 'yellow' : undefined}>
            {`${bug.id}  ${bug.severity ?? 'major'}  ${bug.title}`}
          </Text>
        ))}
      </Box>
    )
  })
}
