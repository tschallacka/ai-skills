# Goal: Hash-routed page shell that works from disk and served

## Current state and prior-goal handoffs

§ 2.1
Depends on goal 01 for the derived state. Confirmed by browser discovery on the current renderer: it has zero anchors, so no page can be linked to or returned to, and content clips at column edges. Those two observations are what this goal exists to fix.

## Outcome and definition of done

§ 3.1
One artifact containing every page, routed by URL hash, with breadcrumbs, a back-stack, and an inline peek that expands a related item without losing position. Demonstrated by opening the same file from disk and over HTTP, deep-linking directly to a unit page, walking back through the stack, and peeking a related unit without navigation.

## Why this goal is needed

§ 4.1
The user asked for pages that flow into each other and for the tool to serve both review and monitoring. That requires an addressable page model with breadcrumbs, a back-stack and a peek; without it every later page is a section of one unnavigable document, which is what the current renderer produces.

## Scope

§ 5.1
In scope: the document skeleton, hash routing, breadcrumbs, back-stack, inline peek, and deriving which mode the plan is in so the leading surface differs. Out of scope: the content of any individual page, which goals 04 to 07 own, and live updates, which goal 10 owns.

## Affected files, systems, data, and interfaces

§ 6.1
The shell module of the Rust crate and the client script it embeds. It emits one artifact carrying every page and the embedded state, and it defines the routes that all later pages register into.

## Dependencies and handoffs

§ 7.1
Depends on goal 01. Hands to goals 04 to 08 the route registration and the shell chrome every page draws inside, and hands to goal 11 the surfaces the depth scale and transitions apply to.

## Implementation approach, risks, and edge cases

§ 8.1
Approach: one preallocated output buffer with values written as slices, so no per-key template rebuild and no temporary string per substitution. Risk: a route that resolves to nothing leaves a blank page, so an unresolved hash lands on the overview and states the hash it could not resolve. Edge case: a deep link with no history must still offer a sensible back destination, and mode derivation must report ambiguity rather than guess when a plan is approved with no started steps.

## Owned work units

§ 9.1
`W07` — Emit the document skeleton through a named production RenderBuffer whose capacity is fixed before rendering. W117 instruments this buffer boundary; the renderer has no runtime dependency on the old interpreter stack.

§ 9.2
`W08` — Map a URL hash to one page and its parameters: overview, goal, unit, finding, test, coverage, history, graph. An unknown hash resolves to the overview with a stated reason rather than a blank page.

§ 9.3
`W09` — Render the breadcrumb trail for the routed page from the plan hierarchy, so a deep link shows where it sits without requiring the reader to have walked there.

§ 9.4
`W10` — Maintain a back-stack across hash navigation so browser back and an in-page back control both return to the previous page rather than the previous scroll position.

§ 9.5
`W11` — Expand a related item inline from any link without changing the route, so a reader can check a dependency without losing position. Escape closes it and focus returns to the invoking link.

§ 9.6
`W12` — Pin every route and its parameters, including the unknown-hash fallback. Fault-inject by removing a route arm and by feeding a malformed hash.

§ 9.7
`W13` — Open the same artifact from the filesystem and over HTTP, deep-link straight to a unit page, walk back through the stack, and open a peek without navigating. Record the interaction for each.

§ 9.8
`W50` — Derive the lifecycle mode from the review status and step statuses: planning while the review is pending or no step has started, implementing once it is approved and work has begun, and complete when every step and its verification have passed. An ambiguous combination is reported as ambiguous rather than guessed.

§ 9.9
`W51` — Select which surface leads for the derived mode and state the mode on the page: planning leads with soundness, implementing leads with execution, complete leads with the outcome. A surface irrelevant to the current mode is reachable but not primary, never silently absent.

§ 9.10
`W52` — Test lifecycle derivation including the exact approved-with-zero-steps empty-approved fixture as ambiguous with the contradiction named while its page remains readable.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | Routing, breadcrumbs, the back-stack, peek and mode selection are all observable behaviour a reader depends on; the router and mode derivation are unit-pinned and the shell is verified in the browser. |

## Goal-size exception
