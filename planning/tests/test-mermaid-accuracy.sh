#!/usr/bin/env bash
# MODE: DEV
# test-mermaid-accuracy — the ARCHITECTURE mermaid diagrams may not name a
# script, artifact, node id or function that does not exist.
#
# Usage: test-mermaid-accuracy.sh
#
# Four mechanical checks, and each one's limit:
#   1. Structure, in awk: balanced quotes and brackets, a recognised header, at
#      least one statement, every referenced node id defined. Only the shape awk
#      can see. Mermaid's own verdict comes from `mmdc`, a dev-flake tool and not
#      a suite dependency, so that portion reports UNCONFIGURED when it is
#      absent. Rendering proves syntax, never accuracy.
#   2. Every `*.sh` named in a block exists, or a redirect in some script writes
#      it (`benchmark-env.sh` is generated).
#   3. Every artifact filename named in a block is a real path or is named by
#      some script. "Written by" is not decidable here — plan paths are composed
#      from variables — so a name only prose knows is a WARN, not a FAIL.
#   4. Every backticked lower_snake identifier resolves: a `name()` definition
#      where the document writes it as a function, else a mention in a script.
#
# Not checkable, hence CODE-STYLE.md §11's rule for the author: whether an arrow
# is the real control flow, a diamond the true condition, a stage order the code.
set -euo pipefail

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism. Placed BEFORE this script's own source of
# lib-test.sh and its t_begin call, so a found compiled binary bypasses the
# bash body -- including t_begin's own dirty-tree precondition and ERR trap --
# entirely, matching every prior conversion's own placement rule. This script
# lives under planning/tests/, two levels below the repository root (the same
# relative shape .github/ci-subjects.sh's own wiring already handles for a
# caller outside planning/scripts/): plan-core-lib.sh is a generated,
# gitignored file that does not exist on a genuinely fresh checkout, so the
# source+exec is guarded on it already being present, unconditionally falling
# through to this script's own bash implementation when it is not. Already
# declares -euo pipefail above and wants to keep it, so no option is forced
# back afterward (unlike ci-subjects.sh's own deliberate -e-free case).
tma_repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
if [ -f "$tma_repo_root/planning/scripts/plan-core-lib.sh" ]; then
    source "$tma_repo_root/planning/scripts/plan-core-lib.sh"
    plan_exec_compiled_binary_if_present test-mermaid-accuracy "$tma_repo_root" "$@"
fi

printf '%s: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it\n' "test-mermaid-accuracy" >&2
exit 69
