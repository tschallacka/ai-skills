<!-- MODE: PROD -->
# Bug report

**Every defect, one file, full story.**

When a bug is found but not fixed in the same breath, it goes here: `BUGS.json`
— one register read with rjq, where each entry carries its reproduction, the
measurement that proves it real, the mechanism once known, and the
verification that fails without the fix.

## What you get

- **Reproduction first.** A report without one is a rumour; the register
  refuses to accept it.
- **Observed vs expected.** The two sentences that define a defect, side by
  side, forever.
- **Closures with proof.** Marking a bug `fixed` requires the fix and *how it
  was verified* — including the mutation check. No silent healing.
- **One binary, not hand-editing.** `bugs add` / `bugs update` write
  through shared validation, so a malformed entry cannot land. Refusals name
  their reason; a rejected write leaves the register exactly as it was.

## Quick start

> File a bug: checkout fails with exit 73 when the target dir is a symlink.
> Repro: `install.sh --target ~/current` twice.
> Close B31 as fixed — fix abc123, verified by the macOS CI leg going green.

## Good to know

This is for defects. Work that is merely queued belongs to the todo skill; a
design preference is neither — it belongs in a decision, not a register.
