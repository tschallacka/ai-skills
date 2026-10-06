// The plan the board shows, as the person picked it from the selector. Null
// means no pick yet: the board falls back to the `plan` option, then to the
// plan changed most recently. `browsing` is whether the plan list is open under
// the `[browse plans]` button. `drill` is how far the person has drilled into
// the chosen plan: a goal, then one of its steps.
export type PlanPick = { dir: string; scope: 'project' | 'global' } | null
export type Drill = { goal: string | null; step: string | null }

declare module 'claude-code' {
  interface PluginState {
    'plan-board': {
      selection: PlanPick
      browsing: boolean
      drill: Drill
    }  }
}
