<!-- MODE: PROD -->
# rjq

**A jq-compatible JSON query tool, shipped as one static binary.**

Use it to read, filter and search a JSON file on a machine without `jq`. It is
the binary the planning and todo skills already use to read their JSON, shipped
as a skill of its own so an agent can use it directly.

The skill's instructions, and the filters that are verified against the binary,
are in [SKILL.md](../SKILL.md).

## Install

Installing this skill places the `rjq` binary for your platform in the shared
bin directory, `${XDG_CONFIG_HOME:-~/.config}/tsch-ai-skills/bin/`. Nothing else
is needed: the binary asks the machine for no other tool.

## Use

```bash
RJQ="${XDG_CONFIG_HOME:-$HOME/.config}/tsch-ai-skills/bin/rjq"
"$RJQ" '.tasks[] | select(.status == "open") | .id' TODO.json
```

`jq` on PATH is preferred when it is present; see SKILL.md.
