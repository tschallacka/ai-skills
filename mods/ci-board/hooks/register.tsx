import type { Register } from 'claude-code'
import { update } from 'claude-code'

import { LOADING_FRAMES, LOADING_FRAME_MS, loadingArt, pickSlogan } from './loading'

// The failing jobs of the branch's latest CI run, as the ci-failures tool reports
// them, or the list of the repo's recent runs: a run picked from the list shows its
// own report. The tool talks to the network, so it runs when the pane opens, when
// the person presses [refresh] or a run, not on a timer. The last report, the
// browsing state, the run list and the picked run are kept in $.state so a redraw
// keeps them. Toggled by `enabled` in settings.json pluginConfigs["ci-board"].options.

const PANE = 'ci-board'
const TOOL = 'show_ci_board'
const READ = 'read_ci_board'
const report = { plugin: 'ci-board', key: 'report' } as const
const browsing = { plugin: 'ci-board', key: 'browsing' } as const
const runs = { plugin: 'ci-board', key: 'runs' } as const
const picked = { plugin: 'ci-board', key: 'run' } as const
const narrow = { plugin: 'ci-board', key: 'narrow' } as const
const busy = { plugin: 'ci-board', key: 'busy' } as const
const progress = { plugin: 'ci-board', key: 'progress' } as const

// The loading screen's animation frame and slogan, which the timer moves while a fetch runs.
let frame = 0
let line = ''
let ticker: { cancel: () => void } | undefined

// The run list's filter buttons: the label and the test a run must pass to be listed.
const FILTERS: { label: string; tone: string; keeps: (conclusion: string) => boolean }[] = [
  { label: 'success', tone: 'green', keeps: conclusion => conclusion === 'success' },
  { label: 'failed', tone: 'red', keeps: conclusion => conclusion === 'failure' || conclusion === 'timed_out' },
  { label: 'cancelled', tone: 'gray', keeps: conclusion => conclusion === 'cancelled' },
  { label: 'in progress', tone: 'yellow', keeps: conclusion => !['success', 'failure', 'timed_out', 'cancelled', 'skipped', 'neutral', 'action_required', 'stale'].includes(conclusion) },
]

// A run in the browse list: its id, what it was for, and how it ended.
type RunRow = { id: string; title: string; branch: string; conclusion: string; when: string; age: string; took: string }

// How long a run took, from when it started to when it finished, as "4m 12s".
function tookOf(startedAt: string, finishedAt: string): string {
  const seconds = Math.round((Date.parse(finishedAt) - Date.parse(startedAt)) / 1000)
  if (!Number.isFinite(seconds) || seconds < 0) return 'unknown'
  if (seconds < 60) return `${seconds}s`
  const minutes = Math.floor(seconds / 60)
  if (minutes < 60) return `${minutes}m ${seconds % 60}s`
  return `${Math.floor(minutes / 60)}h ${minutes % 60}m`
}

// How long ago a run started, in the largest whole unit, as "1 hour ago". The clock is
// read from the system (`date`) when the list is fetched, not from the plugin runtime.
function ago(createdAt: string, nowMs: number): string {
  const seconds = Math.max(0, Math.round((nowMs - Date.parse(createdAt)) / 1000))
  const units: [number, string][] = [
    [31536000, 'year'],
    [2592000, 'month'],
    [604800, 'week'],
    [86400, 'day'],
    [3600, 'hour'],
    [60, 'minute'],
  ]
  for (const [size, name] of units) {
    if (seconds >= size) {
      const count = Math.floor(seconds / size)
      return `${count} ${name}${count === 1 ? '' : 's'} ago`
    }
  }
  return 'just now'
}

// What the tool said: its report on standard output, or its message on standard
// error when it has no report (no runs for the branch, for example).
function reportOf(run: { exitCode: number; stdout: string; stderr: string }): string {
  const out = run.stdout.trim()
  if (out !== '') return out
  const err = run.stderr.trim()
  return err !== '' ? err : `ci-failures exited ${run.exitCode} with no output.`
}

// The tool's own binary, in the shared bin the installer puts it in.
function toolPath(home: string, xdg: string): string {
  const bin = xdg ? `${xdg}/tsch-ai-skills/bin` : `${home}/.config/tsch-ai-skills/bin`
  return `${bin}/ci-failures`
}

// The repo's recent runs, from gh's JSON; an unreadable answer is no runs.
function runsOf(stdout: string, nowMs: number): RunRow[] {
  try {
    const rows = JSON.parse(stdout) as {
      databaseId: number
      displayTitle: string
      headBranch: string
      conclusion: string
      status: string
      createdAt: string
      startedAt: string
      updatedAt: string
    }[]
    return rows.map(row => ({
      id: String(row.databaseId),
      title: row.displayTitle,
      branch: row.headBranch,
      conclusion: row.conclusion || row.status,
      when: row.createdAt.slice(0, 16).replace('T', ' '),
      age: ago(row.createdAt, nowMs),
      took: tookOf(row.startedAt, row.updatedAt),
    }))
  } catch {
    return []
  }
}

export const register: Register = (on, options) => {
  if (options.enabled === false) return

  on('session.start', async ($, e, next) => {
    await $.command.register({
      name: 'ci-board',
      description: 'Show the failing jobs of the latest CI run in a pane; `/ci-board close` hides it',
    })
    await $.tool.register({
      name: TOOL,
      description:
        'Show the failing CI jobs of the current branch to the person, in a pane, and return the report as text for the chat.',
      inputSchema: { type: 'object', properties: {} },
    })
    await $.tool.register({
      name: READ,
      description: 'Read the CI report the person has in the CI board, as text, without running the check or opening the pane.',
      inputSchema: { type: 'object', properties: {} },
    })
    return next(e)
  })

  on('command.run', { command: 'ci-board' }, async ($, e) => {
    if (e.args.trim().toLowerCase() === 'close') {
      await $.ui.close({ id: PANE })
      return { text: 'CI board closed.' }
    }
    await $.ui.open({ id: PANE, title: 'CI failures' })
    return { text: 'CI board opened.' }
  })

  on('tool.call', { tool: `mcp__ci-board__${TOOL}` }, async $ => {
    const home = (await $.env.get('HOME')) ?? ''
    const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
    const run = await $.process.run([toolPath(home, xdg)])
    const text = reportOf(run)
    await update($, report, () => text)
    await $.ui.open({ id: PANE, title: 'CI failures' })

    return { result: text }
  })

  // The agent's read of the report the pane holds. It does not run the check and
  // does not open the pane.
  on('tool.call', { tool: `mcp__ci-board__${READ}` }, async $ => {
    const { value: last } = await $.state.get(report)
    const { value: run } = await $.state.get(picked)
    const { value: isBrowsing } = await $.state.get(browsing)
    const live = await $.ui.selection().catch(() => undefined)
    const shown = last ?? 'No CI report yet. The board has not been refreshed.'
    const lines = [
      run && isBrowsing
        ? `Run in the board: ${run.id} (${run.title})`
        : isBrowsing
          ? 'Run in the board: none picked (the run list is shown)'
          : 'Run in the board: none (the branch report is shown)',
    ]
    if (run && isBrowsing) lines.push(run.text)
    else lines.push(shown)
    lines.push(`Highlighted in the board: ${live?.text ?? 'none'}`)
    return { result: lines.join('\n') }
  })

  on('ui.render', { component: 'Pane', requestId: PANE }, async ($, e) => {
    const { Box, Button, Code, Link, Text } = $.ui.resolve(e)
    const { value: last } = await $.state.get(report)
    const { value: isBrowsing } = await $.state.get(browsing)
    const { value: list } = await $.state.get(runs)
    const { value: run } = await $.state.get(picked)
    const { value: filterLabel } = await $.state.get(narrow)
    const active = FILTERS.find(item => item.label === filterLabel)
    const shown = (list ?? []).filter(row => (active ? active.keeps(row.conclusion) : true))
    const lines = (last ?? 'Not checked yet. Press [refresh] to read the latest CI run.').split('\n')

    // Shows the loading screen while the steps run, one after another, with a progress bar
    // that fills as each step finishes, to 100% when the last one is done.
    const busyWhile = async (label: string, steps: (() => Promise<void>)[]) => {
      frame = 0
      line = pickSlogan()
      ticker?.cancel()
      ticker = $.clock.every(LOADING_FRAME_MS, () => {
        frame += 1
        $.ui.invalidate('ui.render')
      })
      await update($, busy, () => label)
      await update($, progress, () => ({ done: 0, total: steps.length }))
      try {
        for (const [index, step] of steps.entries()) {
          await step()
          await update($, progress, () => ({ done: index + 1, total: steps.length }))
        }
      } finally {
        ticker?.cancel()
        ticker = undefined
        await update($, busy, () => null)
        await update($, progress, () => null)
      }
    }

    // The run list: each row is a button; a press reads that run's own report.
    const openRun = async (row: RunRow) => {
      const home = (await $.env.get('HOME')) ?? ''
      const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
      let text = ''
      let url = ''
      await busyWhile(`run ${row.id}`, [
        async () => {
          text = reportOf(await $.process.run([toolPath(home, xdg), row.id]))
        },
        async () => {
          // The run's page on GitHub, for the link in its details; empty when gh cannot say.
          const page = await $.process.run(['gh', 'run', 'view', row.id, '--json', 'url'])
          url = ''
          if (page.exitCode === 0) {
            try {
              url = JSON.parse(page.stdout).url ?? ''
            } catch {
              url = ''
            }
          }
        },
        async () => {
          await update($, picked, () => ({ id: row.id, title: row.title, text, url, took: row.took, conclusion: row.conclusion }))
        },
      ])
    }
    const browse = () => {
      let nowMs = NaN
      let rows: RunRow[] = []
      return busyWhile('the run list', [
        async () => {
          const clock = await $.process.run(['date', '+%s'])
          nowMs = Number(clock.stdout.trim()) * 1000
        },
        async () => {
          const result = await $.process.run(['gh', 'run', 'list', '--limit', '20', '--json', 'databaseId,displayTitle,headBranch,conclusion,status,createdAt,startedAt,updatedAt'])
          rows = result.exitCode === 0 && Number.isFinite(nowMs) ? runsOf(result.stdout, nowMs) : []
        },
        async () => {
          await update($, runs, () => rows)
          await update($, picked, () => null)
          await update($, browsing, () => true)
        },
      ])
    }

    const { value: fetching } = await $.state.get(busy)
    const { value: task } = await $.state.get(progress)
    let body
    if (fetching) {
      const dots = '.'.repeat((frame % 3) + 1)
      const art = loadingArt(frame % LOADING_FRAMES, 'LOADING', line)
      const done = task?.done ?? 0
      const total = Math.max(task?.total ?? 1, 1)
      const percent = Math.round((100 * done) / total)
      const width = 30
      const filled = Math.round((width * done) / total)
      body = (
        <Box flexDirection="column">
          <Text bold color="cyan">{`Fetching ${fetching}${dots}`}</Text>
          <Text> </Text>
          <Code source={art.join('\n')} />
          <Text color="cyan">{`[${'#'.repeat(filled)}${'-'.repeat(width - filled)}] ${percent}%`}</Text>
          <Text dimColor>{`step ${done} of ${total}`}</Text>
        </Box>
      )
    } else if (isBrowsing && run) {
      body = (
        <Box flexDirection="column">
          <Button variant="primary" onPress={() => update($, picked, () => null)}>
            back to runs
          </Button>
          <Text bold>{`Run ${run.id}: ${run.title}`}</Text>
          {run.url && (
            <Box flexDirection="row">
              <Button onPress={() => $.ui.copy({ text: run.url })}>copy link</Button>
              <Text> </Text>
              <Text dimColor>{run.url}</Text>
            </Box>
          )}
          <Text>{`${run.conclusion === 'success' ? 'Passed' : run.conclusion}  ·  took ${run.took ?? 'unknown'}`}</Text>
          {run.url ? (
            <Button
              onPress={async () => {
                // The desktop's own opener, tried in turn: xdg-open, then gio (GNOME), then open (macOS).
                for (const opener of [['xdg-open', run.url], ['gio', 'open', run.url], ['open', run.url]]) {
                  const opened = await $.process.run(opener).catch(() => ({ exitCode: 1 }))
                  if (opened.exitCode === 0) break
                }
              }}
            >
              view in browser
            </Button>
          ) : (
            <Text dimColor>view in browser: GitHub did not give this run's link</Text>
          )}
          <Text> </Text>
          {run.text.split('\n').map((line, index) => (
            <Text key={index} color={/fail|red|✘/i.test(line) ? 'red' : undefined}>
              {line}
            </Text>
          ))}
        </Box>
      )
    } else if (isBrowsing) {
      body = (
        <Box flexDirection="column">
          <Box flexDirection="row">
            <Button variant="primary" onPress={() => update($, browsing, () => false)}>
              back to report
            </Button>
            <Text> </Text>
            <Button onPress={browse}>refresh runs</Button>
          </Box>
          <Text> </Text>
          <Text bold underline color="cyan">
            Filter
          </Text>
          <Box flexDirection="row">
            {FILTERS.map(item => (
              <Box key={item.label} flexDirection="row">
                <Box minWidth={2}>
                  <Text color={item.tone}>●</Text>
                </Box>
                <Button
                  variant={active?.label === item.label ? 'primary' : undefined}
                  onPress={() => update($, narrow, () => (active?.label === item.label ? null : item.label))}
                >
                  {item.label}
                </Button>
                <Text> </Text>
              </Box>
            ))}
          </Box>
          <Text> </Text>
          {(list ?? []).length === 0 && <Text dimColor>No runs listed. Check that gh is signed in for this repo.</Text>}
          {shown.map(row => {
            // Green for a success, red for a failure, grey for a cancel, yellow while it is still running.
            const tone =
              row.conclusion === 'success' ? 'green'
              : ['failure', 'timed_out'].includes(row.conclusion) ? 'red'
              : row.conclusion === 'cancelled' ? 'gray'
              : 'yellow'
            // A success needs no word: its colour says it. Anything else names its state.
            const word = row.conclusion === 'success' ? '' : `${row.conclusion}  `
            return (
              <Box key={row.id} flexDirection="column">
                <Box flexDirection="row">
                  <Box minWidth={2}>
                    <Text color={tone}>●</Text>
                  </Box>
                  <Button onPress={() => openRun(row)}>{`${word}${row.title}`}</Button>
                </Box>
                <Text italic>{`   ${row.when}  ·  ${row.age ?? 'press refresh runs for its age'}  ·  took ${row.took ?? 'unknown'}  ·  on ${row.branch}`}</Text>
              </Box>
            )
          })}
        </Box>
      )
    } else {
      body = (
        <Box flexDirection="column">
          <Box flexDirection="row">
            <Button
              variant="primary"
              onPress={() =>
                busyWhile('the branch report', [
                  async () => {
                    const home = (await $.env.get('HOME')) ?? ''
                    const xdg = (await $.env.get('XDG_CONFIG_HOME')) ?? ''
                    const result = await $.process.run([toolPath(home, xdg)])
                    await update($, report, () => reportOf(result))
                  },
                ])
              }
            >
              refresh
            </Button>
            <Text> </Text>
            <Button onPress={browse}>browse runs</Button>
          </Box>
          <Text> </Text>
          {lines.map((line, index) => (
            <Text key={index} color={/fail|red|✘/i.test(line) ? 'yellow' : undefined}>
              {line}
            </Text>
          ))}
        </Box>
      )
    }

    return (
      <Box flexDirection="column">
        <Box flexDirection="row" justifyContent="space-between">
          <Text bold inverse>
            CI failures
          </Text>
          <Button role="dismiss" onPress={() => $.ui.close({ id: PANE })}>
            Close
          </Button>
        </Box>
        <Text> </Text>
        {body}
      </Box>
    )
  })
}
