# Testing companion: 01-step-automated-check

## Automated verification

- Target work unit: `W04`
- Future command: run a bounded `node` heredoc against `button-chain.html` after W01-W03 are implemented. The heredoc must use only `node:fs`, `node:assert/strict`, `node:vm`, and a small in-memory DOM harness declared inside the heredoc; it must not create persistent test files.
- Required assertions:
  - The initial parsed document contains exactly one button in `#button-chain-root`.
  - Clicking the current last button appends exactly one new button below it.
  - Earlier non-last buttons do not append additional buttons.
  - Four valid append clicks create four generated buttons after the initial button.
  - Clicking the fourth generated button clears prior page content.
  - The final visible text is exactly `finished`.
  - The completion element has a visible white border on a contrasting background.

## Result for this planning proof

- Status: not run.
- Evidence: this benchmark explicitly forbids creating, opening, serving, inspecting, or testing HTML during the planning-only proof.
- Completion requirement for future executor: paste the exact command, exit code, and assertion summary here before marking W04 complete.

## Command shape

```bash
node <<'NODE'
// Future executor supplies the complete bounded harness here.
// Required modules: node:fs, node:assert/strict, node:vm.
// Required input: ./button-chain.html.
// Required result: process exits 0 only when all W04 assertions pass.
NODE
```
