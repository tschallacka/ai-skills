// The loading animation: a runner with a stack of papers in his arms, drawn on a small
// canvas in front of a scrolling office. A sign on the wall above the desk shows a word
// in big letters, with a slogan under it. The office enters from the right edge and
// leaves on the left; the legs cycle with bent knees and their stride matches the floor's
// speed; a sheet is thrown up from the stack every quarter of the loop and drifts behind.
//
// This file is the one source of the animation. Each mod that uses it gets a copy at its
// own hooks/loading.tsx, made by mods/loading/sync.sh; edit it here, then run the script.
// Use it from a mod with:
//   import { LOADING_FRAMES, LOADING_FRAME_MS, loadingArt, pickSlogan } from './loading'
// and draw loadingArt(frame % LOADING_FRAMES, word, line).join('\n') in a Code element.

export const LOADING_FRAME_MS = 70

const CANVAS_W = 46
const CANVAS_H = 17
const FLOOR_ROW = CANVAS_H - 1
const RUNNER_X = 14
const OFFICE_HEIGHT = 16

// The door is as tall as the canvas, so its top stands well above the runner's head.
const DOOR = [
  '____________',
  ...Array.from({ length: 14 }, (_, i) => (i === 8 ? '|      o   |' : '|          |')),
  '|__________|',
]
// The sign and desk prop is one width whatever the word or slogan, so the loop does not change.
const SIGN_WIDTH = 39
// Big letters for the sign, five rows each, four columns wide. A letter not listed is blank.
const LETTERS: Record<string, string[]> = {
  L: ['#   ', '#   ', '#   ', '#   ', '####'],
  O: ['####', '#  #', '#  #', '#  #', '####'],
  A: ['####', '#  #', '####', '#  #', '#  #'],
  D: ['### ', '#  #', '#  #', '#  #', '### '],
  I: ['####', ' ## ', ' ## ', ' ## ', '####'],
  N: ['#  #', '## #', '# ##', '#  #', '#  #'],
  G: ['####', '#   ', '# ##', '#  #', '####'],
}
const BLANK_LETTER = ['    ', '    ', '    ', '    ', '    ']

const SLOGANS = [
  'Hamsters are running as fast as they can',
  'Reticulating splines',
  'Asking the intern nicely',
  'Consulting the office oracle',
  'Filing the paperwork in triplicate',
  'The coffee machine is thinking',
  'Almost there, pinky promise',
  'Do not feed the paper shredder',
  'Unjamming the printer, again',
  "It's not a bug, it's a feature",
  'Please hold, your call is important',
  'Productivity at 110 percent, loading',
]
// A slogan picked at random, for a caller that wants one each time the screen starts.
export function pickSlogan(): string {
  return SLOGANS[Math.floor(Math.random() * SLOGANS.length)] ?? SLOGANS[0]!
}

const centred = (text: string, width: number) => {
  const cut = text.slice(0, width)
  const left = Math.floor((width - cut.length) / 2)
  return ' '.repeat(left) + cut + ' '.repeat(width - cut.length - left)
}

// The sign board on the wall, its word in big letters, the slogan under it, and the
// desk beneath, all as one prop of OFFICE_HEIGHT rows.
function signDesk(word: string, line: string): string[] {
  const letters = word.toUpperCase().split('').map(ch => LETTERS[ch] ?? BLANK_LETTER)
  const big = Array.from({ length: 5 }, (_, r) => letters.map(g => g[r]).join(' '))
  const inner = SIGN_WIDTH - 2
  const board = [
    '+' + '-'.repeat(inner) + '+',
    ...big.map(row => '|' + centred(row, inner) + '|'),
    '+' + '-'.repeat(inner) + '+',
  ]
  const desk = [
    centred('_'.repeat(27), SIGN_WIDTH),
    centred('|' + '_'.repeat(25) + '|', SIGN_WIDTH),
    centred('| |' + ' '.repeat(21) + '| |', SIGN_WIDTH),
    centred('| |' + ' '.repeat(21) + '| |', SIGN_WIDTH),
  ]
  const rows = [...board, centred(line, SIGN_WIDTH), ...desk]
  return [...Array.from({ length: OFFICE_HEIGHT - rows.length }, () => ' '.repeat(SIGN_WIDTH)), ...rows]
}

// The office props in order: the sign above the desk first, then a plant, a door, and a
// set of drawers.
function officeProps(word: string, line: string): string[][] {
  return [
    signDesk(word, line),
    ['  \\|/ ', ' \\|||/', '  \\|/ ', '  [_] '],
    DOOR,
    ['+-----+', '|=====|', '+-----+', '|=====|', '+-----+'],
  ]
}

// The gap is chosen so the loop is a whole number of strides (192 = 8 x 24 frames).
const OFFICE_GAP = 32
// The office's length in columns. The animation is this plus the canvas width, so the
// office enters from the right and leaves on the left with nothing on screen at the seam.
const OFFICE_LOOP = officeProps('', '').reduce((sum, prop) => sum + prop[0]!.length + OFFICE_GAP, 0)

// One text row per office row, the props side by side with their gaps between them. A
// shorter prop is padded at the top so its feet sit on the floor.
function officeRows(word: string, line: string): string[] {
  const padded = officeProps(word, line).map(prop => [
    ...Array.from({ length: OFFICE_HEIGHT - prop.length }, () => ' '.repeat(prop[0]!.length)),
    ...prop,
  ])
  return Array.from({ length: OFFICE_HEIGHT }, (_, row) =>
    padded.map(prop => prop[row]! + ' '.repeat(OFFICE_GAP)).join(''),
  )
}

const RUNNER_TOP = [
  '    ____    ',
  '   /o  o\\   ',
  '   \\_..-/   ',
  '    /|      ',
  '   /|[=====]',
  '  < |[=====]',
  '   \\|______|',
]

// The legs are two bones each, a thigh and a shin of equal length, solved from the hip
// to a foot that moves in a loop: forward and lifted while it swings, back on the floor
// while it pushes. The knee always bends forward. Units are rows; a column is half a
// unit, because a character is about twice as tall as it is wide.
const HIP = { u: 2.5, v: 7 }
const FEET_ROW = 12
const BONE = 3
// One stride, both legs, takes GAIT_FRAMES frames. A foot moves 2 * STRIDE units (four
// columns per unit of stride) across the floor while it pushes, half the stride. The
// background moves one column a frame, so the stride is GAIT_FRAMES / 8 units: the floor
// slides under the feet at the speed the feet travel.
const GAIT_FRAMES = 24
const STRIDE = GAIT_FRAMES / 8
const LIFT = 2.5
type Pt = { u: number; v: number }

// The two legs at one phase of the gait, as [hip, knee, foot] for each leg. The legs are
// half a cycle apart.
function legsAt(phase: number): [Pt, Pt, Pt][] {
  return [0, Math.PI].map(shift => {
    const p = phase + shift
    // The foot swings forward in the air (sin below zero) and pushes back on the floor.
    const foot: Pt = { u: HIP.u + STRIDE * Math.cos(p), v: FEET_ROW - LIFT * Math.max(0, -Math.sin(p)) }
    const dx = foot.u - HIP.u
    const dy = foot.v - HIP.v
    const d = Math.min(Math.hypot(dx, dy), BONE * 2 - 0.01)
    const mid: Pt = { u: (HIP.u + foot.u) / 2, v: (HIP.v + foot.v) / 2 }
    const h = Math.sqrt(BONE * BONE - (d / 2) * (d / 2))
    const nu = -dy / Math.hypot(dx, dy)
    const nv = dx / Math.hypot(dx, dy)
    const a: Pt = { u: mid.u + nu * h, v: mid.v + nv * h }
    const b: Pt = { u: mid.u - nu * h, v: mid.v - nv * h }
    const knee = a.u > b.u ? a : b
    return [HIP, knee, foot]
  })
}

// The character for one bone's direction: a floor line, an upright, or a diagonal.
function boneChar(from: Pt, to: Pt): string {
  const dc = (to.u - from.u) * 2
  const dr = to.v - from.v
  if (Math.abs(dr) < 0.6) return '_'
  if (Math.abs(dc) < 0.6 * Math.abs(dr)) return '|'
  return dc * dr > 0 ? '\\' : '/'
}

// The cells a leg covers: each bone sampled along its length, as [row, column, char].
// The knee is drawn as a '>' pointing forward, not as the upright of the bone above it.
function legCells(top: number, phase: number): [number, number, string][] {
  const cells: [number, number, string][] = []
  for (const leg of legsAt(phase)) {
    for (let bone = 0; bone < 2; bone++) {
      const from = leg[bone]!
      const to = leg[bone + 1]!
      const steps = Math.ceil(Math.hypot((to.u - from.u) * 2, to.v - from.v) * 2) || 1
      const ch = boneChar(from, to)
      for (let i = bone === 0 ? 0 : 1; i <= steps; i++) {
        const u = from.u + ((to.u - from.u) * i) / steps
        const v = from.v + ((to.v - from.v) * i) / steps
        const isKnee = bone === 0 && i === steps
        cells.push([top + Math.round(v), RUNNER_X + Math.round(u * 2), isKnee ? '>' : ch])
      }
    }
  }
  return cells
}

// The sheet seen face-on, tilting, edge-on, and tilting back: a flip, one state a frame.
const SHEET_STATES: string[][] = [
  ['+---+', '|===|', '+---+'],
  ['/---\\', '|===|', '\\---/'],
  ['  |  ', '  |  ', '  |  '],
  ['/---\\', '|...|', '\\---/'],
]
// A sheet flies for this many frames. It stays put in the air while the office scrolls
// past under it, so it is left behind as the guy moves forward: one column back a frame,
// rising, then sinking.
const SHEET_FLIGHT = 40
const SHEET_LOOP = OFFICE_LOOP + CANVAS_W + 2
const SHEET_THROW_EVERY = SHEET_LOOP / 4
const SHEET_START_ROW = 9
const SHEET_START_COL = 26

// Where the sheets in the air are on this frame, as [row, column, state]. A sheet rises
// from the stack for the first part of its flight and sinks for the rest.
function sheetsAt(index: number): [number, number, number][] {
  const found: [number, number, number][] = []
  for (let k = 0; k < 4; k++) {
    const t = (((index - k * SHEET_THROW_EVERY) % SHEET_LOOP) + SHEET_LOOP) % SHEET_LOOP
    if (t > SHEET_FLIGHT) continue
    const row = Math.round(SHEET_START_ROW - 10 * Math.sin((Math.PI * t) / SHEET_FLIGHT) + t * 0.15)
    const col = SHEET_START_COL - t
    found.push([row, col, (index + k) % SHEET_STATES.length])
  }
  return found
}

// The number of frames before the animation repeats. Use it as the modulus for a frame count.
export const LOADING_FRAMES = SHEET_LOOP

// One animation frame as rows of text, ready to join with newlines for a Code element.
// The word is set in big letters on the sign; the line is the slogan under it.
export function loadingArt(index: number, word = 'LOADING', line = ''): string[] {
  const canvas: string[][] = Array.from({ length: CANVAS_H }, () => Array(CANVAS_W).fill(' '))
  const put = (row: number, col: number, text: string) => {
    for (let i = 0; i < text.length; i++) {
      const c = col + i
      if (row >= 0 && row < CANVAS_H && c >= 0 && c < CANVAS_W && text[i] !== ' ') canvas[row]![c] = text[i]!
    }
  }
  // A dotted ceiling on the top row, drawn first so the office covers it. It keeps the
  // top row non-blank in every frame, so the Code block never shrinks when the door leaves.
  for (let col = 0; col < CANVAS_W; col += 2) canvas[0]![col] = '.'
  // The office: at frame 0 its first column is just past the right edge, and it moves one
  // column left a frame.
  officeRows(word, line).forEach((text, row) => {
    for (let col = 0; col < CANVAS_W; col++) {
      const at = col + index - CANVAS_W
      if (at < 0 || at >= OFFICE_LOOP) continue
      const ch = text[at]!
      if (ch !== ' ') canvas[FLOOR_ROW - OFFICE_HEIGHT + row]![col] = ch
    }
  })
  for (let col = 0; col < CANVAS_W; col++) canvas[FLOOR_ROW]![col] = '_'
  // The runner, feet on the floor. Where his figure is, it marks the canvas, so a sheet
  // passing behind him is hidden by him.
  const top = FLOOR_ROW - 1 - FEET_ROW
  const legs = legCells(top, ((index % GAIT_FRAMES) / GAIT_FRAMES) * 2 * Math.PI)
  const solid = Array.from({ length: CANVAS_H }, () => Array(CANVAS_W).fill(false))
  RUNNER_TOP.forEach((text, i) => {
    for (let c = 0; c < text.length; c++) {
      const row = top + i
      const col = RUNNER_X + c
      if (text[c] !== ' ' && row >= 0 && row < CANVAS_H && col >= 0 && col < CANVAS_W) solid[row]![col] = true
    }
  })
  for (const [row, col] of legs) {
    if (row >= 0 && row < CANVAS_H && col >= 0 && col < CANVAS_W) solid[row]![col] = true
  }
  // The sheets in the air, drawn behind the runner: a sheet cell is skipped where he is.
  for (const [row, col, state] of sheetsAt(index)) {
    SHEET_STATES[state]!.forEach((line, i) => {
      for (let c = 0; c < line.length; c++) {
        const r = row + i
        const x = col + c
        if (line[c] === ' ' || r < 0 || r >= CANVAS_H || x < 0 || x >= CANVAS_W || solid[r]![x]) continue
        canvas[r]![x] = line[c]!
      }
    })
  }
  RUNNER_TOP.forEach((text, i) => put(top + i, RUNNER_X, text))
  for (const [row, col, ch] of legs) put(row, col, ch)
  // Every row keeps its full width, so no row shrinks and the block keeps its shape.
  return canvas.map(row => row.join(''))
}
