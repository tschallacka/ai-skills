# Step: browser acceptance for button-chain

## Owning goal and objective

Goal `01-button-chain`; verify the future file's visible behavior from a clean
browser load.

## Files or areas

Read-only browser interaction with the future root-level `button-chain.html`.
Do not alter the implementation while testing.

## Executable verification flow

1. Open `button-chain.html` from a clean page.
2. Confirm exactly one button is visible.
3. Click the current last button four times, recording the DOM after each
   click. Before the terminal click, expect 2, 3, 4, then 5 total buttons;
   generated counts are 1, 2, 3, then 4.
4. Confirm the fourth generated button click clears all buttons.
5. Confirm the only completion text is exact lowercase `finished` and its
   rendered element has a visible white border.
6. If possible, click or inspect the former location after completion and
   confirm no new button appears.

## Acceptance criteria and handoff

Pass only when every count and terminal presentation matches. Record the
sequence, URL/path, browser result, and any screenshot or DOM evidence in
`ui-story-runs/button-chain.md`; hand the result to the reviewer.
