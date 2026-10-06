// The channel the board shows, by name, once the person has picked one from the channel
// list (null: the channel in the `channel` option), and whether the channel list is open.
declare module 'claude-code' {
  interface PluginState {
    'chat-board': { channel: string | null; listing: boolean; extra: number }
  }
}
