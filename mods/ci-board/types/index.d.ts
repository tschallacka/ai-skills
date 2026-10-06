// The last CI report the board drew: the tool's text, or null before the first
// refresh.
declare module 'claude-code' {
  interface PluginState {
    'ci-board': {
      report: string | null
      browsing: boolean
      runs: { id: string; title: string; branch: string; conclusion: string; when: string; age: string }[] | null
      run: { id: string; title: string; text: string; url: string; took: string; conclusion: string } | null
      narrow: string | null
      busy: string | null
      progress: { done: number; total: number } | null
    }  }
}
