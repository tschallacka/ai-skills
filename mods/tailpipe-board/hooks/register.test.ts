import { expect, test } from 'claude-code/testing'
import { __test } from './register'

const { linesOf, streamsOf, tailpipeClientPath, tailpipeEndpointOf } = __test

test('linesOf parses a real client output line into its typed row shape', async () => {
  const lines = linesOf('1\thello world\n2\tsecond line\n')
  expect(lines).toEqual([
    { id: 1, text: 'hello world' },
    { id: 2, text: 'second line' },
  ])
})

test('linesOf ignores a blank trailing line', async () => {
  expect(linesOf('1\tonly line\n')).toEqual([{ id: 1, text: 'only line' }])
})

test('linesOf preserves a tab inside the text itself', async () => {
  expect(linesOf('1\ta\tb\n')).toEqual([{ id: 1, text: 'a\tb' }])
})

test('streamsOf splits one stream name per line and drops blanks', async () => {
  expect(streamsOf('s1\ns2\n\n')).toEqual(['s1', 's2'])
})

test('tailpipeClientPath resolves under XDG_CONFIG_HOME, falling back to HOME/.config', async () => {
  expect(tailpipeClientPath('', '/xdg')).toBe('/xdg/tsch-ai-skills/bin/tailpipe-client-rs')
  expect(tailpipeClientPath('/home/x', '')).toBe('/home/x/.config/tsch-ai-skills/bin/tailpipe-client-rs')
})

test('tailpipeEndpointOf resolves under XDG_CONFIG_HOME, falling back to HOME/.config', async () => {
  expect(tailpipeEndpointOf('', '/xdg')).toBe('/xdg/tsch-ai-skills/tailpipe/tailpipe.sock')
  expect(tailpipeEndpointOf('/home/x', '')).toBe('/home/x/.config/tsch-ai-skills/tailpipe/tailpipe.sock')
})
