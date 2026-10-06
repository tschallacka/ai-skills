import { expect, test } from 'claude-code/testing'

// turn-cue's hook runs on turn.complete and raises a ui.toast. The hooks a test
// registers with `on` stand beneath the plugin for the engine: one answers
// turn.complete, and one captures the toast the plugin raises.

const finished = {
  answer: 'done',
  durationMs: 42_000,
  isAborted: false,
  reason: 'answer',
} as const

test('a finished turn shows a toast with its length', { options: { enabled: true } }, async ($, on) => {
  const toasts: string[] = []
  on('ui.toast', (_$, e) => {
    toasts.push(e.text)
    return null as never
  })
  on('turn.complete', () => ({ text: '' }) as never)

  await $.turn.complete(finished as never)

  expect(toasts).toEqual(['Turn done in 42 s'])
})

test('switched off, no toast is shown', { options: { enabled: false } }, async ($, on) => {
  const toasts: string[] = []
  on('ui.toast', (_$, e) => {
    toasts.push(e.text)
    return null as never
  })
  on('turn.complete', () => ({ text: '' }) as never)

  await $.turn.complete(finished as never)

  expect(toasts).toEqual([])
})
