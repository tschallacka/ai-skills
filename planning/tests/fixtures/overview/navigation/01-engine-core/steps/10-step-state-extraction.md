# Step: 10-step-state-extraction

## Ownership

- Goal: `01-engine-core`
- Work unit: `W102`
- Type: `source`

## Change target

- File: `src/plan-overview/src/plan/extract.rs`
- Primary symbol or file scope: `extract_state()`
- Subscope: `N/A`

## Objective

§ 4.1
Extract the canonical typed state from the plan tree and serialize that exact state for the parser contract; never invoke the old shell extractor at runtime.

## Instructions

§ 5.1
Implement extraction in src/plan-overview/src/plan/extract.rs from the plan tree W01 reads, without invoking overview-state.sh. Serialize the resulting canonical typed state through the same representation W02 parses, reproduce the field contract field for field, and record any field that cannot be reproduced rather than approximating it. Leave overview-state.sh for its other consumers; the binary does not depend on it.

## Acceptance criteria

§ 6.1
The production extractor serializes a state document that W02 parses successfully, and the parsed field set and values match the extractor's canonical typed state field by field on the fixtures. On a host with no Bash and no jq the binary still produces state. Any field not reproduced is recorded by name rather than silently absent.

## Handoff

§ 7.1
W03 and W50 derive from this unit's output, so the counts, the geometry and the lifecycle mode all flow out of the path plan 5.2 declares is the runtime one. W02 depends on it too, because the document W02 parses is the one this unit produces. Adversarial findings AR-03 and AR-18 recorded the earlier arrangement, in which W02 depended on W01 alongside this unit as a sibling and the whole derive chain hung off W02, so every derived number flowed out of the path the plan said was not the runtime path.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
