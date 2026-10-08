import { expect, test } from 'claude-code/testing'
import { __test } from './register'

const { BOARD_LIST } = __test

test('every listed board has a unique id, a non-empty title and a non-empty label', () => {
  const ids = BOARD_LIST.map(board => board.id)
  expect(new Set(ids).size).toBe(ids.length)
  for (const board of BOARD_LIST) {
    expect(board.title.length).toBeGreaterThan(0)
    expect(board.label.length).toBeGreaterThan(0)
  }
})
