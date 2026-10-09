// The stream the board shows, by name, once picked from the streams list
// (null: no stream picked yet -- the empty-state render), and whether the
// streams list is open.
declare module 'claude-code' {
  interface PluginState {
    'tailpipe-board': { stream: string | null; listing: boolean }
  }
}
