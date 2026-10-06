import type { Register } from 'claude-code'

// The project's open defects and open tasks, most urgent first. The registers
// are read on every draw, so the pane always shows what is on disk. Open means
// not closed: a bug closed as fixed, not-a-defect or obsolete is left out, a
// task closed as done, dropped or obsolete is left out. Toggled by `enabled` in
// settings.json pluginConfigs["register-board"].options.

const PANE = 'register-board'
const TOOL = 'show_register_board'
const READ = 'read_register_board'

// The part of `$.fs` the reading needs.
type Fs = {
  exists: (path: string) => Promise<boolean>
  read: (path: string) => Promise<string>
}

type Item = { id: string; title: string; status: string; priority: string; severity?: string }

const PRIORITY_ORDER = ['urgent', 'high', 'normal', 'low', 'someday']
const SEVERITY_ORDER = ['blocking', 'major', 'minor', 'cosmetic']
const BUG_CLOSED = ['fixed', 'not-a-defect', 'wont-fix', 'obsolete']
const TASK_CLOSED = ['done', 'dropped', 'obsolete']

function rank(order: string[], value: string | undefined): number {
  const index = order.indexOf(value ?? '')
  return index === -1 ? order.length : index
}

// The open bugs and tasks of the project, read from its own registers.
async function openItems(fs: Fs, root: string): Promise<{ bugs: Item[]; tasks: Item[] }> {
  const bugs: Item[] = []
  const tasks: Item[] = []
  const bugFile = `${root}/BUGS.json`
  const taskFile = `${root}/TODO.json`
  if (await fs.exists(bugFile)) {
    const register = JSON.parse(await fs.read(bugFile)) as { bugs?: Item[] }
    for (const bug of register.bugs ?? []) {
      if (!BUG_CLOSED.includes(bug.status)) bugs.push(bug)
    }
  }
  if (await fs.exists(taskFile)) {
    const register = JSON.parse(await fs.read(taskFile)) as { tasks?: Item[] }
    for (const task of register.tasks ?? []) {
      if (!TASK_CLOSED.includes(task.status)) tasks.push(task)
    }
  }
  bugs.sort((a, b) => rank(SEVERITY_ORDER, a.severity) - rank(SEVERITY_ORDER, b.severity))
  tasks.sort((a, b) => rank(PRIORITY_ORDER, a.priority) - rank(PRIORITY_ORDER, b.priority))
  return { bugs, tasks }
}

export const register: Register = (on, options) => {
  if (options.enabled === false) return

  on('session.start', async ($, e, next) => {
    await $.command.register({
      name: 'register-board',
      description: 'Show the open bugs and tasks of the project in a pane; `/register-board close` hides it',
    })
    await $.tool.register({
      name: TOOL,
      description:
        'Show the open bugs and open tasks of the project to the person, in a pane, and return them as text for the chat.',
      inputSchema: { type: 'object', properties: {} },
    })
    await $.tool.register({
      name: READ,
      description: 'Read the open bugs and open tasks of the project as text, without opening the pane.',
      inputSchema: { type: 'object', properties: {} },
    })
    return next(e)
  })

  on('command.run', { command: 'register-board' }, async ($, e) => {
    if (e.args.trim().toLowerCase() === 'close') {
      await $.ui.close({ id: PANE })
      return { text: 'Register board closed.' }
    }
    await $.ui.open({ id: PANE, title: 'Open bugs and tasks' })

    return { text: 'Register board opened.' }
  })

  on('tool.call', { tool: `mcp__register-board__${TOOL}` }, async $ => {
    const root = await $.session.root()
    const fs: Fs = {
      exists: path => $.fs.exists(path),
      read: path => $.fs.read(path),
    }
    const { bugs, tasks } = await openItems(fs, root)
    await $.ui.open({ id: PANE, title: 'Open bugs and tasks' })

    return {
      result: [
        `Open bugs: ${bugs.length}`,
        ...bugs.map(bug => `${bug.id} [${bug.severity ?? 'major'}] ${bug.title}`),
        `Open tasks: ${tasks.length}`,
        ...tasks.map(task => `${task.id} [${task.priority}] ${task.title}`),
      ].join('\n'),
    }
  })

  // The agent's read of the open items. Read-only; the pane is not opened.
  on('tool.call', { tool: `mcp__register-board__${READ}` }, async $ => {
    const root = await $.session.root()
    const fs: Fs = {
      exists: path => $.fs.exists(path),
      read: path => $.fs.read(path),
    }
    const { bugs, tasks } = await openItems(fs, root)
    const live = await $.ui.selection().catch(() => undefined)

    return {
      result: [
        `Open bugs: ${bugs.length}`,
        ...bugs.map(bug => `${bug.id} [${bug.severity ?? 'major'}] ${bug.title}`),
        `Open tasks: ${tasks.length}`,
        ...tasks.map(task => `${task.id} [${task.priority}] ${task.title}`),
        `Highlighted in the board: ${live?.text ?? 'none'}`,
      ].join('\n'),
    }
  })

  on('ui.render', { component: 'Pane', requestId: PANE }, async ($, e) => {
    const { Box, Button, Text } = $.ui.resolve(e)
    const root = await $.session.root()
    const fs: Fs = {
      exists: path => $.fs.exists(path),
      read: path => $.fs.read(path),
    }
    const { bugs, tasks } = await openItems(fs, root)

    return (
      <Box flexDirection="column">
        <Box flexDirection="row" justifyContent="space-between">
          <Text bold inverse>
            Open bugs and tasks
          </Text>
          <Button role="dismiss" onPress={() => $.ui.close({ id: PANE })}>
            Close
          </Button>
        </Box>
        <Text> </Text>
        <Text bold underline color="cyan">
          {`Bugs (${bugs.length} open)`}
        </Text>
        {bugs.length === 0 && <Text dimColor>No open bugs.</Text>}
        {bugs.map(bug => (
          <Text key={bug.id} color={bug.severity === 'blocking' || bug.severity === 'major' ? 'yellow' : undefined}>
            {`${bug.id}  ${bug.severity ?? 'major'}  ${bug.title}`}
          </Text>
        ))}
        <Text> </Text>
        <Text bold underline color="cyan">
          {`Tasks (${tasks.length} open)`}
        </Text>
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
