# Goal: A control-surface look, defined as testable properties

## Current state and prior-goal handoffs

§ 2.1
Depends on goal 02 for the surfaces it styles. The user asked for a graphically striking result that feels like a live operations display when work is happening. Confirmed: the current page has no shared token set, so a colour can be introduced ad hoc.

## Outcome and definition of done

§ 3.1
One design system the pages share: tokens for palette, luminance and type scale; a named depth scale for layered translucent panels; a motion system with declared durations and easings that honours reduced motion; animated page transitions; and an ambient activity indication that reads across a room. Demonstrated by every page drawing only on the tokens, by a measured frame budget during graph growth on the largest fixture, and by a contrast and meaning-redundancy check that no accent or glow is the sole carrier of information.

## Why this goal is needed

§ 4.1
The user rejected the current page as visually unnavigable. A shared token set, a depth scale and a motion system are what make twelve pages read as one instrument rather than twelve documents, and the activity indication is what makes an active plan legible from a distance.

## Scope

§ 5.1
In scope: the token set, the depth scale for layered panels, the motion system with declared durations and easings, the page transition, the activity indication, and the frame-budget and contrast verifications. Out of scope: choosing a tier or measuring frame rate at runtime, which goal 12 owns.

## Affected files, systems, data, and interfaces

§ 6.1
The style layer shared by every page: the token scope, the panel depth classes, the motion tokens, the transition renderer and the activity indicator. Every page draws from these rather than declaring values of its own.

## Dependencies and handoffs

§ 7.1
Depends on goal 02. Hands to goal 12 the named effects a tier can disable, and hands to goal 09 the contrast and frame-budget records.

## Implementation approach, risks, and edge cases

§ 8.1
Approach: every value resolves to a declared token, so an ad-hoc colour is a detectable defect rather than a matter of taste, and neutrals carry a slight hue bias toward the accent rather than being pure grey. Risk: colour, glow or motion becomes the sole carrier of information, which fails for a colour-blind reader, a monochrome display or reduced motion, so the redundancy check requires a text or shape carrier alongside every one of them. Edge case: with reduced motion requested every animated state must still be legible with the motion removed, and the activity indication must be absent when nothing is active rather than idling.

## Owned work units

§ 9.1
`W64` — Define the palette, luminance steps, spacing and type scale as tokens on the root, so every page and panel draws from one place and a colour cannot be introduced ad hoc. Neutrals carry a slight hue bias toward the accent rather than being pure grey.

§ 9.2
`W65` — A named depth scale for layered translucent panels over the dark ground: background blur, border luminance and shadow per level, with a stated maximum so layering cannot become soup. Content contrast is computed against the composited background, not the token.

§ 9.3
`W66` — Declared durations and easings as tokens with a stated purpose each, so a transition cannot be hand-tuned per element. A single reduced-motion block neutralises duration and transform while leaving every state change applied.

§ 9.4
`W67` — Emit the markup and classes that let a route change animate as a transition rather than a cut, including the direction of travel so moving deeper and moving back are distinguishable.

§ 9.5
`W68` — Indicate that the page is live and something is happening, readable at a glance from a distance and without reading a number: a pulse tied to real state arrival, quiescent when nothing is arriving, and never a decorative animation that implies activity that is not occurring.

§ 9.6
`W69` — Measure the frame budget while the graph animates growth on the 337 KB fixture, and record both the numbers and the cinematic tier the page settled at. A measured budget miss at the full tier is expected on a slow machine and is evidence the degradation works, not a failure; a miss that does not step the tier down is a finding.

§ 9.7
`W70` — Check every token pair used for text against its composited background for contrast, and confirm no status, anomaly or activity is conveyed by colour, glow or motion alone. Record the measured ratios.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The aesthetic is only acceptable if it is measured: contrast against composited backgrounds and frame times are recorded as numbers, and effect-only meaning is checked. |

## Goal-size exception
