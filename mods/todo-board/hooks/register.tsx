import type { Register } from 'claude-code'

// The project's own open tasks (TODO.json), most urgent first. The register is
// read on every draw, so the pane always shows what is on disk. Open means not
// closed: a task closed as done, dropped or obsolete is left out. Split out of
// register-board, which showed bugs and tasks together in one pane; this one
// is tasks only, bugs-board is bugs only. Toggled by `enabled` in
// settings.json pluginConfigs["todo-board"].options.

const PANE = 'todo-board'
const TOOL = 'show_todo_board'
const READ = 'read_todo_board'

// The part of `$.fs` the reading needs.
type Fs = {
  exists: (path: string) => Promise<boolean>
  read: (path: string) => Promise<string>
}

// The part of `$.process.run` the resolution needs.
type ProcessRun = (argv: readonly string[]) => Promise<{ exitCode: number; stdout: string; stderr: string }>

type Task = { id: string; title: string; status: string; priority: string }

const PRIORITY_ORDER = ['urgent', 'high', 'normal', 'low', 'someday']
const TASK_CLOSED = ['done', 'dropped', 'obsolete']

function rank(order: string[], value: string | undefined): number {
  const index = order.indexOf(value ?? '')
  return index === -1 ? order.length : index
}

// The todo binary's own path, in the shared bin the installer puts it in --
// the same resolution register-board's own todoBin uses.
function todoBin(home: string, xdg: string): string {
  const bin = xdg ? `${xdg}/tsch-ai-skills/bin` : `${home}/.config/tsch-ai-skills/bin`
  return `${bin}/todo`
}

// The directory `todo` actually reads and writes TODO.json in, resolved once
// via `todo resolve-path`. `resolve-path` prints a bare filename with no
// directory component for today's unresolved/session-root case, so splitting
// off the last path segment naturally falls back to `root` there too; a
// process-run failure (the binary missing, for example) falls back to the
// session root the same way.
async function registersDir(run: ProcessRun, home: string, xdg: string, root: string): Promise<string> {
  const result = await run([todoBin(home, xdg), 'resolve-path']).catch(() => undefined)
  if (!result || result.exitCode !== 0) return root
  const file = result.stdout.trim()
  const slash = file.lastIndexOf('/')
  return slash === -1 ? root : file.slice(0, slash)
}

// The open tasks of the project, read from its own register.
async function openTasks(fs: Fs, run: ProcessRun, home: string, xdg: string, root: string): Promise<Task[]> {
  const dir = await registersDir(run, home, xdg, root)
  const tasks: Task[] = []
  const taskFile = `${dir}/TODO.json`
  if (await fs.exists(taskFile)) {
    const register = JSON.parse(await fs.read(taskFile)) as { tasks?: Task[] }
    for (const task of register.tasks ?? []) {
      if (!TASK_CLOSED.includes(task.status)) tasks.push(task)
    }
  }
  tasks.sort((a, b) => rank(PRIORITY_ORDER, a.priority) - rank(PRIORITY_ORDER, b.priority))
  return tasks
}

// Exposes the pure functions above for a unit test to call directly, without
// spawning the binary or driving the mod runtime -- nothing here is read by
// Claude Code itself, which only ever reads the `register` export below.
export const __test = {
  rank,
  todoBin,
  registersDir,
  openTasks,
}

export const register: Register = (on, options) => {
  if (options.enabled === false) return

  on('session.start', async ($, e, next) => {
    await $.command.register({
      name: 'todo-board',
      description: 'Show the project’s open tasks in a pane; `/todo-board close` hides it',
    })
    await $.tool.register({
      name: TOOL,
      description: 'Show the project’s open tasks to the person, in a pane, and return them as text for the chat.',
      inputSchema: { type: 'object', properties: {} },
    })
    await $.tool.register({
      name: READ,
      description: 'Read the project’s open tasks as text, without opening the pane.',
      inputSchema: { type: 'object', properties: {} },
    })
    return next(e)
  })

  on('command.run', { command: 'todo-board' }, async ($, e) => {
    if (e.args.trim().toLowerCase() === 'close') {
      await $.ui.close({ id: PANE })
      return { text: 'Todo board closed.' }
    }
    await $.ui.open({ id: PANE, title: 'Open tasks' })

    return { text: 'Todo board opened.' }
  })

  on('tool.call', { tool: `mcp__todo-board__${TOOL}` }, async $ => {
    const root = await $.session.root()
    const home = (await $.env.get('HOME')) ?? ''
    const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
    const fs: Fs = {
      exists: path => $.fs.exists(path),
      read: path => $.fs.read(path),
    }
    const tasks = await openTasks(fs, argv => $.process.run(argv), home, xdg, root)
    await $.ui.open({ id: PANE, title: 'Open tasks' })

    return {
      result: [
        `Open tasks: ${tasks.length}`,
        ...tasks.map(task => `${task.id} [${task.priority}] ${task.title}`),
      ].join('\n'),
    }
  })

  // The agent's read of the open tasks. Read-only; the pane is not opened.
  on('tool.call', { tool: `mcp__todo-board__${READ}` }, async $ => {
    const root = await $.session.root()
    const home = (await $.env.get('HOME')) ?? ''
    const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
    const fs: Fs = {
      exists: path => $.fs.exists(path),
      read: path => $.fs.read(path),
    }
    const tasks = await openTasks(fs, argv => $.process.run(argv), home, xdg, root)
    const live = await $.ui.selection().catch(() => undefined)

    return {
      result: [
        `Open tasks: ${tasks.length}`,
        ...tasks.map(task => `${task.id} [${task.priority}] ${task.title}`),
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
    const tasks = await openTasks(fs, argv => $.process.run(argv), home, xdg, root)

    return (
      <Box flexDirection="column">
        <Box flexDirection="row" justifyContent="space-between">
          <Text bold inverse>
            Open tasks
          </Text>
          <Button role="dismiss" onPress={() => $.ui.close({ id: PANE })}>
            Close
          </Button>
        </Box>
        <Text> </Text>
        {tasks.length === 0 && <Text dimColor>No open tasks.</Text>}
        {tasks.map(task => (
          <Text key={task.id} color={task.priority === 'urgent' || task.priority === 'high' ? 'yellow' : undefined}>
            {`${task.id}  ${task.priority}  ${task.title}`}
          </Text>
        ))}
      </Box>
    )
  })
}
