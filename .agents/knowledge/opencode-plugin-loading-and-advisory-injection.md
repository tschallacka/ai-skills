<!-- MODE: DEV -->
# Can an opencode plugin ship as a local file, and can a hook add text a model will read?

Measured 2026-09-10, opencode 1.18.29. **Yes to both.** `plugin` in
`opencode.jsonc` accepts a local file path, not only an npm package name, and
`tool.execute.after` can append text straight into a tool's own result — the
same text the model reads back — with no separate side-channel needed.

## How it was measured

A scratch directory with its own `opencode.jsonc`:

```jsonc
{ "plugin": ["./local-plugin.js"] }
```

`opencode debug config` in that directory printed a `plugin_origins` entry:

```json
{ "spec": "./local-plugin.js", "source": ".../opencode.jsonc", "scope": "local" }
```

with the resolved import specifier surfaced elsewhere in the same output as
`file:///.../local-plugin.js` — a relative path is accepted and resolved
against the config file's own directory, no `node_modules` involved. The one
real installed example on this machine (`opencode-goal-plugin`) had suggested
only the bare-package-name form (`"plugin": ["opencode-goal-plugin"]`,
resolved via `node_modules`); this was the open question the local-path test
settled.

Second, a plugin exporting:

```js
"tool.execute.after": async (input, output) => {
  if (input.tool !== "bash") return
  if (/\bmc\b/.test(input.args.command)) {
    output.output += "\n\n[tui-hint] mc has a shipped app profile."
  }
}
```

run via `opencode run "run: mc --version" --print-logs --log-level INFO`. The
appended line showed up in the actual transcript directly after `mc`'s real
stdout, inside the same tool-result block — confirmed by the model's own next
turn, which was a plain acknowledgement of "mc --version", i.e. it read the
appended text as part of the tool's output.

`input.tool` for a shell call is the literal string `"bash"`; `input.args`
carries `{command, ...}` — see `@opencode-ai/plugin`'s `dist/index.d.ts`
(`Hooks["tool.execute.after"]`) for the full input/output shape, and
[[agent-identity-across-harnesses]] for `tool.execute.before` and `shell.env`,
which this entry does not repeat.

## What it means here

`tui-hint-plugin`'s opencode variant ships as a single `.js` file
(`tui-hint-plugin/opencode/tui-hint-plugin.js`) with no npm packaging or
publish step: the installer only needs to place the file somewhere stable and
add its absolute path to the target's `opencode.jsonc` `plugin` array — the
same "edit one config file's array" shape B235 already solved for codex's
`config.toml`, just JSON instead of TOML.

opencode has no `additionalContext`-equivalent side channel for a *completed*
tool call (unlike Claude Code's PreToolUse `additionalContext`, which rides
alongside the call rather than inside its result). `tool.execute.after`
mutating `output.output` is the only delivery mechanism verified so far for
"annotate a finished bash call for the model to read" on opencode.
