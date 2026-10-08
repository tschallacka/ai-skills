import { expect, test } from 'claude-code/testing'
import { __test } from './register'

const { bugsBin, registersDir, openBugs } = __test

test('bugsBin resolves under XDG_CONFIG_HOME, falling back to HOME/.config', async () => {
  expect(bugsBin('', '/xdg')).toBe('/xdg/tsch-ai-skills/bin/bugs')
  expect(bugsBin('/home/x', '')).toBe('/home/x/.config/tsch-ai-skills/bin/bugs')
})

test('registersDir uses the resolved worktree directory when bugs resolve-path reports one', async () => {
  const seen: (readonly string[])[] = []
  const run = async (argv: readonly string[]) => {
    seen.push(argv)
    return { exitCode: 0, stdout: '/xdg/tsch-ai-skills/registers/me/proj/BUGS.json\n', stderr: '' }
  }
  const dir = await registersDir(run, '', '/xdg', '/project')
  expect(dir).toBe('/xdg/tsch-ai-skills/registers/me/proj')
  expect(seen).toEqual([['/xdg/tsch-ai-skills/bin/bugs', 'resolve-path']])
})

test('registersDir falls back to the session root on the bare-filename default', async () => {
  const run = async () => ({ exitCode: 0, stdout: 'BUGS.json\n', stderr: '' })
  const dir = await registersDir(run, '', '/xdg', '/project')
  expect(dir).toBe('/project')
})

test('registersDir falls back to the session root when the process fails to run at all', async () => {
  const run = async (): Promise<{ exitCode: number; stdout: string; stderr: string }> => {
    throw new Error('ENOENT')
  }
  const dir = await registersDir(run, '', '/xdg', '/project')
  expect(dir).toBe('/project')
})

test('registersDir falls back to the session root on a non-zero exit', async () => {
  const run = async () => ({ exitCode: 1, stdout: '', stderr: 'boom' })
  const dir = await registersDir(run, '', '/xdg', '/project')
  expect(dir).toBe('/project')
})

test('openBugs reads the register from the resolved directory, sorted and with closed entries dropped', async () => {
  const files: Record<string, string> = {
    '/project/.worktree/BUGS.json': JSON.stringify({
      bugs: [
        { id: 'B1', title: 'minor one', status: 'reported', severity: 'minor' },
        { id: 'B2', title: 'blocking one', status: 'reported', severity: 'blocking' },
        { id: 'B3', title: 'closed one', status: 'fixed', severity: 'major' },
      ],
    }),
  }
  const fs = {
    exists: async (path: string) => path in files,
    read: async (path: string) => files[path],
  }
  const run = async () => ({ exitCode: 0, stdout: '/project/.worktree/BUGS.json\n', stderr: '' })

  const bugs = await openBugs(fs, run, '', '/xdg', '/project')

  expect(bugs.map(b => b.id)).toEqual(['B2', 'B1'])
})

test('openBugs falls back to the session root when the binary is missing', async () => {
  const files: Record<string, string> = {
    '/project/BUGS.json': JSON.stringify({
      bugs: [{ id: 'B1', title: 'x', status: 'reported' }],
    }),
  }
  const fs = {
    exists: async (path: string) => path in files,
    read: async (path: string) => files[path],
  }
  const run = async (): Promise<{ exitCode: number; stdout: string; stderr: string }> => {
    throw new Error('spawn ENOENT')
  }

  const bugs = await openBugs(fs, run, '', '/xdg', '/project')

  expect(bugs.map(b => b.id)).toEqual(['B1'])
})
