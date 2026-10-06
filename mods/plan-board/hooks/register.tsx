import type { Register } from 'claude-code'
import { update } from 'claude-code'

import type { Drill, PlanPick } from '../types'

// The plan board: the chosen plan's summary, its goals as a numbered list, and
// the drill-down under them. A goal opens its steps, a step opens its details
// and its testing notes. A `[browse plans]` button opens a selector over the
// project's plans, then the global ones. Progress is read from the planning
// skill's own files on every draw. The pick, the browse toggle and the drill
// are kept in $.state, so a redraw keeps them.
//
// The board is a pane in the terminal. Desktop has no pane, so the same summary
// is also the text the agent's `show_plan_board` tool returns, which the chat
// shows. Toggled by `enabled` in settings.json pluginConfigs["plan-board"].options.

const PANE = 'plan-board'
const TOOL = 'show_plan_board'
const READ = 'read_plan_board'
const SCROLL = 'scroll_plan_board'
const OPEN = 'open_plan_board'
const selection = { plugin: 'plan-board', key: 'selection' } as const
const browsing = { plugin: 'plan-board', key: 'browsing' } as const
const drill = { plugin: 'plan-board', key: 'drill' } as const
const NO_DRILL: Drill = { goal: null, step: null }

// A row of a progress table: its first cell as `name`, its last as `status`, and
// every cell kept so a step's name can be read from its Stepname column.
type Row = { name: string; status: string; cells: string[] }
type Table = { isGoals: boolean; headers: string[]; rows: Row[] }

// The part of `$.fs` the progress reading needs.
type Fs = {
  exists: (path: string) => Promise<boolean>
  list: (path?: string) => Promise<{ name: string; kind: string }[]>
  read: (path: string) => Promise<string>
  stat: (path: string) => Promise<{ mtimeMs: number }>
}

// Where the plans are: the session's own `.plans`, and the global folder.
type Roots = { project: string; global: string }

type Progress = {
  isGoals: boolean
  goalRows: Row[]
  stepsByGoal: Table[]
  doneGoals: number
  doneSteps: number
  totalSteps: number
  percent: number
}

type Entry = {
  dir: string
  name: string
  scope: 'project' | 'global'
  mtimeMs: number
  progress: Progress
}

// The planning skill writes a Markdown table per progress.md: a header row, a
// separator, then one row per goal (with a Goalname column) or per step. The
// status is the last column; a row counts as done when it says complete (or
// carries the check mark) and not "incomplete".
function tableOf(text: string): Table {
  const lines = text.split('\n').filter(line => line.startsWith('|'))
  const headers = (lines[0] ?? '').split('|').slice(1, -1).map(cell => cell.trim())
  const isGoals = headers.includes('Goalname')
  const rows = lines
    .slice(1)
    .filter(line => !/^\|\s*-/.test(line))
    .map(line => {
      const cells = line.split('|').slice(1, -1).map(cell => cell.trim())
      return { name: cells[0] ?? '', status: cells[cells.length - 1] ?? '', cells }
    })
  return { isGoals, headers, rows }
}

// A step's name: its Stepname cell where the table has one, else its first cell.
function stepName(table: Table, row: Row): string {
  const index = table.headers.indexOf('Stepname')
  return index >= 0 ? (row.cells[index] ?? row.name) : row.name
}

function isDone(status: string): boolean {
  const text = status.toLowerCase()
  if (text.includes('incomplete') || text.includes('not ')) return false
  return text.includes('✅') || text.includes('complete')
}

function percent(done: number, total: number): number {
  return total === 0 ? 0 : Math.round((done * 100) / total)
}

// A line of Markdown as the pane draws it: the style it takes and its text.
type MarkdownLine = { style: 'heading' | 'quote' | 'body' | 'blank'; text: string }

// Inline markers the pane cannot draw: bold, italic and code become their plain
// text, so `**word**` reads as `word`.
function plain(text: string): string {
  return text
    .replace(/\*\*(.+?)\*\*/g, '$1')
    .replace(/__(.+?)__/g, '$1')
    .replace(/`([^`]+)`/g, '$1')
    .replace(/(^|\s)\*(\S[^*]*)\*/g, '$1$2')
}

// The Markdown of a planning step, line by line. Headings are bold, the `§`
// lines the planning skill writes for each section are dimmed, bullets and
// checkboxes become `•`, `☐` and `☑`, and a rule becomes a dashed line.
function markdownLines(markdown: string): MarkdownLine[] {
  return markdown.split('\n').map((raw): MarkdownLine => {
    const line = raw.trimEnd()
    if (line.trim() === '') return { style: 'blank', text: '' }
    if (/^\s*-{3,}\s*$/.test(line)) return { style: 'quote', text: '─'.repeat(24) }
    const heading = /^#{1,6}\s+(.*)$/.exec(line)
    if (heading) return { style: 'heading', text: plain(heading[1] ?? '') }
    if (line.startsWith('§')) return { style: 'quote', text: plain(line) }
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

// One plan's progress: for a goal plan, the goals done and the steps across
// every goal, each goal's own steps read from its progress file.
async function measure(fs: Fs, planDir: string): Promise<Progress | null> {
  const file = `${planDir}/progress.md`
  if (!(await fs.exists(file))) return null
  const table = tableOf(await fs.read(file))

  if (table.isGoals) {
    const stepsByGoal: Table[] = []
    let doneSteps = 0
    let totalSteps = 0
    for (const goal of table.rows) {
      const goalFile = `${planDir}/${goal.name}/progress.md`
      const steps: Table = (await fs.exists(goalFile))
        ? tableOf(await fs.read(goalFile))
        : { isGoals: false, headers: [], rows: [] }
      stepsByGoal.push(steps)
      doneSteps += steps.rows.filter(row => isDone(row.status)).length
      totalSteps += steps.rows.length
    }
    return {
      isGoals: true,
      goalRows: table.rows,
      stepsByGoal,
      doneGoals: table.rows.filter(row => isDone(row.status)).length,
      doneSteps,
      totalSteps,
      percent: percent(doneSteps, totalSteps),
    }
  }

  const done = table.rows.filter(row => isDone(row.status)).length
  return {
    isGoals: false,
    goalRows: table.rows,
    stepsByGoal: [],
    doneGoals: done,
    doneSteps: done,
    totalSteps: table.rows.length,
    percent: percent(done, table.rows.length),
  }
}

// The plans under one folder, most recently changed first.
async function listPlans(fs: Fs, base: string, scope: 'project' | 'global'): Promise<Entry[]> {
  if (!(await fs.exists(base))) return []
  const entries: Entry[] = []
  for (const folder of await fs.list(base)) {
    if (folder.kind !== 'dir') continue
    const dir = `${base}/${folder.name}`
    const progress = await measure(fs, dir)
    if (!progress) continue
    const { mtimeMs } = await fs.stat(`${dir}/progress.md`)
    entries.push({ dir, name: folder.name, scope, mtimeMs, progress })
  }
  return entries.sort((a, b) => b.mtimeMs - a.mtimeMs)
}

function statusOf(progress: Progress): 'open' | 'completed' {
  return progress.totalSteps > 0 && progress.percent === 100 ? 'completed' : 'open'
}

// Incomplete plans first, then the project's own before the global ones, then
// the most recently changed.
function order(a: Entry, b: Entry): number {
  const openFirst = (entry: Entry) => (statusOf(entry.progress) === 'open' ? 0 : 1)
  const projectFirst = (entry: Entry) => (entry.scope === 'project' ? 0 : 1)
  return openFirst(a) - openFirst(b) || projectFirst(a) - projectFirst(b) || b.mtimeMs - a.mtimeMs
}

// Every plan, in the selector's order, and the one to show: the person's pick,
// else the `plan` option, else the plan changed most recently.
async function gather(
  fs: Fs,
  roots: Roots,
  plan: string,
  pick: PlanPick,
): Promise<{ all: Entry[]; chosen: Entry | undefined }> {
  const projectPlans = await listPlans(fs, `${roots.project}/.plans`, 'project')
  const globalPlans = await listPlans(fs, roots.global, 'global')
  const all = [...projectPlans, ...globalPlans].sort(order)
  const chosen =
    (pick && all.find(entry => entry.dir === pick.dir)) ||
    (plan && all.find(entry => entry.name === plan)) ||
    all.reduce<Entry | undefined>(
      (newest, entry) => (!newest || entry.mtimeMs > newest.mtimeMs ? entry : newest),
      undefined,
    )
  return { all, chosen }
}

// Where a step stands, from the status the planning skill records for it: done,
// in progress, or not started (the skill's `incomplete`).
function stageOf(status: string): 'done' | 'in progress' | 'not started' {
  if (isDone(status)) return 'done'
  if (/in[- ]progress/i.test(status)) return 'in progress'
  return 'not started'
}

// The colour a summary line is drawn in: the plan name in cyan, a finished
// status in green and an open one in yellow, the current goal or step in
// magenta, and the counts plain.
function summaryColor(line: string): string | undefined {
  if (line.startsWith('Plan:')) return 'cyan'
  if (line.startsWith('Status:')) return line.includes('completed') ? 'green' : 'yellow'
  if (line.startsWith('Current')) return 'magenta'
  return undefined
}

// The chosen plan's summary as lines of text: the pane shows them, and the tool
// returns them for the chat.
function summaryLines(chosen: Entry | undefined, roots: Roots): string[] {
  if (!chosen) {
    return [`No plans found in ${roots.project}/.plans or ${roots.global}.`]
  }
  const p = chosen.progress
  const lines = [
    `Plan: ${chosen.name} (${chosen.scope})`,
    `Status: ${statusOf(p)}, ${p.percent}% done`,
  ]

  if (p.isGoals) {
    const goalPct = percent(p.doneGoals, p.goalRows.length)
    lines.push(
      `Goals: ${p.doneGoals} of ${p.goalRows.length} done (${goalPct}%)`,
      `Steps: ${p.doneSteps} of ${p.totalSteps} done (${p.percent}%)`,
    )

    // The current goal is the first unfinished one; once every goal is done it
    // is the last, with its own step counts.
    const focus = p.goalRows.findIndex(row => !isDone(row.status))
    const index = focus === -1 ? p.goalRows.length - 1 : focus
    const goalName = p.goalRows[index]?.name ?? '-'
    const goalSteps = p.stepsByGoal[index]?.rows ?? []
    const goalDone = goalSteps.filter(row => isDone(row.status)).length
    const goalStepPct = percent(goalDone, goalSteps.length)
    lines.push(
      '',
      `Current goal: ${goalName}, ${goalStepPct}% done (${goalDone} of ${goalSteps.length} steps)`,
    )
  } else {
    // A plan kept as one step table: there is no goal level, so the current
    // step is the first unfinished one.
    const next = p.goalRows.find(row => !isDone(row.status))
    lines.push('', `Current step: ${next?.name ?? 'all done'}`)
  }
  return lines
}

// The text as the pane shows it, for matching: the Markdown and bullet marks the
// pane drops are removed, and whitespace is collapsed.
function comparable(text: string): string {
  return text.replace(/[`*•☐☑]/g, '').replace(/\s+/g, ' ').trim()
}

// The sections of a step that are the planning skill's own bookkeeping, not
// something a person reads: who owns the step, what file it changes, the
// atomicity checklist and the handoff to the next step.
const INTERNAL_SECTION = /^(ownership|change target|atomicity check|handoff|owned work units)$/i

// A step's Markdown with the bookkeeping sections and the `§` section numbers
// taken out. What is left is the objective, the instructions, the acceptance
// criteria, and anything else a person needs to read.
function humanMarkdown(markdown: string): string {
  const kept: string[] = []
  let hidden = false
  for (const raw of markdown.split('\n')) {
    const heading = /^#{1,6}\s+(.*)$/.exec(raw.trim())
    if (heading) hidden = INTERNAL_SECTION.test((heading[1] ?? '').trim())
    if (hidden || raw.trimStart().startsWith('§')) continue
    kept.push(raw)
  }
  return kept.join('\n')
}

// The heading a selected piece of text lies under, read from the step's own
// Markdown: the last heading above the first line that holds it. Null when the
// text is not in the file.
function sectionOf(markdown: string, text: string): string | null {
  const needle = comparable(text).slice(0, 60)
  if (needle === '') return null
  let current: string | null = null
  for (const raw of markdown.split('\n')) {
    const heading = /^#{1,6}\s+(.*)$/.exec(raw.trim())
    if (heading) current = (heading[1] ?? '').trim()
    if (comparable(raw).includes(needle)) return current ?? '(before the first heading)'
  }
  return null
}

// The drill as the chosen plan can show it: a goal or step that is not in the
// plan falls back to the plan's top level, so the pane is never left with no way
// back. Reads only; the stored drill is left as it is.
function resolveDrill(drillAt: Drill, chosen: Entry | undefined): Drill {
  const p = chosen?.progress
  if (!p || !p.isGoals || drillAt.goal === null) return NO_DRILL
  const index = p.goalRows.findIndex(row => row.name === drillAt.goal)
  if (index === -1) return NO_DRILL
  if (drillAt.step === null) return drillAt
  const steps = p.stepsByGoal[index]
  const found = steps?.rows.some(row => stepName(steps, row) === drillAt.step) ?? false
  return found ? drillAt : { goal: drillAt.goal, step: null }
}

// The redraw timer's cancel handle, kept so a re-registered session replaces the
// timer rather than adding a second one.
let tick: { cancel: () => void } | undefined

// The key a line of a drawn Markdown block is addressed by, so a scroll can land on it.
function blockKey(block: string, index: number): string {
  return `${block}-${index}`
}

export const register: Register = (on, options) => {
  if (options.enabled === false) return

  const plan = String(options.plan ?? '')
  const plansDir = String(options.plansDir ?? '')

  on('session.start', async ($, e, next) => {
    await $.command.register({
      name: 'plan-board',
      description: 'Show the progress of a plan in a pane; `/plan-board close` hides it',
    })
    await $.tool.register({
      name: TOOL,
      description:
        'Show the progress board of a plan (its goals and steps done, and the current goal) to the person. Opens the pane where there is one, and returns the summary as text for the chat.',
      inputSchema: { type: 'object', properties: {} },
    })
    await $.tool.register({
      name: READ,
      description:
        'Read which plan, goal and step the person has open in the plan board, and the file of that step. Use it when the person refers to "this step" or "this goal". Read-only.',
      inputSchema: { type: 'object', properties: {} },
    })
    await $.tool.register({
      name: OPEN,
      description:
        'Open a plan in the plan board at a goal, and optionally a step, so the person sees it: the plan by name (its folder name), the goal by its name, the step by its name. Only the plan is required.',
      inputSchema: {
        type: 'object',
        properties: {
          plan: { type: 'string', description: 'The plan folder name, e.g. windows-interactive-shell.' },
          goal: { type: 'string', description: 'The goal folder name, e.g. 01-backend-abstraction.' },
          step: { type: 'string', description: 'The step file name without .md, under the goal.' },
        },
        required: ['plan'],
      },
    })
    await $.tool.register({
      name: SCROLL,
      description:
        'Scroll the plan board to a paragraph of the plan, goal, step or testing notes it shows, by words in that paragraph. Lists nothing; says when no line matches.',
      inputSchema: {
        type: 'object',
        properties: { text: { type: 'string', description: 'Words in the paragraph to scroll to.' } },
        required: ['text'],
      },
    })
    // The pane is redrawn every few seconds, so progress made by the agent
    // shows up without the person asking again.
    tick?.cancel()
    tick = $.clock.every(5000, () => $.ui.invalidate('ui.render'))

    return next(e)
  })

  on('command.run', { command: 'plan-board' }, async ($, e) => {
    if (e.args.trim().toLowerCase() === 'close') {
      await $.ui.close({ id: PANE })
      return { text: 'Plan board closed.' }
    }
    await $.ui.open({ id: PANE, title: 'Plan progress' })

    return { text: 'Plan board opened.' }
  })

  // The agent's way to show the board. It opens the pane where the surface has
  // one, and returns the summary as text, so a surface without a pane (the
  // desktop app) shows it in the chat.
  on('tool.call', { tool: `mcp__plan-board__${TOOL}` }, async $ => {
    const home = (await $.env.get('HOME')) ?? ''
    const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
    const roots: Roots = {
      project: await $.session.root(),
      global: plansDir || `${xdg || `${home}/.config`}/tsch-ai-skills/plans`,
    }
    const fs: Fs = {
      exists: path => $.fs.exists(path),
      list: path => $.fs.list(path),
      read: path => $.fs.read(path),
      stat: path => $.fs.stat(path),
    }
    const { value } = await $.state.get(selection)
    const { chosen } = await gather(fs, roots, plan, value ?? null)
    await $.ui.open({ id: PANE, title: 'Plan progress' })

    return {
      result: ['Plan board shown to the person:', ...summaryLines(chosen, roots)].join('\n'),
    }
  })

  // The agent's way to open a plan in the board at a goal and step: the person's pick,
  // browse and drill are set as if they had chosen it, so the board draws it.
  on('tool.call', { tool: `mcp__plan-board__${OPEN}` }, async ($, e) => {
    const home = (await $.env.get('HOME')) ?? ''
    const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
    const roots: Roots = {
      project: await $.session.root(),
      global: plansDir || `${xdg || `${home}/.config`}/tsch-ai-skills/plans`,
    }
    const fs: Fs = {
      exists: path => $.fs.exists(path),
      list: path => $.fs.list(path),
      read: path => $.fs.read(path),
      stat: path => $.fs.stat(path),
    }
    const wanted = String(e.plan ?? '').trim()
    const { all } = await gather(fs, roots, plan, null)
    const found = all.find(entry => entry.name === wanted)
    if (!found) {
      return { result: `No plan named "${wanted}". Plans: ${all.map(entry => entry.name).join(', ')}` }
    }
    const goal = e.goal ? String(e.goal).trim() : null
    const step = e.step ? String(e.step).trim() : null
    if (goal !== null && !(await fs.exists(`${found.dir}/${goal}`))) {
      return { result: `Plan ${found.name} has no goal "${goal}".` }
    }
    if (goal !== null && step !== null && !(await fs.exists(`${found.dir}/${goal}/steps/${step}.md`))) {
      return { result: `Goal ${goal} has no step "${step}".` }
    }
    await update($, selection, () => ({ dir: found.dir, scope: found.scope }))
    await update($, browsing, () => false)
    await update($, drill, () => ({ goal, step }))
    await $.ui.open({ id: PANE, title: 'Plan progress', focus: true })
    return { result: `Opened ${found.name}${goal ? ` at goal ${goal}` : ''}${step ? `, step ${step}` : ''} in the plan board.` }
  })

  // The agent's way to move the board to a paragraph: the first drawn line holding the
  // words, searched in the text the board shows now (a step and its testing notes, a
  // goal, or the plan's own text).
  on('tool.call', { tool: `mcp__plan-board__${SCROLL}` }, async ($, e) => {
    const home = (await $.env.get('HOME')) ?? ''
    const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
    const roots: Roots = {
      project: await $.session.root(),
      global: plansDir || `${xdg || `${home}/.config`}/tsch-ai-skills/plans`,
    }
    const fs: Fs = {
      exists: path => $.fs.exists(path),
      list: path => $.fs.list(path),
      read: path => $.fs.read(path),
      stat: path => $.fs.stat(path),
    }
    const { value: pick } = await $.state.get(selection)
    const { value: picked } = await $.state.get(drill)
    const at: Drill = picked ?? NO_DRILL
    const { chosen } = await gather(fs, roots, plan, pick ?? null)
    if (!chosen) return { result: 'No plan is open in the board.' }

    const stepDir = at.goal !== null ? `${chosen.dir}/${at.goal}/steps` : ''
    const shown: { block: string; file: string }[] =
      at.step !== null
        ? [
            { block: 'step', file: `${stepDir}/${at.step}.md` },
            { block: 'testing', file: `${stepDir}/${at.step}-testing.md` },
          ]
        : at.goal !== null
          ? [{ block: 'goal', file: `${chosen.dir}/${at.goal}/goal.md` }]
          : [{ block: 'plan', file: `${chosen.dir}/plan-description.md` }]

    const wanted = String(e.text ?? '').trim().toLowerCase()
    for (const { block, file } of shown) {
      if (!(await fs.exists(file))) continue
      const drawn = markdownLines(humanMarkdown(await fs.read(file)))
      const index = drawn.findIndex(line => line.style !== 'blank' && line.text.toLowerCase().includes(wanted))
      if (index >= 0) {
        await $.ui.open({ id: PANE, title: 'Plan progress', focus: true })
        await $.ui.scroll({ to: { key: blockKey(block, index) }, in: PANE, block: 'start' })
        return { result: `Scrolled the ${block} to "${drawn[index]?.text ?? ''}".` }
      }
    }
    return { result: `Nothing matching "${wanted}" in what the plan board shows.` }
  })

  // The agent's read of what the person has open: the plan, whether the browser
  // is showing, and the goal and step the drill-down is at. Read-only.
  on('tool.call', { tool: `mcp__plan-board__${READ}` }, async $ => {
    const home = (await $.env.get('HOME')) ?? ''
    const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
    const roots: Roots = {
      project: await $.session.root(),
      global: plansDir || `${xdg || `${home}/.config`}/tsch-ai-skills/plans`,
    }
    const fs: Fs = {
      exists: path => $.fs.exists(path),
      list: path => $.fs.list(path),
      read: path => $.fs.read(path),
      stat: path => $.fs.stat(path),
    }
    const { value: pick } = await $.state.get(selection)
    const { value: isBrowsing } = await $.state.get(browsing)
    const { value: picked } = await $.state.get(drill)
    const at: Drill = picked ?? NO_DRILL
    const { chosen } = await gather(fs, roots, plan, pick ?? null)
    // The text the person selected: the engine's last selection. Its section is
    // found in the step's own file.
    const live = await $.ui.selection().catch(() => undefined)
    const text = live?.text ?? null
    const stepFile =
      chosen && at.goal !== null && at.step !== null
        ? `${chosen.dir}/${at.goal}/steps/${at.step}.md`
        : null
    const inSection =
      stepFile && text && (await fs.exists(stepFile))
        ? sectionOf(await fs.read(stepFile), text)
        : null

    const lines = [
      `Plan: ${chosen ? `${chosen.name} (${chosen.scope}, ${chosen.dir})` : 'none'}`,
      `Plan chosen: ${pick ? 'picked by the person' : 'by default (the plan option, or the most recently changed plan)'}`,
      `Browsing plans: ${isBrowsing ? 'yes' : 'no'}`,
      `Goal: ${at.goal ?? 'none'}`,
      `Step: ${at.step ?? 'none'}`,
      `Text highlighted: ${text ?? 'none'}`,
      `Selected in section: ${text ? (inSection ?? 'not found in this step file') : 'none'}`,
    ]
    if (chosen && at.goal !== null && at.step !== null) {
      lines.push(`Step file: ${chosen.dir}/${at.goal}/steps/${at.step}.md`)
    }
    return { result: lines.join('\n') }
  })

  // While the person has a step open in the board, the agent is told to ask
  // them to select the text they mean when a reference like "this step" or
  // "here" is unclear. The board's open or closed state is not kept, so an open
  // step is the signal.
  on('prompt.compose', async ($, e, next) => {
    const composed = await next(e)
    const { value: picked } = await $.state.get(drill)
    if (!picked || picked.step === null) return composed

    return {
      sections: [
        ...composed.sections,
        {
          id: 'plan-board-selection',
          scope: 'session',
          text:
            'The person has a plan step open in the plan board. When they say "this step", "this section", "here" or "this paragraph" and it is not clear what they mean, call read_plan_board to see the plan, the step, the section and any text they selected. If that is still unclear, ask them to select the text they mean in the plan board.',
        },
      ],
    }
  })

  on('ui.render', { component: 'Pane', requestId: PANE }, async ($, e) => {
    const { Box, Button, Text } = $.ui.resolve(e)
    // Markdown drawn as Text rows, each styled by its line.
    const rendered = (markdown: string, block: string) =>
      markdownLines(markdown).map((line, index) => {
        const key = blockKey(block, index)
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
    const roots: Roots = {
      project: await $.session.root(),
      global: plansDir || `${xdg || `${home}/.config`}/tsch-ai-skills/plans`,
    }
    const fs: Fs = {
      exists: path => $.fs.exists(path),
      list: path => $.fs.list(path),
      read: path => $.fs.read(path),
      stat: path => $.fs.stat(path),
    }
    const { value: pick } = await $.state.get(selection)
    const { value: isBrowsing } = await $.state.get(browsing)
    const { value: picked } = await $.state.get(drill)
    const { all, chosen } = await gather(fs, roots, plan, pick ?? null)
    // A drill naming a goal or step the plan no longer has falls back to the plan.
    const at: Drill = resolveDrill(picked ?? NO_DRILL, chosen)

    // The plan list, drawn in full while browsing: the engine scrolls it.
    const listed = isBrowsing
      ? all.map(entry => (
          <Button
            key={entry.dir}
            onPress={() => {
              update($, selection, () => ({ dir: entry.dir, scope: entry.scope }))
              update($, browsing, () => false)
              update($, drill, () => NO_DRILL)            }}
          >
            {`${entry.dir === chosen?.dir ? '> ' : '  '}${entry.scope === 'project' ? 'project' : 'global'}: ${entry.name}  ${entry.progress.percent}% ${statusOf(entry.progress)}`}
          </Button>
        ))
      : []

    // The drill-down under the summary: the goal the person opened, its steps,
    // and the step's details. A goal or step that no longer exists is ignored.
    const p = chosen?.progress
    const goalIndex =
      p && p.isGoals && at.goal !== null ? p.goalRows.findIndex(row => row.name === at.goal) : -1
    const goalSteps = p && goalIndex >= 0 ? p.stepsByGoal[goalIndex] : undefined
    // The goal's steps in order, so a step can step to its neighbours.
    const stepNames = goalSteps ? goalSteps.rows.map(row => stepName(goalSteps, row)) : []
    const stepIndex = at.step !== null ? stepNames.indexOf(at.step) : -1
    const previousStep = stepIndex > 0 ? stepNames[stepIndex - 1] : undefined
    const nextStep =
      stepIndex >= 0 && stepIndex < stepNames.length - 1 ? stepNames[stepIndex + 1] : undefined
    // The row of the step being viewed, for its own state under the title.
    const viewedRow =
      goalSteps && at.step !== null
        ? goalSteps.rows.find(row => stepName(goalSteps, row) === at.step)
        : undefined
    const viewedStage = viewedRow ? stageOf(viewedRow.status) : null
    // The goal's own text, from its goal.md, when a goal is open and no step is.
    const goalText =
      chosen && goalSteps && at.goal !== null && at.step === null &&
      (await fs.exists(`${chosen.dir}/${at.goal}/goal.md`))
        ? humanMarkdown(await fs.read(`${chosen.dir}/${at.goal}/goal.md`))
        : null
    // The plan's own text, from its plan-description.md, under the goal list.
    const planText =
      chosen && at.goal === null && (await fs.exists(`${chosen.dir}/plan-description.md`))
        ? humanMarkdown(await fs.read(`${chosen.dir}/plan-description.md`))
        : null
    const stepDir = chosen && at.goal !== null ? `${chosen.dir}/${at.goal}/steps` : ''
    const stepText =
      goalSteps && at.step !== null && (await fs.exists(`${stepDir}/${at.step}.md`))
        ? humanMarkdown(await fs.read(`${stepDir}/${at.step}.md`))
        : null
    const testingText =
      goalSteps && at.step !== null && (await fs.exists(`${stepDir}/${at.step}-testing.md`))
        ? humanMarkdown(await fs.read(`${stepDir}/${at.step}-testing.md`))
        : null

    const content = (
      <Box flexDirection="column">
        <Button variant="primary" onPress={() => update($, browsing, open => !(open ?? false))}>
          {isBrowsing ? 'hide plans' : 'browse plans'}
        </Button>
        {!isBrowsing && at.goal !== null && at.step !== null && (
          <Button variant="primary" onPress={() => update($, drill, () => ({ goal: at.goal, step: null }))}>
            back to goal
          </Button>
        )}
        <Text> </Text>
        <Box flexDirection="row" justifyContent="space-between">
          <Text bold inverse>
            Plan progress
          </Text>
          <Button role="dismiss" onPress={() => $.ui.close({ id: PANE })}>
            Close
          </Button>
        </Box>
        <Text> </Text>
        {!isBrowsing && viewedStage !== null && (
          <Text color={viewedStage === 'done' ? 'green' : viewedStage === 'in progress' ? 'yellow' : undefined}>
            {`This step: ${viewedStage}, testing notes ${testingText === null ? 'none' : 'written'}`}
          </Text>
        )}
        {!isBrowsing && viewedStage !== null && <Text> </Text>}
        {isBrowsing && <Text dimColor>{`Pick a plan, incomplete first (${all.length}):`}</Text>}
        {listed}
        {!isBrowsing &&
          summaryLines(chosen, roots).map((line, index) => (
            <Text key={index} color={summaryColor(line)}>
              {line}
            </Text>
          ))}

        {/* A plan without goals lists its steps and their state; a step has no
            drill-down, because its file's location is not part of the skill's layout. */}
        {!isBrowsing && p && !p.isGoals && p.goalRows.length > 0 && (
          <Box flexDirection="column">
            <Text> </Text>
            <Text bold underline color="cyan">
              Steps:
            </Text>
            {p.goalRows.map((row, index) => (
              <Text key={index} color={isDone(row.status) ? 'green' : undefined}>
                {`${index + 1}. ${row.name}  ${stageOf(row.status)}`}
              </Text>
            ))}
          </Box>
        )}

        {!isBrowsing && p && p.isGoals && at.goal === null && (
          <Box flexDirection="column">
            <Text> </Text>
            <Text bold underline color="cyan">
              Goals (open one to see its steps):
            </Text>
            {p.goalRows.map((goal, index) => {
              const steps = p.stepsByGoal[index]
              const done = steps?.rows.filter(row => isDone(row.status)).length ?? 0
              const total = steps?.rows.length ?? 0
              return (
                <Button
                  key={goal.name}
                  onPress={() => {
                    update($, drill, () => ({ goal: goal.name, step: null }))
                  }}
                >
                  {`${index + 1}. ${goal.name}  ${done} of ${total} steps, ${percent(done, total)}%`}
                </Button>
              )
            })}
            <Text> </Text>
            {planText !== null && rendered(planText, 'plan')}
          </Box>
        )}

        {!isBrowsing && goalSteps && at.goal !== null && at.step === null && (
          <Box flexDirection="column">
            <Text> </Text>
            <Button variant="primary" onPress={() => update($, drill, () => NO_DRILL)}>back to plan goals</Button>
            {goalSteps.rows.map((row, index) => {
              const name = stepName(goalSteps, row)
              return (
                <Button
                  key={name}
                  onPress={() => {
                    update($, drill, () => ({ goal: at.goal, step: name }))                  }}
                >
                  {`${index + 1}. ${name}  ${stageOf(row.status)}`}
                </Button>
              )
            })}
            <Text> </Text>
            {goalText === null ? <Text bold>{`Goal: ${at.goal}`}</Text> : rendered(goalText, 'goal')}
          </Box>
        )}

        {!isBrowsing && goalSteps && at.step !== null && (
          <Box flexDirection="column">
            <Text> </Text>
            <Box flexDirection="row" justifyContent="space-between">
              {previousStep !== undefined ? (
                <Button variant="primary" onPress={() => update($, drill, () => ({ goal: at.goal, step: previousStep }))}>
                  {'< Previous step'}
                </Button>
              ) : (
                <Text> </Text>
              )}
              <Button variant="primary" onPress={() => update($, drill, () => ({ goal: at.goal, step: null }))}>
                back to steps
              </Button>
              {nextStep !== undefined ? (
                <Button variant="primary" onPress={() => update($, drill, () => ({ goal: at.goal, step: nextStep }))}>
                  {'Next step >'}
                </Button>
              ) : (
                <Text> </Text>
              )}
            </Box>
            <Text> </Text>
            {stepText === null ? (
              <Text dimColor>This step has no file.</Text>
            ) : (
              rendered(stepText, 'step')
            )}
            <Text bold underline color="cyan">
              Testing
            </Text>
            {testingText === null ? (
              <Text dimColor>No testing file: the acceptance criteria above are the test.</Text>
            ) : (
              rendered(testingText, 'testing')
            )}
          </Box>
        )}
      </Box>
    )

    // No box of its own: the pane's height is the window, and the engine scrolls
    // the whole tree inside it, so the plan list, goals, steps and details all
    // scroll when they reach past the pane's bottom.
    return content
  })
}
