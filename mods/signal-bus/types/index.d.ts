// The shared signal bus. Every board writes what the person did in it to `signals`,
// and the bus keeps which pane is open (`panes`, in open order) and which one holds
// the keyboard (`focus`). The bus reads the pane events of every plugin, so the
// boards need not report their own tabs.

export type Signal = { at: number; board: string; kind: string; detail: string }

declare module 'claude-code' {
  interface PluginState {
    'signal-bus': {
      panes: string[]
      focus: string | null
      signals: Signal[]
    }
  }
}
