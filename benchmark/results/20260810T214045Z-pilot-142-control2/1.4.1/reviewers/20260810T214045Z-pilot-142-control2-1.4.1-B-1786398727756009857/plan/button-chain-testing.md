# Button-chain testing companion

## Purpose

This companion records the future proof strategy for `button-chain.html` at the plan level. It complements the per-step `*-testing.md` files under the goal step directories.

## Related work units

- `W01`: static review confirms exactly one initial button in `#button-chain-root`.
- `W02`: static review confirms only the current last button can append and each accepted nonterminal click appends exactly one button.
- `W03`: static review confirms `.finished-state` provides a visible white border around exact lowercase `finished`.
- `W04`: future static implementation review records pass/fail evidence.
- `W05`: future browser story `US-01` exercises five rendered-page clicks and verifies the fourth generated button triggers the terminal state.
- `W06`: current planning-proof artifact audit confirms no HTML/HTM file was created during this proof.
- `W07`: future build readiness review confirms the implementation goal is ready for verification.

## Future verification sequence

1. Complete W01-W03 in the future implementation without creating any file other than `button-chain.html`.
2. Run W07 build readiness review.
3. Run W04 static implementation review.
4. Run W05 using `ui-story-runs/US-01.md`: click the initial button, then generated buttons 1, 2, 3, and 4 through the rendered UI only.
5. Pass only when generated buttons 1-3 each append exactly one next button, generated button 4 clears the chain, and the page shows only exact lowercase `finished` with a visible white border.

## Planning-proof status

This benchmark run does not execute the future HTML verification. The only executed proof is the artifact audit: no `.html` or `.htm` files were created in the isolated benchmark workspace.
