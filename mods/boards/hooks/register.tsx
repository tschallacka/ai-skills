import type { Register } from 'claude-code'

// A launcher pane: every board this repository ships, one row each, so a
// person can open one by clicking it rather than remembering its own slash
// command. A click calls the same `$.ui.open({ id, title })` a board's own
// command handler calls, so it opens exactly the way typing that command
// would -- this mod never draws a board's own content, only the list.
// Toggled by `enabled` in settings.json pluginConfigs["boards"].options.

const PANE = 'boards'
const TOOL = 'show_boards'
const READ = 'read_boards'

type BoardEntry = { id: string; title: string; label: string }

// Every board this repository ships, by its own pane id and the title it
// itself opens with. Listed regardless of which companion skill is actually
// installed in this Claude root -- this mod has no way to ask which sibling
// mods are present -- so clicking a board whose skill is not installed here
// opens an empty pane: nothing has registered a renderer for that id yet, but
// the click itself is the same safe, harmless action a board's own command
// performs.
const BOARD_LIST: readonly BoardEntry[] = [
  { id: 'bugs-board', title: 'Open bugs', label: 'Bugs' },
  { id: 'todo-board', title: 'Open tasks', label: 'Tasks' },
  { id: 'decision-board', title: 'Questions', label: 'Questions' },
  { id: 'register-board', title: 'Open bugs and tasks', label: 'Bugs & tasks (combined)' },
  { id: 'chat-board', title: 'Chat', label: 'Chat' },
  { id: 'ci-board', title: 'CI failures', label: 'CI' },
  { id: 'plan-board', title: 'Plan progress', label: 'Plan' },
  { id: 'brainstorm-board', title: 'Brainstorms', label: 'Brainstorms' },
  { id: 'tui-hint-board', title: 'Terminal program profiles', label: 'TUI hints' },
]

// Exposed for a unit test to check directly; nothing here is read by Claude
// Code itself, which only ever reads the `register` export below.
export const __test = { BOARD_LIST }

export const register: Register = (on, options) => {
  if (options.enabled === false) return

  on('session.start', async ($, e, next) => {
    await $.command.register({
      name: 'boards',
      description: 'Show every board the project ships as a clickable list in a pane; `/boards close` hides it',
    })
    await $.tool.register({
      name: TOOL,
      description:
        'Show every board the project ships to the person, in a pane, and return the list as text for the chat.',
      inputSchema: { type: 'object', properties: {} },
    })
    await $.tool.register({
      name: READ,
      description: 'Read the list of boards the launcher shows, as text, without opening the pane.',
      inputSchema: { type: 'object', properties: {} },
    })
    return next(e)
  })

  on('command.run', { command: 'boards' }, async ($, e) => {
    if (e.args.trim().toLowerCase() === 'close') {
      await $.ui.close({ id: PANE })
      return { text: 'Boards closed.' }
    }
    await $.ui.open({ id: PANE, title: 'Boards' })

    return { text: 'Boards opened.' }
  })

  on('tool.call', { tool: `mcp__boards__${TOOL}` }, async $ => {
    await $.ui.open({ id: PANE, title: 'Boards' })
    return { result: ['Boards:', ...BOARD_LIST.map(board => board.label)].join('\n') }
  })

  // Read-only; the pane is not opened.
  on('tool.call', { tool: `mcp__boards__${READ}` }, async () => {
    return { result: ['Boards:', ...BOARD_LIST.map(board => board.label)].join('\n') }
  })

  on('ui.render', { component: 'Pane', requestId: PANE }, async ($, e) => {
    const { Box, Button, Text } = $.ui.resolve(e)

    return (
      <Box flexDirection="column">
        <Box flexDirection="row" justifyContent="space-between">
          <Text bold inverse>
            Boards
          </Text>
          <Button role="dismiss" onPress={() => $.ui.close({ id: PANE })}>
            Close
          </Button>
        </Box>
        <Text> </Text>
        {BOARD_LIST.map(board => (
          <Button key={board.id} onPress={() => $.ui.open({ id: board.id, title: board.title })}>
            {board.label}
          </Button>
        ))}
      </Box>
    )
  })
}
