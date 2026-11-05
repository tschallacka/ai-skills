"""Builds a side-by-side report across the wording variants
run-wording-sweep.sh produces under one sweep directory -- one
treatment-only case per *.txt file under wordings/, grouped by which lever
(mcp, edit, read) it rewords. Unlike compare-hook-matrix.py (which asks
whether a lever helps at all), this asks which PHRASING of an
already-on lever moves adoption furthest.

Usage: compare-wording-sweep.py <sweep-dir>
Writes <sweep-dir>/wording-sweep-comparison.md and prints it to stdout.
"""

import json
import sys
from pathlib import Path

LEVER_ORDER = ["mcp", "edit", "read", "bash"]


def load_json(path):
    try:
        with open(path, encoding="utf-8") as handle:
            return json.load(handle)
    except (OSError, json.JSONDecodeError):
        return {}


def load_text(path):
    try:
        with open(path, encoding="utf-8") as handle:
            return handle.read()
    except OSError:
        return ""


def case_report(case_dir, wordings_dir, variant_name):
    telemetry = load_json(case_dir / "telemetry.json")
    score = load_json(case_dir / "score.json")
    timeline = load_json(case_dir / "edit-timeline.json")

    wall_seconds = None
    eval_text = load_text(case_dir / "evaluation.md")
    for line in eval_text.splitlines():
        if line.startswith("- Wall clock:"):
            wall_seconds = line.split(":", 1)[1].strip()

    adoption = telemetry.get("ai_text_editor_adoption", {})
    mentions = telemetry.get("ai_text_editor_text_mentions", {})
    wording_text = load_text(wordings_dir / f"{variant_name}.txt").strip()

    return {
        "wording_text": wording_text,
        "wall_seconds": wall_seconds,
        "adopted": adoption.get("adopted", "n/a"),
        "ai_text_editor_calls": adoption.get("total_calls", "n/a"),
        "first_call_index": adoption.get("first_call_sequence_index", "n/a"),
        "text_mentions": mentions.get("count", "n/a"),
        "mention_snippets": mentions.get("snippets", []),
        "internal_edit_count": timeline.get("internal_edit_count", "n/a"),
        "mcp_edit_count": timeline.get("mcp_edit_count", "n/a"),
        "mcp_resolve_count": timeline.get("mcp_resolve_count", "n/a"),
        "mechanism_switch_count": timeline.get("mechanism_switch_count", "n/a"),
        "tax_rate_preserved": score.get("tax_rate_external_edit_preserved", "n/a"),
        "reformat_preserved": score.get("external_reformat_preserved", "n/a"),
        "outcome": score.get("outcome", "n/a"),
        "asked_for_feedback": score.get("asked_for_feedback", "n/a"),
        "feedback_phrase_matched": score.get("feedback_phrase_matched"),
        "worker_final_summary": score.get("worker_final_summary"),
    }


def main():
    if len(sys.argv) != 2:
        print("Usage: compare-wording-sweep.py <sweep-dir>", file=sys.stderr)
        return 64

    sweep_dir = Path(sys.argv[1])
    framework_dir = Path(__file__).resolve().parent
    wordings_dir = framework_dir / "wordings"

    available = {
        p.name: p
        for p in sweep_dir.iterdir()
        if p.is_dir() and (p / "treatment").is_dir()
    }
    if not available:
        print(f"compare-wording-sweep.py: no wording-variant case dirs found under {sweep_dir}", file=sys.stderr)
        return 1

    by_lever = {lever: [] for lever in LEVER_ORDER}
    for name in sorted(available):
        lever = name.split("-", 1)[0]
        by_lever.setdefault(lever, []).append(name)

    lines = [
        f"# ai-text-editor-usage wording sweep: {sweep_dir.name}",
        "",
        "Within each lever, every row has the SAME lever on (the other two",
        "off) and task-prompt.md byte-identical across every row -- only the",
        "lever's own wording changes. This answers which phrasing of an",
        "already-effective lever moves unprompted adoption furthest, not",
        "whether the lever helps at all (run-hook-matrix-sweep.sh answers that).",
        "",
    ]

    reports = {}
    for lever in LEVER_ORDER:
        names = by_lever.get(lever, [])
        if not names:
            continue
        lines.append(f"## {lever} wordings")
        lines.append("")
        lines.append("| variant | adopted | internal edits | mcp edits | resolves | switches | outcome | wall |")
        lines.append("|---|---|---|---|---|---|---|---|")
        for name in names:
            report = case_report(available[name] / "treatment", wordings_dir, name)
            reports[name] = report
            lines.append(
                "| {name} | {adopted} | {internal} | {mcp} | {resolves} | {switches} | {outcome} | {wall} |".format(
                    name=name,
                    adopted=report["adopted"],
                    internal=report["internal_edit_count"],
                    mcp=report["mcp_edit_count"],
                    resolves=report["mcp_resolve_count"],
                    switches=report["mechanism_switch_count"],
                    outcome=report["outcome"],
                    wall=report["wall_seconds"],
                )
            )
        lines.append("")

    lines += ["## Wording text, per variant", ""]
    for lever in LEVER_ORDER:
        for name in by_lever.get(lever, []):
            report = reports[name]
            lines.append(f"### {name}")
            lines.append("```")
            lines.append(report["wording_text"] or "(missing)")
            lines.append("```")
            lines.append("")

    lines += ["## Worker's own final summary, per variant", ""]
    for lever in LEVER_ORDER:
        for name in by_lever.get(lever, []):
            report = reports[name]
            lines.append(f"### {name}")
            if report["asked_for_feedback"]:
                lines.append(f"**Asked for feedback** -- matched phrase: `{report['feedback_phrase_matched']}`")
            lines.append("```")
            lines.append(report["worker_final_summary"] or "(no result text)")
            lines.append("```")
            lines.append("")

    lines += ["## Mentions of ai-text-editor in reasoning, per variant", ""]
    for lever in LEVER_ORDER:
        for name in by_lever.get(lever, []):
            report = reports[name]
            lines.append(f"### {name}")
            lines.append("```")
            lines.append("\n---\n".join(report["mention_snippets"]) or "(none)")
            lines.append("```")
            lines.append("")

    report_text = "\n".join(lines) + "\n"
    (sweep_dir / "wording-sweep-comparison.md").write_text(report_text, encoding="utf-8")
    print(report_text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
