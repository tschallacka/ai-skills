// register-board and ci-board's own types/index.d.ts augment PluginState for
// the `$.state` they keep across redraws; this mod keeps none (it re-reads
// DECISIONS.json on every draw, the same stateless shape register-board's
// own read side uses), so there is nothing to declare there. What this mod
// does add is one typed MCP tool input -- the doc-blessed way to narrow
// `e.id`/`e.letter` in the answer_decision_board handler from `unknown` to
// `string`, instead of every argument staying loose (McpToolCallInputFallback).
declare module 'claude-code' {
  interface McpToolInputs {
    'mcp__decision-board__answer_decision_board': { id: string; letter: string }
  }
}
