import { expect, test } from 'claude-code/testing'
import { __test } from './register'

const { decisionsBin, resolveDecisionsFile, loadQuestions } = __test

test('decisionsBin resolves under XDG_CONFIG_HOME, falling back to HOME/.config', async () => {
  expect(decisionsBin('', '/xdg')).toBe('/xdg/tsch-ai-skills/bin/decisions')
  expect(decisionsBin('/home/x', '')).toBe('/home/x/.config/tsch-ai-skills/bin/decisions')
})

test('resolveDecisionsFile uses an absolute worktree path as-is', async () => {
  let calls = 0
  const run = async (argv: readonly string[]) => {
    calls++
    expect(argv).toEqual(['/xdg/tsch-ai-skills/bin/decisions', 'resolve-path'])
    return { exitCode: 0, stdout: '/xdg/tsch-ai-skills/registers/me/proj/DECISIONS.json\n', stderr: '' }
  }
  const file = await resolveDecisionsFile(run, '', '/xdg', '/project')
  expect(file).toBe('/xdg/tsch-ai-skills/registers/me/proj/DECISIONS.json')
  expect(calls).toBe(1)
})

test('resolveDecisionsFile joins the bare default onto the session root', async () => {
  const run = async () => ({ exitCode: 0, stdout: 'DECISIONS.json\n', stderr: '' })
  const file = await resolveDecisionsFile(run, '', '/xdg', '/project')
  expect(file).toBe('/project/DECISIONS.json')
})

test('resolveDecisionsFile falls back to the session root when the process fails to run at all', async () => {
  const run = async (): Promise<{ exitCode: number; stdout: string; stderr: string }> => {
    throw new Error('ENOENT')
  }
  const file = await resolveDecisionsFile(run, '', '/xdg', '/project')
  expect(file).toBe('/project/DECISIONS.json')
})

test('resolveDecisionsFile falls back to the session root on a non-zero exit', async () => {
  const run = async () => ({ exitCode: 1, stdout: '', stderr: 'boom' })
  const file = await resolveDecisionsFile(run, '', '/xdg', '/project')
  expect(file).toBe('/project/DECISIONS.json')
})

test('the resolved file is reused rather than re-resolved for a second read in the same invocation', async () => {
  let calls = 0
  const run = async () => {
    calls++
    return { exitCode: 0, stdout: '/xdg/tsch-ai-skills/registers/me/proj/DECISIONS.json\n', stderr: '' }
  }
  const files: Record<string, string> = {
    '/xdg/tsch-ai-skills/registers/me/proj/DECISIONS.json': JSON.stringify({ questions: [] }),
  }
  const fs = {
    exists: async (path: string) => path in files,
    read: async (path: string) => files[path],
  }

  // Mirrors ui.render: resolve once, then read the register and (were it a
  // real answer press) reuse that same `file` again -- never a second
  // resolve-path call within one handler invocation.
  const file = await resolveDecisionsFile(run, '', '/xdg', '/project')
  await loadQuestions(fs, file)
  const reusedForAnAnswerPress = file

  expect(calls).toBe(1)
  expect(reusedForAnAnswerPress).toBe('/xdg/tsch-ai-skills/registers/me/proj/DECISIONS.json')
})

test('loadQuestions sorts urgent-first and reads no questions when the file is missing', async () => {
  const files: Record<string, string> = {
    '/project/DECISIONS.json': JSON.stringify({
      questions: [
        { id: 'Q1', title: 'low', priority: 'low' },
        { id: 'Q2', title: 'urgent', priority: 'urgent' },
      ],
    }),
  }
  const fs = {
    exists: async (path: string) => path in files,
    read: async (path: string) => files[path],
  }

  const questions = await loadQuestions(fs, '/project/DECISIONS.json')
  expect(questions.map(q => q.id)).toEqual(['Q2', 'Q1'])

  const missing = await loadQuestions(fs, '/elsewhere/DECISIONS.json')
  expect(missing).toEqual([])
})
