import type { Register } from 'claude-code'

// The project's open questions (DECISIONS.json), most urgent first. The
// register is read on every draw, the same way register-board reads
// BUGS.json/TODO.json, so the pane always shows what is on disk; a 30-second
// timer also forces a redraw, the same pattern ci-board's JOB_REFRESH_MS
// uses, so a question answered elsewhere (the CLI, or an agent via MCP)
// disappears from the open list without the person reopening the pane.
// Toggled by `enabled` in settings.json pluginConfigs["decision-board"].options.

const PANE = 'decision-board'
const TOOL = 'show_decision_board'
const READ = 'read_decision_board'
const ANSWER = 'answer_decision_board'

// The part of `$.fs` the reading needs.
type Fs = {
  exists: (path: string) => Promise<boolean>
  read: (path: string) => Promise<string>
}

type Choice = { letter: string; label: string }
type Question = { id: string; title: string; status: string; priority: string; branch: string; options: Choice[] }

const PRIORITY_ORDER = ['urgent', 'high', 'normal', 'low', 'someday']

function rank(value: string): number {
  const index = PRIORITY_ORDER.indexOf(value)
  return index === -1 ? PRIORITY_ORDER.length : index
}

// Urgent first; `Array.prototype.sort` is a stable sort, so two questions of
// the same priority keep the order the register itself lists them in.
function sortQuestionsUrgentFirst(questions: Question[]): Question[] {
  return [...questions].sort((a, b) => rank(a.priority) - rank(b.priority))
}

// The project's open questions, read from its own DECISIONS.json, sorted
// urgent-first. No register file at all reads as no open questions, the same
// as register-board treats a missing BUGS.json/TODO.json.
async function openQuestions(fs: Fs, root: string): Promise<Question[]> {
  const file = `${root}/DECISIONS.json`
  if (!(await fs.exists(file))) return []
  const parsed = JSON.parse(await fs.read(file)) as { questions?: Question[] }
  return sortQuestionsUrgentFirst((parsed.questions ?? []).filter(question => question.status === 'open'))
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

// Exposes the pure functions above for a unit test to call directly, without
// spawning the binary or driving the mod runtime -- nothing here is read by
// Claude Code itself, which only ever reads the `register` export below.
export const __test = { sortQuestionsUrgentFirst, answerArgs }

export const register: Register = (on, options) => {
  if (options.enabled === false) return

  on('session.start', async ($, e, next) => {
    await $.command.register({
      name: 'decision-board',
      description: 'Show the project’s open questions in a pane; `/decision-board close` hides it',
    })
    await $.tool.register({
      name: TOOL,
      description: 'Show the project’s open questions to the person, in a pane, and return them as text for the chat.',
      inputSchema: { type: 'object', properties: {} },
    })
    await $.tool.register({
      name: READ,
      description: 'Read the project’s open questions as text, without opening the pane.',
      inputSchema: { type: 'object', properties: {} },
    })
    await $.tool.register({
      name: ANSWER,
      description: 'Record the person’s pick for an open question: sets it to answered.',
      inputSchema: {
        type: 'object',
        properties: {
          id: { type: 'string', description: 'The question id, e.g. Q3.' },
          letter: { type: 'string', description: 'One of the question’s own option letters.' },
        },
        required: ['id', 'letter'],
      },
    })
    return next(e)
  })

  on('command.run', { command: 'decision-board' }, async ($, e) => {
    if (e.args.trim().toLowerCase() === 'close') {
      await $.ui.close({ id: PANE })
      return { text: 'Decision board closed.' }
    }
    await $.ui.open({ id: PANE, title: 'Open questions' })
    return { text: 'Decision board opened.' }
  })

  on('tool.call', { tool: `mcp__decision-board__${TOOL}` }, async $ => {
    const root = await $.session.root()
    const fs: Fs = { exists: path => $.fs.exists(path), read: path => $.fs.read(path) }
    const questions = await openQuestions(fs, root)
    await $.ui.open({ id: PANE, title: 'Open questions' })

    return {
      result: [
        `Open questions: ${questions.length}`,
        ...questions.map(q => `${q.id} [${q.priority}] ${q.title} (${q.branch})`),
      ].join('\n'),
    }
  })

  // The agent's read of the open questions. Read-only; the pane is not opened.
  on('tool.call', { tool: `mcp__decision-board__${READ}` }, async $ => {
    const root = await $.session.root()
    const fs: Fs = { exists: path => $.fs.exists(path), read: path => $.fs.read(path) }
    const questions = await openQuestions(fs, root)
    const live = await $.ui.selection().catch(() => undefined)

    return {
      result: [
        `Open questions: ${questions.length}`,
        ...questions.map(q => `${q.id} [${q.priority}] ${q.title} (${q.branch})`),
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
    return { result: run.exitCode === 0 ? `${id} answered ${letter}.` : (run.stderr.trim() || `decisions exited ${run.exitCode}`) }
  })

  const REFRESH_MS = 30_000
  let refresh: { cancel: () => void } | undefined

  on('ui.render', { component: 'Pane', requestId: PANE }, async ($, e) => {
    const { Box, Button, Text } = $.ui.resolve(e)
    const root = await $.session.root()
    const home = (await $.env.get('HOME')) ?? ''
    const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
    const fs: Fs = { exists: path => $.fs.exists(path), read: path => $.fs.read(path) }
    const questions = await openQuestions(fs, root)

    if (!refresh) refresh = $.clock.every(REFRESH_MS, () => $.ui.invalidate('ui.render'))

    const answer = async (id: string, letter: string) => {
      const file = `${root}/DECISIONS.json`
      const bin = decisionsBin(home, xdg)
      await $.process.run(answerArgs(bin, id, letter, file))
      $.ui.invalidate('ui.render')
    }

    return (
      <Box flexDirection="column">
        <Box flexDirection="row" justifyContent="space-between">
          <Text bold inverse>
            Open questions
          </Text>
          <Button role="dismiss" onPress={() => $.ui.close({ id: PANE })}>
            Close
          </Button>
        </Box>
        <Text> </Text>
        {questions.length === 0 && <Text dimColor>No open questions.</Text>}
        {questions.map(q => (
          <Box key={q.id} flexDirection="column">
            <Text bold color={q.priority === 'urgent' || q.priority === 'high' ? 'yellow' : undefined}>
              {`${q.id}  [${q.priority}]  ${q.title}`}
            </Text>
            <Text italic dimColor>{`   on ${q.branch || 'no branch recorded'}`}</Text>
            <Box flexDirection="row">
              {q.options.map(option => (
                <Box key={option.letter} flexDirection="row">
                  <Button onPress={() => answer(q.id, option.letter)}>{`${option.letter}: ${option.label}`}</Button>
                  <Text> </Text>
                </Box>
              ))}
            </Box>
            <Text> </Text>
          </Box>
        ))}
      </Box>
    )
  })
}
