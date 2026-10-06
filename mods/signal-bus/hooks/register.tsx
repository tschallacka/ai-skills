import type { Register } from 'claude-code'
import { update } from 'claude-code'

// The signal bus: one place the session's boards and their tabs can be read from.
// Tabs: every pane that opens or closes, from any plugin, is kept in open order,
// and the pane holding the keyboard is kept as the focus. Signals: each board
// appends what the person did in it (a pick, a step opened) to `signals`, the
// newest last. `read_signal_bus` returns all of it as text. Toggled by `enabled`
// in settings.json pluginConfigs["signal-bus"].options.

const TOOL = 'read_signal_bus'
const panes = { plugin: 'signal-bus', key: 'panes' } as const
const focus = { plugin: 'signal-bus', key: 'focus' } as const
const signals = { plugin: 'signal-bus', key: 'signals' } as const
const KEEP = 20
// The boards whose state changes are signals. A plugin may write only its own
// state, so the boards do not write to the bus: the bus hears each change to
// theirs and records it.
const BOARDS = ['plan-board', 'brainstorm-board', 'tui-hint-board', 'ci-board']

// A changed value as one short line, for the signal's detail.
function summaryOf(value: unknown): string {
  const text = typeof value === 'string' ? value : (JSON.stringify(value) ?? 'none')
  const line = text.split('\n')[0] ?? ''
  return line.length > 120 ? `${line.slice(0, 117)}...` : line
}

export const register: Register = (on, options) => {
  if (options.enabled === false) return

  on('session.start', async ($, e, next) => {
    await $.tool.register({
      name: TOOL,
      description:
        'Read the boards the person has open: the open tabs, the tab holding the keyboard, and what the person did in the boards recently. Read-only; it opens nothing.',
      inputSchema: { type: 'object', properties: {} },
    })
    return next(e)
  })

  // A pane opened, by any plugin: it joins the tabs, unless the open was refused.
  on('ui.open', async ($, e, next) => {
    const result = await next(e)
    const refused = typeof result === 'object' && result !== null && 'deny' in result
    if (!refused) {
      await update($, panes, prev => [...(prev ?? []).filter(id => id !== e.id), e.id])
    }
    return result
  })

  // A pane closed, whoever closed it: it leaves the tabs and, if it held the keyboard, the focus.
  on('ui.close', async ($, e, next) => {
    const result = await next(e)
    await update($, panes, prev => (prev ?? []).filter(id => id !== e.id))
    await update($, focus, prev => (prev === e.id ? null : prev))
    return result
  })

  // The keyboard moving onto a pane. Another site (the prompt band) holds no pane.
  on('ui.focus', async ($, e, next) => {
    await update($, focus, () => (e.component === 'Pane' ? e.requestId : null))
    return next(e)
  })

  // A board's state changing is a signal: its key and the new value, newest last.
  on('state.set', async ($, e, next) => {
    if (BOARDS.includes(e.plugin)) {
      const signal = { at: Date.now(), board: e.plugin, kind: e.key, detail: summaryOf(e.value) }
      await update($, signals, prev => [...(prev ?? []), signal].slice(-KEEP))
    }
    return next(e)
  })

  on('tool.call', { tool: `mcp__signal-bus__${TOOL}` }, async $ => {
    const { value: openTabs } = await $.state.get(panes)
    const { value: held } = await $.state.get(focus)
    const { value: recent } = await $.state.get(signals)
    const tabs = openTabs ?? []
    const lines = [
      `Open tabs: ${tabs.length === 0 ? 'none' : tabs.join(', ')}`,
      `Holding the keyboard: ${held ?? 'none'}`,
      'Recent signals, newest first:',
    ]
    const shown = [...(recent ?? [])].reverse()
    if (shown.length === 0) lines.push('none yet')
    for (const signal of shown) {
      const when = new Date(signal.at).toISOString().slice(11, 16)
      lines.push(`${when} ${signal.board} ${signal.kind}: ${signal.detail}`)
    }
    return { result: lines.join('\n') }
  })
}
