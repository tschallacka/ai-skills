// The brainstorm the board shows, by its folder. Null means the list.
declare module 'claude-code' {
  interface PluginState {
    'brainstorm-board': { pick: string | null }  }
}
