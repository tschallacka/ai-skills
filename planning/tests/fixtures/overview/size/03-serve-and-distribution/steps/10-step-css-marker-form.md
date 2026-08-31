# Step: 10-step-css-marker-form

## Ownership

- Goal: `03-serve-and-distribution`
- Work unit: `W109`
- Type: `test`

## Change target

- File: `tests/test-mode-markers.sh`
- Primary symbol or file scope: `marker_of()`
- Subscope: `N/A`

## Objective

§ 4.1
Read a CSS block-comment marker form so a stylesheet can declare its audience. The reader accepts a hash comment, an HTML comment and a double-slash comment; none is valid CSS, so the three stylesheets the crate embeds can hold no readable marker and would fail the gate with no way to pass it.

## Instructions

§ 5.1
In marker_of() in tests/test-mode-markers.sh, add a CSS block-comment form alongside the three it already reads, so a marker written as a slash-star comment on the file's first lines is recognised. Read it under the same head-of-file window as the others rather than widening the window for one kind. Change marker_of only; the exemption arms are W76's.

## Acceptance criteria

§ 6.1
A stylesheet carrying a marker in the CSS comment form is read and classified; the same stylesheet with the marker removed still fails the gate. The three existing comment forms are unchanged, shown by the gate's result over the repository being identical before and after apart from the newly readable files. A marker placed below the head-of-file window is still not read, for CSS as for every other kind.

## Handoff

§ 7.1
W64, W65 and W66 can carry the marker their audience requires instead of needing an exemption for being stylesheets, and W108 has a gate that can classify every file under the crate rather than most of them.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
