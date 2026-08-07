# Goal: Button-chain contract

## Current state and prior-goal handoffs

§ 2.1
No future HTML implementation exists or may be created during this proof. The source brief requires one initial button, one appended button per current-last-button activation, and a fourth-generated-button completion state; the comparable 1.4.0 plan names button-chain.html, #button-chain, and appendButtonChain() as the bounded future targets.

## Outcome and definition of done

§ 3.1
Define the future HTML structure and exact append/completion behavior as independently reviewable work units, with contract proof before browser execution.

## Why this goal is needed

§ 4.1
This goal establishes the exact future UI contract before any executor edits the HTML. It prevents ambiguity about the initial control, current-last-button ownership, generated-button counting, document clearing, finished text, and white-border output.

## Scope

§ 5.1
Included: the future button-chain.html #button-chain markup, appendButtonChain() behavior, and a bounded semantic contract review. Excluded: creating or editing button-chain.html, opening or serving it, browser execution, and implementation-time fixes.

## Affected files, systems, data, and interfaces

§ 6.1
Future file and symbols: button-chain.html, #button-chain, and appendButtonChain(). Planning files owned here: 01-button-chain-contract/goal.md, its progress tracker, and the three atomic step documents plus their testing companions.

## Dependencies and handoffs

§ 7.1
W01 defines the one-button markup boundary. W02 consumes W01 and defines append/completion semantics. W07 reviews W02 before handing the exact contract to W03, the future browser proof in the next goal.

## Implementation approach, risks, and edge cases

§ 8.1
Keep markup and behavior separate even though both target the same future file. Count the fourth generated button as the fourth newly appended button activation after the initial button. Each activation before completion must add exactly one button below the current last button; completion must clear the document and show finished with a visible white border. The main risks are filename convention drift and off-by-one counting; reopen review if either assumption changes.

## Owned work units

§ 9.1
W01 — future #button-chain markup; W02 — future appendButtonChain() behavior; W07 — semantic contract verification. Together they define and independently check the future button-chain outcome.

## Goal-size exception

§ 10.1
Not applicable: this goal owns three work units and satisfies the normal 2–10 work-unit goal-size limit.
