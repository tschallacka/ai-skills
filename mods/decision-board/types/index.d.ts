// The person's own priority/branch filters for the pane, kept across
// redraws the same way ci-board keeps its own browsing state: null/false
// means "no filter", not "unset" -- the pane always has a definite filter
// state to draw from. `register-board`'s own types/index.d.ts has nothing to
// augment here (it keeps no `$.state` at all); this mod now does.
declare module 'claude-code' {
  interface PluginState {
    'decision-board': {
      priorityFilter: string | null
      branchOnly: boolean
    }
  }
  interface McpToolInputs {
    'mcp__decision-board__answer_decision_board': { id: string; letter: string }
  }
}
