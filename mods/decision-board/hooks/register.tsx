import type { Register } from 'claude-code'
import { update } from 'claude-code'

// The project's questions (DECISIONS.json), most urgent first. A question's
// lifecycle is open -> decided -> implemented: the user answering one does
// not make it vanish from this pane, since a decided question is still
// outstanding work -- the agent's, not the user's -- until it is actually
// carried out and marked implemented. The pane defaults to a "Pending" view
// (open + decided) and can be toggled to "Implemented" to see what is
// already done. The register is read on every draw, the same way
// register-board reads BUGS.json/TODO.json, so the pane always shows what is
// on disk; a timer also forces a redraw, the same pattern ci-board's
// JOB_REFRESH_MS uses, so a question answered or implemented elsewhere (the
// CLI, or an agent via MCP) updates without the person reopening the pane.
// The interval is short (REFRESH_MS below): unlike ci-board's network-bound
// GitHub calls, this is a single cheap local file read, so there is no
// reason to make a person wait up to 30 seconds to see a question raised a
// moment ago.
// Toggled by `enabled` in settings.json pluginConfigs["decision-board"].options.

const PANE = 'decision-board'
const TOOL = 'show_decision_board'
const READ = 'read_decision_board'
const ANSWER = 'answer_decision_board'
const IMPLEMENT = 'implement_decision_board'

const priorityFilterKey = { plugin: 'decision-board', key: 'priorityFilter' } as const
const branchOnlyKey = { plugin: 'decision-board', key: 'branchOnly' } as const
const viewKey = { plugin: 'decision-board', key: 'view' } as const

// The part of `$.fs` the reading needs.
type Fs = {
  exists: (path: string) => Promise<boolean>
  read: (path: string) => Promise<string>
}

type Choice = { letter: string; label: string }
type Question = {
  id: string
  title: string
  status: string
  priority: string
  branch: string
  options: Choice[]
  context: string
  chosen: string | null
  resolution: string | null
  created_at: string
  updated_at: string
}

type View = 'pending' | 'implemented'

const PRIORITY_ORDER = ['urgent', 'high', 'normal', 'low', 'someday']

// One color per priority, so the list reads at a glance without counting on
// bold alone; a priority this register does not know about (future-proofing
// against a schema addition) falls back to no color rather than guessing.
const PRIORITY_COLOR: Record<string, string> = {
  urgent: 'red',
  high: 'yellow',
  normal: 'cyan',
  low: 'blue',
  someday: 'gray',
}

function rank(value: string): number {
  const index = PRIORITY_ORDER.indexOf(value)
  return index === -1 ? PRIORITY_ORDER.length : index
}

// Urgent first; `Array.prototype.sort` is a stable sort, so two questions of
// the same priority keep the order the register itself lists them in.
function sortQuestionsUrgentFirst(questions: Question[]): Question[] {
  return [...questions].sort((a, b) => rank(a.priority) - rank(b.priority))
}

// Open and decided are both still outstanding (an answer is not the same as
// an implementation); implemented, closed, dropped and obsolete are resting
// states this pane does not otherwise surface.
function pendingQuestions(questions: Question[]): Question[] {
  return questions.filter(q => q.status === 'open' || q.status === 'decided')
}

function implementedQuestions(questions: Question[]): Question[] {
  return questions.filter(q => q.status === 'implemented')
}

function questionsForView(questions: Question[], view: View): Question[] {
  return view === 'implemented' ? implementedQuestions(questions) : pendingQuestions(questions)
}

// The pane's own priority/branch filters, each optional and independent of
// the other. Narrowing is the board's own convenience: the agent-facing
// tools (show/read) never apply it, so an agent always sees the whole truth
// regardless of what a person last picked in the pane.
function filterQuestions(
  questions: Question[],
  filter: { priority: string | null; branch: string | null },
): Question[] {
  return questions.filter(
    q =>
      (filter.priority === null || q.priority === filter.priority) &&
      (filter.branch === null || q.branch === filter.branch),
  )
}

// Every question in the register, read from its own DECISIONS.json, sorted
// urgent-first. No register file at all reads as no questions, the same as
// register-board treats a missing BUGS.json/TODO.json.
async function loadQuestions(fs: Fs, root: string): Promise<Question[]> {
  const file = `${root}/DECISIONS.json`
  if (!(await fs.exists(file))) return []
  const parsed = JSON.parse(await fs.read(file)) as { questions?: Question[] }
  return sortQuestionsUrgentFirst(parsed.questions ?? [])
}

// The decisions binary's path, in the shared bin the installer puts it in --
// the same resolution ci-board's own toolPath uses for ci-failures.
function decisionsBin(home: string, xdg: string): string {
  const bin = xdg ? `${xdg}/tsch-ai-skills/bin` : `${home}/.config/tsch-ai-skills/bin`
  return `${bin}/decisions`
}

// The argument list `decisions answer <id> <letter> --file PATH` expects,
// built once here so the button handler and the agent-facing tool call the
// binary the identical way.
function answerArgs(bin: string, id: string, letter: string, file: string): string[] {
  return [bin, 'answer', id, letter, '--file', file]
}

// `decisions implement <id> [note] --file PATH`; note is optional, unlike
// answer's letter, so an empty one is simply omitted rather than passed as
// an empty positional.
function implementArgs(bin: string, id: string, note: string, file: string): string[] {
  const args = [bin, 'implement', id]
  if (note) args.push(note)
  args.push('--file', file)
  return args
}

// "Date when asked", in the loose relative phrasing a person reads faster
// than a timestamp: same-day counts in minutes/hours, this month in days,
// the next in "last month", older in whole months, then whole years. An
// unparseable timestamp reads as '' rather than 'NaN years ago'.
function relativeAge(iso: string, nowMs: number): string {
  const then = Date.parse(iso)
  if (!Number.isFinite(then)) return ''
  const seconds = Math.max(0, Math.round((nowMs - then) / 1000))
  const minute = 60
  const hour = 3600
  const day = 86400
  const month = 2_592_000 // 30 days
  const year = 31_536_000 // 365 days
  if (seconds < minute) return 'just now'
  if (seconds < hour) {
    const n = Math.floor(seconds / minute)
    return `${n} minute${n === 1 ? '' : 's'} ago`
  }
  if (seconds < day) {
    const n = Math.floor(seconds / hour)
    return `${n} hour${n === 1 ? '' : 's'} ago`
  }
  if (seconds < month) {
    const n = Math.floor(seconds / day)
    return n === 1 ? 'yesterday' : `${n} days ago`
  }
  if (seconds < month * 2) return 'last month'
  if (seconds < year) return `${Math.floor(seconds / month)} months ago`
  const n = Math.floor(seconds / year)
  return n === 1 ? 'last year' : `${n} years ago`
}

// The label text for whichever option letter was chosen, or the letter
// itself if the register no longer lists it (an option set edited after the
// pick, in principle -- defensive rather than expected).
function chosenLabel(question: Question): string {
  const picked = question.options.find(o => o.letter === question.chosen)
  return picked ? `${question.chosen}: ${picked.label}` : question.chosen ?? ''
}

// Exposes the pure functions above for a unit test to call directly, without
// spawning the binary or driving the mod runtime -- nothing here is read by
// Claude Code itself, which only ever reads the `register` export below.
export const __test = {
  sortQuestionsUrgentFirst,
  answerArgs,
  implementArgs,
  filterQuestions,
  pendingQuestions,
  implementedQuestions,
  questionsForView,
  relativeAge,
  chosenLabel,
}

export const register: Register = (on, options) => {
  if (options.enabled === false) return

  on('session.start', async ($, e, next) => {
    await $.command.register({
      name: 'decision-board',
      description: 'Show the project’s questions in a pane; `/decision-board close` hides it',
    })
    await $.tool.register({
      name: TOOL,
      description: 'Show the project’s pending (open + decided) questions to the person, in a pane, and return them as text for the chat.',
      inputSchema: { type: 'object', properties: {} },
    })
    await $.tool.register({
      name: READ,
      description: 'Read the project’s pending (open + decided) questions as text, without opening the pane.',
      inputSchema: { type: 'object', properties: {} },
    })
    await $.tool.register({
      name: ANSWER,
      description: 'Record the person’s pick for an open question: sets it to decided -- pending implementation, not yet done.',
      inputSchema: {
        type: 'object',
        properties: {
          id: { type: 'string', description: 'The question id, e.g. Q3.' },
          letter: { type: 'string', description: 'One of the question’s own option letters.' },
        },
        required: ['id', 'letter'],
      },
    })
    await $.tool.register({
      name: IMPLEMENT,
      description: 'Mark a decided question as carried out in the code, recording what was done. Refused unless the question is currently decided.',
      inputSchema: {
        type: 'object',
        properties: {
          id: { type: 'string', description: 'The question id, e.g. Q3.' },
          note: { type: 'string', description: 'What was implemented, for the record.' },
        },
        required: ['id'],
      },
    })
    return next(e)
  })

  on('command.run', { command: 'decision-board' }, async ($, e) => {
    if (e.args.trim().toLowerCase() === 'close') {
      await $.ui.close({ id: PANE })
      return { text: 'Decision board closed.' }
    }
    await $.ui.open({ id: PANE, title: 'Questions' })
    return { text: 'Decision board opened.' }
  })

  // The agent's tools (show/read/answer/implement) deliberately never apply
  // the pane's own priority/branch/view filters: an agent must always see
  // the whole truth, regardless of what a person last narrowed the view to.
  on('tool.call', { tool: `mcp__decision-board__${TOOL}` }, async $ => {
    const root = await $.session.root()
    const fs: Fs = { exists: path => $.fs.exists(path), read: path => $.fs.read(path) }
    const all = await loadQuestions(fs, root)
    const pending = pendingQuestions(all)
    await $.ui.open({ id: PANE, title: 'Questions' })

    return {
      result: [
        `Pending questions: ${pending.length}`,
        ...pending.map(q => `${q.id} [${q.priority}/${q.status}] ${q.title} (${q.branch})`),
        `Implemented: ${implementedQuestions(all).length}`,
      ].join('\n'),
    }
  })

  // The agent's read of the pending questions. Read-only; the pane is not opened.
  on('tool.call', { tool: `mcp__decision-board__${READ}` }, async $ => {
    const root = await $.session.root()
    const fs: Fs = { exists: path => $.fs.exists(path), read: path => $.fs.read(path) }
    const all = await loadQuestions(fs, root)
    const pending = pendingQuestions(all)
    const live = await $.ui.selection().catch(() => undefined)

    return {
      result: [
        `Pending questions: ${pending.length}`,
        ...pending.map(q => `${q.id} [${q.priority}/${q.status}] ${q.title} (${q.branch})`),
        `Implemented: ${implementedQuestions(all).length}`,
        `Highlighted in the board: ${live?.text ?? 'none'}`,
      ].join('\n'),
    }
  })

  // The agent's way to answer a question directly, without opening the pane
  // or pressing a button -- the same underlying call the pane's own buttons
  // make (answerArgs), so the two paths can never disagree about what the
  // binary is told.
  on('tool.call', { tool: `mcp__decision-board__${ANSWER}` }, async ($, e) => {
    const root = await $.session.root()
    const home = (await $.env.get('HOME')) ?? ''
    const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
    const id = String(e.id ?? '').trim()
    const letter = String(e.letter ?? '').trim()
    if (!id || !letter) return { result: 'Both id and letter are required.' }
    const file = `${root}/DECISIONS.json`
    const bin = decisionsBin(home, xdg)
    const run = await $.process.run(answerArgs(bin, id, letter, file))
    $.ui.invalidate('ui.render')
    return { result: run.exitCode === 0 ? `${id} decided: ${letter}.` : (run.stderr.trim() || `decisions exited ${run.exitCode}`) }
  })

  // The agent's way to record that a decided question's pick was carried
  // out, the same underlying call the pane's own "Mark implemented" button
  // makes (implementArgs).
  on('tool.call', { tool: `mcp__decision-board__${IMPLEMENT}` }, async ($, e) => {
    const root = await $.session.root()
    const home = (await $.env.get('HOME')) ?? ''
    const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
    const id = String(e.id ?? '').trim()
    const note = String(e.note ?? '').trim()
    if (!id) return { result: 'id is required.' }
    const file = `${root}/DECISIONS.json`
    const bin = decisionsBin(home, xdg)
    const run = await $.process.run(implementArgs(bin, id, note, file))
    $.ui.invalidate('ui.render')
    return { result: run.exitCode === 0 ? `${id} implemented.` : (run.stderr.trim() || `decisions exited ${run.exitCode}`) }
  })

  const REFRESH_MS = 2_000
  let refresh: { cancel: () => void } | undefined

  on('ui.render', { component: 'Pane', requestId: PANE }, async ($, e) => {
    const { Box, Button, Text } = $.ui.resolve(e)
    const root = await $.session.root()
    const home = (await $.env.get('HOME')) ?? ''
    const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
    const fs: Fs = { exists: path => $.fs.exists(path), read: path => $.fs.read(path) }
    const all = await loadQuestions(fs, root)

    if (!refresh) refresh = $.clock.every(REFRESH_MS, () => $.ui.invalidate('ui.render'))

    const { value: priorityFilter } = await $.state.get(priorityFilterKey)
    const { value: branchOnly } = await $.state.get(branchOnlyKey)
    const { value: viewValue } = await $.state.get(viewKey)
    const view: View = viewValue === 'implemented' ? 'implemented' : 'pending'
    // Only asks git for the current branch when the toggle actually needs it:
    // every other render costs one file read and nothing else.
    let activeBranch: string | null = null
    if (branchOnly) {
      const head = await $.process.run(['git', 'rev-parse', '--abbrev-ref', 'HEAD']).catch(() => undefined)
      activeBranch = head && head.exitCode === 0 ? head.stdout.trim() : null
    }
    const inView = questionsForView(all, view)
    const questions = filterQuestions(inView, { priority: priorityFilter ?? null, branch: activeBranch })
    const now = Date.now()

    const answer = async (id: string, letter: string) => {
      const file = `${root}/DECISIONS.json`
      const bin = decisionsBin(home, xdg)
      await $.process.run(answerArgs(bin, id, letter, file))
      $.ui.invalidate('ui.render')
    }

    const implementPick = async (id: string) => {
      const file = `${root}/DECISIONS.json`
      const bin = decisionsBin(home, xdg)
      await $.process.run(implementArgs(bin, id, '', file))
      $.ui.invalidate('ui.render')
    }

    const heading = view === 'implemented' ? 'Implemented questions' : 'Pending questions'

    return (
      <Box flexDirection="column">
        <Box flexDirection="row" justifyContent="space-between">
          <Text bold inverse>
            {heading}
          </Text>
          <Button role="dismiss" onPress={() => $.ui.close({ id: PANE })}>
            Close
          </Button>
        </Box>
        <Text> </Text>
        <Box flexDirection="row">
          <Text dimColor>View: </Text>
          <Button
            variant={view === 'pending' ? 'primary' : undefined}
            onPress={() => update($, viewKey, () => 'pending')}
          >
            Pending
          </Button>
          <Text> </Text>
          <Button
            variant={view === 'implemented' ? 'primary' : undefined}
            onPress={() => update($, viewKey, () => 'implemented')}
          >
            Implemented
          </Button>
        </Box>
        <Box flexDirection="row">
          <Text dimColor>Priority: </Text>
          {PRIORITY_ORDER.map(p => (
            <Box key={p} flexDirection="row">
              <Button
                variant={priorityFilter === p ? 'primary' : undefined}
                onPress={() => update($, priorityFilterKey, () => (priorityFilter === p ? null : p))}
              >
                {p}
              </Button>
              <Text> </Text>
            </Box>
          ))}
          <Button
            variant={branchOnly ? 'primary' : undefined}
            onPress={() => update($, branchOnlyKey, () => !branchOnly)}
          >
            this branch only
          </Button>
        </Box>
        <Text> </Text>
        {inView.length === 0 && <Text dimColor>{view === 'implemented' ? 'Nothing implemented yet.' : 'Nothing pending.'}</Text>}
        {inView.length > 0 && questions.length === 0 && <Text dimColor>No questions match this filter.</Text>}
        {questions.map(q => (
          <Box key={q.id} flexDirection="column">
            <Text bold color={PRIORITY_COLOR[q.priority]}>
              {`${q.id}  [${q.priority}${q.status === 'decided' ? ' · decided' : ''}]  ${q.title}`}
            </Text>
            <Text italic dimColor>
              {`   on ${q.branch || 'no branch recorded'} · asked ${relativeAge(q.created_at, now) || 'at an unknown time'}`}
            </Text>
            {q.status === 'open' && (
              <Box flexDirection="column">
                {q.options.map(option => (
                  <Box key={option.letter} flexDirection="row">
                    <Button onPress={() => answer(q.id, option.letter)}>{`${option.letter}: ${option.label}`}</Button>
                  </Box>
                ))}
              </Box>
            )}
            {q.status === 'decided' && (
              <Box flexDirection="column">
                <Text dimColor>{`   picked ${chosenLabel(q)} · awaiting implementation`}</Text>
                <Box flexDirection="row">
                  <Button onPress={() => implementPick(q.id)}>Mark implemented</Button>
                </Box>
              </Box>
            )}
            {q.status === 'implemented' && (
              <Text dimColor>
                {`   picked ${chosenLabel(q)}${q.resolution ? ` · ${q.resolution}` : ''} · implemented ${relativeAge(q.updated_at, now) || 'at an unknown time'}`}
              </Text>
            )}
            <Text> </Text>
          </Box>
        ))}
      </Box>
    )
  })
}
