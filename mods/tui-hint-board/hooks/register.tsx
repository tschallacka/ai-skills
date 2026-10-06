import type { Register } from 'claude-code'
import { update } from 'claude-code'

// The terminal-program profiles the interactive-shell skill ships and the agent
// has written (appprofiles/*.md): the list is shown first, and a profile opens
// as formatted text with a button back to the list. The chosen profile is kept
// in $.state. Toggled by `enabled` in settings.json pluginConfigs["tui-hint-board"].options.

const PANE = 'tui-hint-board'
const TOOL = 'show_tui_profiles'
const READ = 'read_tui_hint_board'
const OPEN = 'open_tui_profile'
const SECTION = 'scroll_tui_profile'
const profile = { plugin: 'tui-hint-board', key: 'profile' } as const
const filter = { plugin: 'tui-hint-board', key: 'filter' } as const

type Fs = {
  exists: (path: string) => Promise<boolean>
  list: (path?: string) => Promise<{ name: string; kind: string }[]>
  read: (path: string) => Promise<string>
}

// Inline markers the pane cannot draw are removed.
function plain(text: string): string {
  return text
    .replace(/\*\*(.+?)\*\*/g, '$1')
    .replace(/__(.+?)__/g, '$1')
    .replace(/`([^`]+)`/g, '$1')
}

// The profile's Markdown, line by line: headings, table rows left as text, and
// bullets as `•`.
function lines(markdown: string): { style: 'heading' | 'body' | 'blank' | 'divider'; text: string }[] {
  return markdown.split('\n').map(raw => {
    const line = raw.trimEnd()
    if (line.trim() === '') return { style: 'blank' as const, text: '' }
    // Longer than any pane; the pane cuts it at its own edge.
    if (/^\s*-{3,}\s*$/.test(line)) return { style: 'divider' as const, text: '─'.repeat(400) }
    const heading = /^#{1,6}\s+(.*)$/.exec(line)
    if (heading) return { style: 'heading' as const, text: plain(heading[1] ?? '') }
    const bullet = /^(\s*)[-*]\s+(.*)$/.exec(line)
    if (bullet) return { style: 'body' as const, text: `${bullet[1] ?? ''}• ${plain(bullet[2] ?? '')}` }
    return { style: 'body' as const, text: plain(line) }
  })
}

// Whether a profile name matches the filter the person typed: any case, any part of the name.
function matches(name: string, query: string): boolean {
  return name.toLowerCase().includes(query.trim().toLowerCase())
}

// A profile the board lists, with every folder it is written in: the agent's own
// notes (appprofiles.d) and the shipped one (appprofiles). One name, one entry.
type Profile = { name: string; dirs: string[] }

// The profile names the board lists: the shipped profiles and the agent's own.
async function listProfiles(fs: Fs, dirs: string[]): Promise<Profile[]> {
  const found: Profile[] = []
  for (const dir of dirs) {
    if (!(await fs.exists(dir))) continue
    for (const entry of await fs.list(dir)) {
      if (entry.kind === 'file' && entry.name.endsWith('.md') && entry.name !== 'FORMAT.md') {
        const name = entry.name.replace(/\.md$/, '')
        const known = found.find(item => item.name === name)
        if (known) known.dirs.push(dir)
        else found.push({ name, dirs: [dir] })
      }
    }
  }
  return found.sort((a, b) => a.name.localeCompare(b.name))
}

// A profile's text: the shipped text, with the agent's notes for this system appended
// under their own heading, so system-specific knowledge extends the seeded one.
async function bodyOf(fs: Fs, item: Profile): Promise<string | null> {
  let shippedText: string | null = null
  let ownText: string | null = null
  for (const dir of item.dirs) {
    const text = await fs.read(`${dir}/${item.name}.md`).catch(() => null)
    if (text === null) continue
    // The marker line tells the reader the file is a profile; the board does not show it.
    const body = text.replace(/^<!--[^\n]*-->\n?/, '').trim()
    if (dir.endsWith('appprofiles.d')) ownText = body
    else shippedText = body
  }
  if (shippedText !== null && ownText !== null) {
    return `${shippedText}\n\n---\n\n## Added for this system\n\n${ownText}`
  }
  return shippedText ?? ownText
}

export const register: Register = (on, options) => {
  if (options.enabled === false) return

  on('session.start', async ($, e, next) => {
    await $.command.register({
      name: 'tui-hint-board',
      description: 'Show the terminal-program profiles the agent has in a pane; `/tui-hint-board close` hides it',
    })
    await $.tool.register({
      name: TOOL,
      description: 'Show the terminal-program profiles the agent knows, in a pane, and return their names as text.',
      inputSchema: { type: 'object', properties: {} },
    })
    await $.tool.register({
      name: READ,
      description: 'Read which terminal program profile the person has picked in the board, as text, without opening the pane.',
      inputSchema: { type: 'object', properties: {} },
    })
    await $.tool.register({
      name: SECTION,
      description:
        'Scroll the open profile in the board to one of its titled sections, by part of the section title (e.g. "Keys", "Added for this system"). Lists the section titles when none matches.',
      inputSchema: {
        type: 'object',
        properties: { section: { type: 'string', description: 'Part of the section title to scroll to.' } },
        required: ['section'],
      },
    })
    await $.tool.register({
      name: OPEN,
      description:
        'Open one terminal program profile in the board for the person to read, by its name (its file name without .md). Shows the shipped text with any notes for this system under "Added for this system".',
      inputSchema: {
        type: 'object',
        properties: { name: { type: 'string', description: 'The profile name, e.g. less or lldb.' } },
        required: ['name'],
      },
    })
    return next(e)
  })

  on('command.run', { command: 'tui-hint-board' }, async ($, e) => {
    if (e.args.trim().toLowerCase() === 'close') {
      await $.ui.close({ id: PANE })
      return { text: 'TUI hint board closed.' }
    }
    await $.ui.open({ id: PANE, title: 'Terminal program profiles', focus: true })
    return { text: 'TUI hint board opened.' }
  })

  on('tool.call', { tool: `mcp__tui-hint-board__${TOOL}` }, async $ => {
    const home = (await $.env.get('HOME')) ?? ''
    const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
    const fs: Fs = {
      exists: path => $.fs.exists(path),
      list: path => $.fs.list(path),
      read: path => $.fs.read(path),
    }
    const shipped = `${xdg || `${home}/.config`}/tsch-ai-skills/appprofiles`
    const own = `${xdg || `${home}/.config`}/tsch-ai-skills/appprofiles.d`
    const names = await listProfiles(fs, [own, shipped])
    await $.ui.open({ id: PANE, title: 'Terminal program profiles', focus: true })

    return {
      result: [
        names.length === 0 ? 'No terminal program profiles yet.' : 'Terminal program profiles:',
        ...names.map(item => item.name),
      ].join('\n'),
    }
  })

  // The agent's way to move the open profile to one of its sections: the section is
  // the first heading whose title contains the words given. The reading view keys each
  // heading, so the scroll lands it at the top of the pane.
  on('tool.call', { tool: `mcp__tui-hint-board__${SECTION}` }, async ($, e) => {
    const home = (await $.env.get('HOME')) ?? ''
    const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
    const fs: Fs = {
      exists: path => $.fs.exists(path),
      list: path => $.fs.list(path),
      read: path => $.fs.read(path),
    }
    const shipped = `${xdg || `${home}/.config`}/tsch-ai-skills/appprofiles`
    const own = `${xdg || `${home}/.config`}/tsch-ai-skills/appprofiles.d`
    const everyone = await listProfiles(fs, [own, shipped])
    const { value: picked } = await $.state.get(profile)
    const chosen = everyone.find(item => item.name === picked)
    const text = chosen ? await bodyOf(fs, chosen) : null
    if (!chosen || text === null) {
      return { result: 'No profile is open in the board. Open one first with open_tui_profile.' }
    }
    const headings = lines(text)
      .map((line, index) => ({ line, index }))
      .filter(item => item.line.style === 'heading')
    const wanted = String(e.section ?? '').trim().toLowerCase()
    const found = headings.find(item => item.line.text.toLowerCase().includes(wanted))
    if (!found) {
      return { result: `No section matching "${wanted}". Sections: ${headings.map(item => item.line.text).join(', ')}` }
    }
    await $.ui.open({ id: PANE, title: 'Terminal program profiles', focus: true })
    await $.ui.scroll({ to: { key: `section-${found.index}` }, in: PANE, block: 'start' })
    return { result: `Scrolled ${chosen.name} to "${found.line.text}".` }
  })

  // The agent's way to open a profile for the person: the board picks it and opens
  // the pane, so the reading view shows it with the notes for this system.
  on('tool.call', { tool: `mcp__tui-hint-board__${OPEN}` }, async ($, e) => {
    const home = (await $.env.get('HOME')) ?? ''
    const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
    const fs: Fs = {
      exists: path => $.fs.exists(path),
      list: path => $.fs.list(path),
      read: path => $.fs.read(path),
    }
    const shipped = `${xdg || `${home}/.config`}/tsch-ai-skills/appprofiles`
    const own = `${xdg || `${home}/.config`}/tsch-ai-skills/appprofiles.d`
    const names = await listProfiles(fs, [own, shipped])
    const wanted = String(e.name ?? '').trim()
    const found = names.find(item => item.name === wanted)
    if (!found) {
      return { result: `No profile named "${wanted}". Profiles: ${names.map(item => item.name).join(', ')}` }
    }
    await update($, profile, () => found.name)
    await $.ui.open({ id: PANE, title: 'Terminal program profiles', focus: true })
    return { result: `Opened the ${found.name} profile in the board.` }
  })

  // The agent's read of the board's pick: the profile the person chose, or none.
  // Read-only; the pane is not opened.
  on('tool.call', { tool: `mcp__tui-hint-board__${READ}` }, async $ => {
    const home = (await $.env.get('HOME')) ?? ''
    const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
    const fs: Fs = {
      exists: path => $.fs.exists(path),
      list: path => $.fs.list(path),
      read: path => $.fs.read(path),
    }
    const shipped = `${xdg || `${home}/.config`}/tsch-ai-skills/appprofiles`
    const own = `${xdg || `${home}/.config`}/tsch-ai-skills/appprofiles.d`
    const names = await listProfiles(fs, [own, shipped])
    const { value: picked } = await $.state.get(profile)
    const chosen = names.find(item => item.name === picked)
    const lines = [
      chosen ? `Picked in the board: ${chosen.name}` : 'Picked in the board: none (the list is shown)',
      `Profiles available: ${names.length}`,
    ]
    if (chosen) lines.push(`Files: ${chosen.dirs.map(dir => `${dir}/${chosen.name}.md`).join(', ')}`)
    // The text the person has highlighted in the pane, when they have: a part of the profile.
    const live = await $.ui.selection().catch(() => undefined)
    lines.push(`Highlighted in the board: ${live?.text ?? 'none'}`)
    return { result: lines.join('\n') }
  })

  // The ring moving onto one of the last three profiles scrolls the list so the
  // profile two below it sits at the bottom: the focused one stays third from the
  // bottom, and the list moves up one with each step until the end of the list.
  on('ui.focus', { requestId: PANE }, async ($, e, next) => {
    if (e.component === 'Pane' && e.element !== undefined) {
      const home = (await $.env.get('HOME')) ?? ''
      const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
      const fs: Fs = {
        exists: path => $.fs.exists(path),
        list: path => $.fs.list(path),
        read: path => $.fs.read(path),
      }
      const shipped = `${xdg || `${home}/.config`}/tsch-ai-skills/appprofiles`
      const own = `${xdg || `${home}/.config`}/tsch-ai-skills/appprofiles.d`
      const { value: query } = await $.state.get(filter)
      const names = (await listProfiles(fs, [own, shipped])).filter(item => matches(item.name, query ?? ''))
      const index = names.findIndex(item => item.name === e.element)
      const last = names.length - 1
      if (index >= 0 && index >= last - 2) {
        const target = names[Math.min(index + 2, last)]
        if (target) await $.ui.scroll({ to: { key: target.name }, in: PANE, block: 'end' })
      }
    }
    return next(e)
  })

  on('ui.render', { component: 'Pane', requestId: PANE }, async ($, e) => {
    const { Box, Button, Input, Text } = $.ui.resolve(e)
    const home = (await $.env.get('HOME')) ?? ''
    const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
    const fs: Fs = {
      exists: path => $.fs.exists(path),
      list: path => $.fs.list(path),
      read: path => $.fs.read(path),
    }
    const shipped = `${xdg || `${home}/.config`}/tsch-ai-skills/appprofiles`
    const own = `${xdg || `${home}/.config`}/tsch-ai-skills/appprofiles.d`
    const everyone = await listProfiles(fs, [own, shipped])
    const { value: picked } = await $.state.get(profile)
    const { value: query } = await $.state.get(filter)
    const names = everyone.filter(item => matches(item.name, query ?? ''))
    const chosen = everyone.find(item => item.name === picked)
    const text = chosen ? await bodyOf(fs, chosen) : null

    return (
      <Box flexDirection="column">
        <Box flexDirection="row" justifyContent="space-between">
          <Text bold inverse>
            Terminal program profiles
          </Text>
        </Box>
        {!(chosen && text !== null) && (
          <Input key="profile-filter" label="filter: " placeholder="type a program name" onSubmit={text => update($, filter, () => text)} />
        )}
        {!(chosen && text !== null) && !!query?.trim() && (
          <Box flexDirection="row">
            <Text>Active filter: </Text>
            <Text inverse color="cyan">{` ${query.trim()} `}</Text>
            <Text> </Text>
            <Button variant="primary" onPress={() => update($, filter, () => '')}>clear filter</Button>
          </Box>
        )}
        <Text> </Text>
        {chosen && text !== null ? (
          <Box flexDirection="column">
            <Button variant="primary" autoFocus onPress={() => update($, profile, () => null)}>
              back to profiles
            </Button>
            <Text dimColor>
              {chosen.dirs.length > 1
                ? 'shipped profile, with your notes for this system'
                : chosen.dirs[0]?.endsWith('appprofiles.d')
                  ? 'your own profile'
                  : 'shipped profile'}
            </Text>
            <Text> </Text>
            {lines(text).map((line, index) =>
              line.style === 'blank' ? (
                <Text key={index}> </Text>
              ) : line.style === 'divider' ? (
                <Text key={index} dimColor wrap="truncate">
                  {line.text}
                </Text>
              ) : line.style === 'heading' ? (
                <Box key={`section-${index}`}>
                  <Text bold underline color="cyan">
                    {line.text}
                  </Text>
                </Box>
              ) : (
                <Text key={index}>{line.text}</Text>
              ),
            )}
          </Box>
        ) : (
          <Box flexDirection="column">
            {names.length === 0 && <Text dimColor>No terminal program profiles yet.</Text>}
            {names.map((item, index) => (
              <Button
                key={item.name}
                autoFocus={index === 0 || undefined}
                onPress={() => update($, profile, () => item.name)}
              >
                {item.name}
              </Button>
            ))}
          </Box>
        )}
        <Text> </Text>
        <Button role="dismiss" onPress={() => $.ui.close({ id: PANE })}>
          Close
        </Button>
      </Box>
    )
  })
}
