// tui-hint-plugin for opencode.
//
// Mirrors the Claude Code PreToolUse hook in ../hooks/pre-tool-use.sh (and
// shares its matching logic in ../hooks/lib.sh): when a bash tool call's
// target program has a shipped interactive-shell app profile
// (${XDG_CONFIG_HOME:-~/.config}/tsch-ai-skills/appprofiles/<name>.md) or an
// agent-written one in its sibling appprofiles.d/ (marker-gated, see
// hasMarker below and interactive-shell/appprofiles/FORMAT.md), append a
// short advisory to the tool's own output text. opencode has no
// additionalContext-style side channel for a completed tool call, so the
// advisory rides in output.output -- the same text the model reads back as
// the tool's result.
//
// Uses tool.execute.after (not .before): the advisory names the program that
// already ran, and appending to the result never risks altering args a
// .before hook would have to leave untouched anyway.
//
// Which profile a command line invokes is read from each profile's own
// ### Invocation patterns (FORMAT.md), not hardcoded here -- a profile whose
// filename is not simply its leading word (a git subcommand, e.g.) declares
// its own matching patterns and this plugin picks them up with no code
// change.
import { existsSync, readdirSync, readFileSync } from "node:fs"
import { join } from "node:path"
import { homedir } from "node:os"

const MARKER = "<!-- tui-app-profile: v1 -->"

function configRoot() {
  const base = process.env.XDG_CONFIG_HOME || join(homedir(), ".config")
  return join(base, "tsch-ai-skills")
}

// The program a shell command line runs is not always its first word: a
// leading VAR=value assignment, or a leading sudo/env wrapper, is common and
// would otherwise hide the real target from a naive match. Word-split on
// purpose (not quote-aware) -- this is advisory only, never blocking, so a
// wrong guess on an unusual quoting shape costs a missed or spurious
// reminder, not a wrong action. Returns the REST of the line, tokens
// rejoined, for matching against multi-word ### Invocation patterns.
function strippedCommand(commandLine) {
  const tokens = commandLine.trim().split(/\s+/).filter(Boolean)
  let i = 0
  while (i < tokens.length) {
    const token = tokens[i]
    if (token.includes("=") && !token.startsWith("-")) {
      i += 1
      continue
    }
    if (token === "sudo") {
      i += 1
      while (i < tokens.length) {
        if (tokens[i] === "-u" || tokens[i] === "--user") {
          i += 2
          continue
        }
        if (tokens[i].startsWith("-")) {
          i += 1
          continue
        }
        break
      }
      continue
    }
    if (token === "env") {
      i += 1
      while (i < tokens.length) {
        if (tokens[i].startsWith("-")) {
          i += 1
          continue
        }
        if (tokens[i].includes("=")) {
          i += 1
          continue
        }
        break
      }
      continue
    }
    break
  }
  return tokens.slice(i).join(" ")
}

function firstWord(command) {
  const first = command.split(/\s+/, 1)[0] || ""
  return first.split("/").pop()
}

// One extended-regex pattern per line, from a profile's own ### Invocation
// section (FORMAT.md); empty array when the file has none. The section ends
// at the next "### " heading or end of file.
function invocationPatterns(file) {
  let text
  try {
    text = readFileSync(file, "utf8")
  } catch {
    return []
  }
  const lines = text.split(/\r?\n/)
  const patterns = []
  let inSection = false
  for (const line of lines) {
    if (/^### Invocation\s*$/.test(line)) {
      inSection = true
      continue
    }
    if (/^### /.test(line)) {
      inSection = false
      continue
    }
    if (inSection && line.trim()) patterns.push(line.trim())
  }
  return patterns
}

function hasMarker(file) {
  let text
  try {
    text = readFileSync(file, "utf8")
  } catch {
    return false
  }
  return text.split(/\r?\n/, 1)[0] === MARKER
}

function testPatterns(patterns, command) {
  return patterns.some((pattern) => {
    try {
      return new RegExp(pattern).test(command)
    } catch {
      return false
    }
  })
}

// Finds the profile (basename, without .md) that a stripped command line
// invokes, searching one directory. When requireMarker is true, a candidate
// file must carry the literal marker line (appprofiles.d/ is agent-writable
// and so not self-trusting the way the vendor directory is); when false,
// the directory itself is the trust boundary and no marker is required.
function matchProfile(dir, command, requireMarker) {
  if (!existsSync(dir)) return undefined

  const word = firstWord(command)
  if (word && word !== "FORMAT") {
    const candidate = join(dir, `${word}.md`)
    if (existsSync(candidate)) {
      const trusted = !requireMarker || hasMarker(candidate)
      if (trusted) {
        const patterns = invocationPatterns(candidate)
        if (patterns.length === 0) return word
        if (testPatterns(patterns, command)) return word
      }
    }
  }

  let entries
  try {
    entries = readdirSync(dir)
  } catch {
    return undefined
  }
  for (const entry of entries) {
    if (!entry.endsWith(".md")) continue
    const base = entry.slice(0, -3)
    if (base === "FORMAT") continue
    const file = join(dir, entry)
    if (requireMarker && !hasMarker(file)) continue
    const patterns = invocationPatterns(file)
    if (patterns.length === 0) continue
    if (testPatterns(patterns, command)) return base
  }
  return undefined
}

export const TuiHintPlugin = async () => {
  return {
    "tool.execute.after": async (input, output) => {
      if (input.tool !== "bash") return
      const command = input.args && input.args.command
      if (!command) return

      const stripped = strippedCommand(command)
      if (!stripped) return

      const root = configRoot()
      const vendorDir = join(root, "appprofiles")
      const agentDir = join(root, "appprofiles.d")

      let program = matchProfile(vendorDir, stripped, false)
      let profile = program ? join(vendorDir, `${program}.md`) : undefined
      if (!profile) {
        program = matchProfile(agentDir, stripped, true)
        profile = program ? join(agentDir, `${program}.md`) : undefined
      }
      if (!profile) return

      output.output +=
        `\n\n[tui-hint] ${program} has a shipped interactive-shell app profile at ` +
        `${profile} -- screen layout, keybindings, dialogs, and known quirks. ` +
        `A plain bash call cannot observe its screen or send it real keystrokes; ` +
        `consider driving it through the interactive-shell skill instead, especially ` +
        `for anything beyond a one-shot non-interactive invocation.`
    },
  }
}
