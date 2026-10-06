import type { Register } from 'claude-code'

// A toast when a turn finishes. The person sees the outcome without reading
// the transcript; the agent is not told anything, so this is not a hook in the
// sense the repository's own hook plugins are. Toggled by `enabled` in
// settings.json pluginConfigs["turn-cue"].options.
export const register: Register = (on, options) => {
  if (options.enabled === false) return

  on('turn.complete', async ($, e, next) => {
    const seconds = Math.round(e.durationMs / 1000)
    const outcome = e.isAborted ? 'stopped' : 'done'
    $.ui.toast(`Turn ${outcome} in ${seconds} s`)

    return next(e)
  })
}
