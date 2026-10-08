import { expect, test } from 'claude-code/testing'
import { __test } from './register'

const { todoBin, registersDir, openTasks } = __test

test('todoBin resolves under XDG_CONFIG_HOME, falling back to HOME/.config', async () => {
  expect(todoBin('', '/xdg')).toBe('/xdg/tsch-ai-skills/bin/todo')
  expect(todoBin('/home/x', '')).toBe('/home/x/.config/tsch-ai-skills/bin/todo')
})

test('registersDir uses the resolved worktree directory when todo resolve-path reports one', async () => {
  const seen: (readonly string[])[] = []
  const run = async (argv: readonly string[]) => {
    seen.push(argv)
    return { exitCode: 0, stdout: '/xdg/tsch-ai-skills/registers/me/proj/TODO.json\n', stderr: '' }
  }
  const dir = await registersDir(run, '', '/xdg', '/project')
  expect(dir).toBe('/xdg/tsch-ai-skills/registers/me/proj')
  expect(seen).toEqual([['/xdg/tsch-ai-skills/bin/todo', 'resolve-path']])
})

test('registersDir falls back to the session root on the bare-filename default', async () => {
  const run = async () => ({ exitCode: 0, stdout: 'TODO.json\n', stderr: '' })
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

test('openTasks reads the register from the resolved directory, sorted and with closed entries dropped', async () => {
  const files: Record<string, string> = {
    '/project/.worktree/TODO.json': JSON.stringify({
      tasks: [
        { id: 'T1', title: 'low one', status: 'open', priority: 'low' },
        { id: 'T2', title: 'urgent one', status: 'open', priority: 'urgent' },
        { id: 'T3', title: 'done one', status: 'done', priority: 'normal' },
      ],
    }),
  }
  const fs = {
    exists: async (path: string) => path in files,
    read: async (path: string) => files[path],
  }
  const run = async () => ({ exitCode: 0, stdout: '/project/.worktree/TODO.json\n', stderr: '' })

  const tasks = await openTasks(fs, run, '', '/xdg', '/project')

  expect(tasks.map(t => t.id)).toEqual(['T2', 'T1'])
})

test('openTasks falls back to the session root when the binary is missing', async () => {
  const files: Record<string, string> = {
    '/project/TODO.json': JSON.stringify({
      tasks: [{ id: 'T1', title: 'x', status: 'open', priority: 'normal' }],
    }),
  }
  const fs = {
    exists: async (path: string) => path in files,
    read: async (path: string) => files[path],
  }
  const run = async (): Promise<{ exitCode: number; stdout: string; stderr: string }> => {
    throw new Error('spawn ENOENT')
  }

  const tasks = await openTasks(fs, run, '', '/xdg', '/project')

  expect(tasks.map(t => t.id)).toEqual(['T1'])
})
