// The person's own priority/branch filters and Open/Pending/Implemented view
// toggle for the pane, kept across redraws the same way ci-board keeps its
// own browsing state: null means "unset, default to open" for `view`,
// the same way null/false means "no filter" for the others -- the pane
// always has a definite state to draw from. `register-board`'s own
// types/index.d.ts has nothing to augment here (it keeps no `$.state` at
// all); this mod now does.
declare module 'claude-code' {
  interface PluginState {
    'decision-board': {
      priorityFilter: string | null
      branchOnly: boolean
      view: 'open' | 'pending' | 'implemented' | null
      lastLogged: { id: string; at: number } | null
    }
  }
  interface McpToolInputs {
    'mcp__decision-board__answer_decision_board': { id: string; letter: string }
    'mcp__decision-board__implement_decision_board': { id: string; note?: string }
  }
}
