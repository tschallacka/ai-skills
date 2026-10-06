// The profile the board shows, by its file name without `.md`. Null means the list.
declare module 'claude-code' {
  interface PluginState {
    'tui-hint-board': { profile: string | null; filter: string }
  }
}
