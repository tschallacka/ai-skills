import type { Register } from 'claude-code'
import { update } from 'claude-code'

// The brainstorms the brainstorm skill recorded: each is a `brainstorm.md` in a
// plan's folder, the project's own `.plans` first and the global plans after.
// The list is shown first; a brainstorm opens as formatted text, with a button
// back to the list. The chosen one is kept in $.state, so a redraw keeps it.
// Toggled by `enabled` in settings.json pluginConfigs["brainstorm-board"].options.

const PANE = 'brainstorm-board'
const TOOL = 'show_brainstorm_board'
const READ = 'read_brainstorm_board'
const SCROLL = 'scroll_brainstorm_board'
const pick = { plugin: 'brainstorm-board', key: 'pick' } as const

type Entry = { dir: string; name: string; scope: 'project' | 'global'; mtimeMs: number; firstLine: string }

// The part of `$.fs` the reading needs.
type Fs = {
  exists: (path: string) => Promise<boolean>
  list: (path?: string) => Promise<{ name: string; kind: string }[]>
  read: (path: string) => Promise<string>
  stat: (path: string) => Promise<{ mtimeMs: number }>
}

type Line = { style: 'heading' | 'quote' | 'body' | 'blank'; text: string }

// Inline markers the pane cannot draw are removed, so `**word**` reads as `word`.
function plain(text: string): string {
  return text
    .replace(/\*\*(.+?)\*\*/g, '$1')
    .replace(/__(.+?)__/g, '$1')
    .replace(/`([^`]+)`/g, '$1')
    .replace(/(^|\s)\*(\S[^*]*)\*/g, '$1$2')
}

// The Markdown of a brainstorm, line by line, with the planning skill's `§`
// section numbers taken out, which a reader does not need.
function markdownLines(markdown: string): Line[] {
  return markdown
    .split('\n')
    .filter(raw => !raw.trimStart().startsWith('§'))
    .map((raw): Line => {
      const line = raw.trimEnd()
      if (line.trim() === '') return { style: 'blank', text: '' }
      if (/^\s*-{3,}\s*$/.test(line)) return { style: 'quote', text: '─'.repeat(24) }
      const heading = /^#{1,6}\s+(.*)$/.exec(line)
      if (heading) return { style: 'heading', text: plain(heading[1] ?? '') }
      const task = /^(\s*)[-*]\s+\[( |x|X)\]\s+(.*)$/.exec(line)
      if (task) {
        const box = task[2] === ' ' ? '☐' : '☑'
        return { style: 'body', text: `${task[1] ?? ''}${box} ${plain(task[3] ?? '')}` }
      }
      const bullet = /^(\s*)[-*]\s+(.*)$/.exec(line)
      if (bullet) return { style: 'body', text: `${bullet[1] ?? ''}• ${plain(bullet[2] ?? '')}` }
      return { style: 'body', text: plain(line) }
    })
}

// The first heading of a brainstorm, which names it; its folder name otherwise.
function titleOf(markdown: string, fallback: string): string {
  const heading = markdown.split('\n').find(line => /^#{1,6}\s+/.test(line))
  return heading ? plain(heading.replace(/^#{1,6}\s+/, '').trim()) : fallback
}

// The brainstorms under one folder, most recently changed first.
async function listBrainstorms(fs: Fs, base: string, scope: 'project' | 'global'): Promise<Entry[]> {
  if (!(await fs.exists(base))) return []
  const entries: Entry[] = []
  for (const folder of await fs.list(base)) {
    if (folder.kind !== 'dir') continue
    const file = `${base}/${folder.name}/brainstorm.md`
    if (!(await fs.exists(file))) continue
    // A file that vanishes between the checks is skipped, not allowed to fail the list.
    const text = await fs.read(file).catch(() => null)
    const mtimeMs = await fs.stat(file).then(stat => stat.mtimeMs).catch(() => 0)
    if (text === null) continue
    entries.push({
      dir: `${base}/${folder.name}`,
      name: folder.name,
      scope,
      mtimeMs,
      firstLine: titleOf(text, folder.name),
    })
  }
  return entries.sort((a, b) => b.mtimeMs - a.mtimeMs)
}

// The redraw timer's cancel handle, kept so a re-registered session replaces the
// timer rather than adding a second one.
let tick: { cancel: () => void } | undefined

export const register: Register = (on, options) => {
  if (options.enabled === false) return

  const plansDir = String(options.plansDir ?? '')

  on('session.start', async ($, e, next) => {
    await $.command.register({
      name: 'brainstorm-board',
      description: 'Show the brainstorms in a pane; `/brainstorm-board close` hides it',
    })
    await $.tool.register({
      name: TOOL,
      description:
        'Show the brainstorms recorded for this project and the global plans, in a pane, and return the list as text for the chat.',
      inputSchema: { type: 'object', properties: {} },
    })
    await $.tool.register({
      name: READ,
      description:
        'Read which brainstorm the person has picked in the brainstorm board, and the brainstorms recorded, as text, without opening the pane.',
      inputSchema: { type: 'object', properties: {} },
    })
    await $.tool.register({
      name: SCROLL,
      description:
        'Scroll the brainstorm board to a paragraph of the brainstorm it shows, by words in that paragraph. Says when no line matches.',
      inputSchema: {
        type: 'object',
        properties: { text: { type: 'string', description: 'Words in the paragraph to scroll to.' } },
        required: ['text'],
      },
    })
    // A brainstorm recorded while the pane is open shows up on the next redraw.
    tick?.cancel()
    tick = $.clock.every(5000, () => $.ui.invalidate('ui.render'))
    return next(e)
  })

  on('command.run', { command: 'brainstorm-board' }, async ($, e) => {
    if (e.args.trim().toLowerCase() === 'close') {
      await $.ui.close({ id: PANE })
      return { text: 'Brainstorm board closed.' }
    }
    await $.ui.open({ id: PANE, title: 'Brainstorms' })

    return { text: 'Brainstorm board opened.' }
  })

  // The agent's way to show the board. It opens the pane and returns the list.
  on('tool.call', { tool: `mcp__brainstorm-board__${TOOL}` }, async $ => {
    const home = (await $.env.get('HOME')) ?? ''
    const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
    const project = await $.session.root()
    const global = plansDir || `${xdg || `${home}/.config`}/tsch-ai-skills/plans`
    const fs: Fs = {
      exists: path => $.fs.exists(path),
      list: path => $.fs.list(path),
      read: path => $.fs.read(path),
      stat: path => $.fs.stat(path),
    }
    const all = [
      ...(await listBrainstorms(fs, `${project}/.plans`, 'project')),
      ...(await listBrainstorms(fs, global, 'global')),
    ]
    await $.ui.open({ id: PANE, title: 'Brainstorms' })

    const listed = all.map(entry => `${entry.scope}: ${entry.firstLine} (${entry.name})`)
    return {
      result: [
        all.length === 0 ? 'No brainstorms recorded.' : 'Brainstorms recorded:',
        ...listed,
      ].join('\n'),
    }
  })

  // The agent's way to move the board to a paragraph of the picked brainstorm: the
  // first drawn line holding the words. Keys match the reading view's lines.
  on('tool.call', { tool: `mcp__brainstorm-board__${SCROLL}` }, async ($, e) => {
    const home = (await $.env.get('HOME')) ?? ''
    const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
    const project = await $.session.root()
    const global = plansDir || `${xdg || `${home}/.config`}/tsch-ai-skills/plans`
    const fs: Fs = {
      exists: path => $.fs.exists(path),
      list: path => $.fs.list(path),
      read: path => $.fs.read(path),
      stat: path => $.fs.stat(path),
    }
    const { value: chosenDir } = await $.state.get(pick)
    const all = [
      ...(await listBrainstorms(fs, `${project}/.plans`, 'project')),
      ...(await listBrainstorms(fs, global, 'global')),
    ]
    const chosen = all.find(entry => entry.dir === chosenDir)
    if (!chosen) return { result: 'No brainstorm is picked in the board.' }
    const text = await fs.read(`${chosen.dir}/brainstorm.md`).catch(() => null)
    if (text === null) return { result: 'The picked brainstorm could not be read.' }

    const wanted = String(e.text ?? '').trim().toLowerCase()
    const drawn = markdownLines(text)
    const index = drawn.findIndex(line => line.style !== 'blank' && line.text.toLowerCase().includes(wanted))
    if (index < 0) return { result: `Nothing matching "${wanted}" in the brainstorm.` }
    await $.ui.open({ id: PANE, title: 'Brainstorms', focus: true })
    await $.ui.scroll({ to: { key: `brainstorm-${index}` }, in: PANE, block: 'start' })
    return { result: `Scrolled to "${drawn[index]?.text ?? ''}".` }
  })

  // The agent's read of the board's pick: the brainstorm the person chose, its
  // file, and the list. Read-only; the pane is not opened.
  on('tool.call', { tool: `mcp__brainstorm-board__${READ}` }, async $ => {
    const home = (await $.env.get('HOME')) ?? ''
    const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
    const project = await $.session.root()
    const global = plansDir || `${xdg || `${home}/.config`}/tsch-ai-skills/plans`
    const fs: Fs = {
      exists: path => $.fs.exists(path),
      list: path => $.fs.list(path),
      read: path => $.fs.read(path),
      stat: path => $.fs.stat(path),
    }
    const { value: chosenDir } = await $.state.get(pick)
    const all = [
      ...(await listBrainstorms(fs, `${project}/.plans`, 'project')),
      ...(await listBrainstorms(fs, global, 'global')),
    ]
    const chosen = all.find(entry => entry.dir === chosenDir)
    const lines = [
      chosen ? 'Picked in the board: one brainstorm' : 'Picked in the board: none (the list is shown)',
      `Brainstorms recorded: ${all.length}`,
      ...all.map(entry => `${entry.scope}: ${entry.firstLine} (${entry.name})`),
    ]
    if (chosen) lines.push(`Picked: ${chosen.scope}: ${chosen.firstLine}`, `File: ${chosen.dir}/brainstorm.md`)
    const live = await $.ui.selection().catch(() => undefined)
    lines.push(`Highlighted in the board: ${live?.text ?? 'none'}`)
    return { result: lines.join('\n') }
  })

  on('ui.render', { component: 'Pane', requestId: PANE }, async ($, e) => {
    const { Box, Button, Text } = $.ui.resolve(e)
    const rendered = (markdown: string) =>
      markdownLines(markdown).map((line, index) => {
        const key = `brainstorm-${index}`
        if (line.style === 'blank') return <Box key={key}><Text> </Text></Box>
        if (line.style === 'heading')
          return (
            <Box key={key}>
              <Text bold underline color="cyan">
                {line.text}
              </Text>
            </Box>
          )
        if (line.style === 'quote') return <Box key={key}><Text dimColor>{line.text}</Text></Box>
        return <Box key={key}><Text>{line.text}</Text></Box>
      })

    const home = (await $.env.get('HOME')) ?? ''
    const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
    const project = await $.session.root()
    const global = plansDir || `${xdg || `${home}/.config`}/tsch-ai-skills/plans`
    const fs: Fs = {
      exists: path => $.fs.exists(path),
      list: path => $.fs.list(path),
      read: path => $.fs.read(path),
      stat: path => $.fs.stat(path),
    }
    const { value: chosenDir } = await $.state.get(pick)
    const all = [
      ...(await listBrainstorms(fs, `${project}/.plans`, 'project')),
      ...(await listBrainstorms(fs, global, 'global')),
    ]
    const chosen = all.find(entry => entry.dir === chosenDir)
    // A file removed between the listing and this read is treated as missing, so
    // the pane shows the list instead of drawing nothing.
    const text = chosen ? await fs.read(`${chosen.dir}/brainstorm.md`).catch(() => null) : null

    return (
      <Box flexDirection="column">
        <Box flexDirection="row" justifyContent="space-between">
          <Text bold inverse>
            Brainstorms
          </Text>
          <Button role="dismiss" onPress={() => $.ui.close({ id: PANE })}>
            Close
          </Button>
        </Box>
        <Text> </Text>
        {chosen && text !== null ? (
          <Box flexDirection="column">
            <Button variant="primary" onPress={() => update($, pick, () => null)}>
              back to brainstorms
            </Button>
            <Text dimColor>{`${chosen.scope} plan: ${chosen.name}`}</Text>
            <Text> </Text>
            {rendered(text)}
          </Box>
        ) : (
          <Box flexDirection="column">
            {all.length === 0 && <Text dimColor>No brainstorms recorded yet.</Text>}
            {all.map(entry => (
              <Button
                key={entry.dir}
                onPress={() => update($, pick, () => entry.dir)}
              >
                {`${entry.scope}: ${entry.firstLine}  (${entry.name})`}
              </Button>
            ))}
          </Box>
        )}
      </Box>
    )
  })
}
