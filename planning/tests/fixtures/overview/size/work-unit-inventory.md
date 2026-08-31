# Work-unit inventory: plan-overview-rebuild

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| The largest existing plan renders and serves, where it exits 126 today | W01,W02,W06,W47 | W01 and W02 remove the argument path; W06 and W47 prove it on the 337 KB fixture |

| The page is produced in one pass with no per-key template rebuild | W07,W05 | W07 writes one buffer; W05 pins the parse and derive contract behind it |

| Every stored state field is presented somewhere deliberate | W02,W39,W31 | W02 reports the parsed field set, W39 fails on an unconsumed field, W31 is the reader check |

| Relationships are the navigation, not printed text | W24,W37,W26,W40 | W24 and W37 render edges as links; W26 and W40 walk them in the browser |

| One artifact works from disk, served, and deep-linked | W07,W08,W13,W16 | W07 embeds state, W08 routes, W13 verifies all three, W16 removes the servers it replaces |

| Tests are visible as content rather than a pointer | W28,W30,W31 | W28 renders the companion procedure, W30 pins it, W31 reads it in the browser |

| Findings and coverage are fully readable with nothing clipped | W27,W29,W25,W31 | W27 and W29 render them, W25 pins field presence, W31 verifies no clipping at width |

| The plan evolution, including discards and their reasons, is visible | W32,W33,W34,W35,W36 | W32 to W34 render it, W35 pins that an entry with no reason is flagged not dropped, W36 reads it |

| Planning and implementing states are distinguished and stated | W50,W51,W52 | W50 derives, W51 selects the leading surface, W52 pins the boundaries including ambiguous |

| Autoplay follows the right subject for the mode | W41,W53,W42,W44,W45 | W41 derives active states from served state, W53 selects the subject including the complete-mode case where there is none, W44 pins both the derivation and the selection under two fault injections, W45 verifies it live |

| Concurrent active work is legible and its tabs are truthful | W43,W44,W45 | W43 renders one tab per active state and removes a departed one, W44 pins the lifecycle, W45 verifies |

| The page follows the plan directory as it changes | W58,W59,W60,W61,W62,W63 | W58 and W59 detect and coalesce, W60 publishes, W61 applies without losing position, W62 pins, W63 verifies |

| The graph reveals structure, orphans and cycles | W37,W38,W40 | W37 renders, W38 names anomalies, W40 finds an orphan through the graph |

| Graph growth animates from a stable layout | W54,W56,W57,W69 | W54 makes layout deterministic, W56 animates the difference, W57 pins stability, W69 measures the animation |

| The look is a control surface defined by tokens, depth and motion | W64,W65,W66,W67,W68 | One token source, a bounded depth scale, declared motion, animated transitions, and a truthful activity pulse |

| Cinematics degrade with measured framerate and never lose information | W71,W72,W73,W74,W70 | W71 declares the tiers, W72 measures, W73 applies and states why, W74 pins no flapping, W70 proves nothing is carried by effect alone |

| Legibility is measured, not assumed | W70,W69 | W70 records composited contrast ratios and non-colour carriers, W69 records frame times |

| No second renderer exists to disagree with the first | W15,W16,W17,W48 | W15 and W16 delete the jq renderer and the four rungs, W17 ships only what exists, W48 proves the gates still pass |

| A platform without an artifact is told, not silently broken | W18,W19,W20,W49,W104 | W18 owns the notice, W104 declines to place an artifact on a host matching no pair and reaches that notice rather than failing, W19 builds the set the pair is matched against, and W20 and W49 verify the path end to end |

| Every journey passed with real interaction | W46 | W46 works the story table; a story may only be excluded with a recorded user approval |

| Every number the pages present is derived once and traceable to its enumeration | W03,W04,W30 | W03 derives counts and W04 geometry from them; W30 asserts a rendered number matches what it counts |

| A reader can orient and return from any page, including a deep link | W09,W10,W12,W13 | W09 renders breadcrumbs, W10 the back-stack, W12 pins the routes, W13 verifies from disk and served |

| A related item can be checked without losing the reader position | W11,W61,W13 | W11 opens a peek without changing the route, W61 preserves it across a live update, W13 verifies |

| The overview, goal and unit pages each show one thing in full | W21,W22,W23,W25 | W21 to W23 render them without truncation, W25 pins required field presence |

| The binary serves the pages it renders, replacing four runtimes | W14,W16,W13 | W14 serves, W16 removes the rungs it replaces, W13 verifies the served path |

| Structure can be inspected without leaving the graph | W55,W54,W40 | W55 opens node detail beside the graph, W54 keeps layout stable, W40 verifies the orphan hunt |

| Every test the plan declares can actually fail the gate that runs it, and no shipped artifact reddens a gate it cannot satisfy | W75,W76,W48 | W75 gives the crate tests a discovery path, without which the ten declared cargo units can never redden the suite; W76 exempts the compiled artifacts from the comment-marker gate with a stated reason rather than a blanket; W48 runs both gates and records the result |

| The renderer is a crate inside this repository that needs nothing installed to run | W77,W17,W06 | W77 owns the crate manifest and pins the no-dependency property; W17 ships the built artifacts; W06 proves on the 337 KB fixture that the argument-length limit is gone |

| Nothing shipped, declared or documented names the removed renderer | W78,W79,W80,W81,W82,W83,W84 | W78 and W79 delete the template and the serve wrapper, W80 drops the runtime any-of group, W81 and W82 correct the manifest and the map as a pair, W83 corrects the agent-facing contract and W84 the reference documentation |

| The suite tests the binary rather than the deleted renderer, with no coverage lost | W85,W86,W87,W88,W89 | W85 and W86 rewrite the render and serve assertions against the binary, W87 covers the plan-dir synonym directly instead of by reference, W88 corrects the ratchet count and W89 removes the stale allowlist arm |

| Memory stays bounded on the largest plan, which is why the rewrite exists | W07,W90,W47,W91,W92 | W07 writes one preallocated buffer, W90 sets the ceiling and proves it bites under per-field concatenation, and W92 and W91 supply the size and navigation fixtures the buffer count must be identical across — W47 renders the size fixture the ceiling is stated against |

| Every state a story observes exists in a checked-in fixture, not only in a live plan | W91,W92,W93,W94,W95,W96,W97,W98,W99 | W91 and W92 freeze the two real plans, W93 to W98 carry the edge cases no real plan has, and W99 fails when a fixture stops carrying the case it exists for |

| The crate is a complete, buildable program with a decided command-line contract and one input path | W77,W100,W101,W102,W02,W03,W50 | W77 declares the crate, W100 makes every module reachable, W101 fixes the flag contract the contract and documentation quote, W102 produces the state the runtime chain uses, and W03 and W50 derive from it rather than from W02, which reads a supplied document behind the served state endpoint |

| A host receives one runnable artifact, and the compiler is a development dependency only | W103,W104,W105,W106 | W103 declares the toolchain floor, W104 places only the matching artifact, W105 decides what the package carries, W106 records six installs with the placed artifact executed |

| Every file the crate ships or builds from declares its audience, and the gate that checks it covers all of them | W107,W108,W109,W76 | W107 pins the toolchain with the marker its kind takes, W109 makes a stylesheet's marker readable at all, W76 exempts only what cannot carry one and drops the arm for the deleted runtime, W108 checks the whole crate once under two mutations |


| A missing compiler reddens the automated gate instead of being reported as unconfigured | W112,W75,W103,W107 | W112 installs the pinned toolchain in CI and makes the leg refuse rather than degrade there, W75 adds the leg, W103 and W107 name the same floor from the flake and the crate |

| The generated installer is regenerated by a unit that owns it, not as a side effect of editing its sources | W113,W80,W17,W114,W104,W18 | W113 owns install.sh as a generated-file update with the generator recorded; W17, W114, W104 and W18 are the four units that edit installer/src, and W80 changes the requires.tsv the generator also reads. W110 is deliberately absent: binaries.tsv is not a generator input and produces no hunk, which an earlier version of this row asserted the opposite of |


| One compiler floor, named in three places and proven identical | W103,W107,W112,W116 | W103 puts the floor on the existing flake declaration, W107 pins the crate, W112 installs it in CI and makes the leg refuse rather than degrade, W116 records all three resolved versions and the crate test count |

| The artifacts exist in the tree at a decided location | W115,W19,W110 | W115 commits the five artifacts under planning/bin with their sizes recorded, W19 builds them per platform, W110 declares and checksums them |

| What ships is declared once, validated against the tree, and matches what is built | W110,W111,W17,W104,W115 | W110 declares the per-target set and checksums it, W111 validates the declaration against the tree in both directions under three mutations, W115 puts the artifacts there, W17 makes them available to an install and W104 places the one that matches |

| Every tracked file the plan adds under a skill is registered by the unit that accounts for it | W114,W99,W110,W111 | W114 registers the fixture corpus in the dev arm and binaries.tsv in the prod arm, and proves the gate was not run vacuously before staging. W99 and W110 create what it registers. W111 creates the repository-root test, which is registered nowhere on purpose: the gate globs inside each skill directory, so an entry for it would name a planning/tests path that does not exist |

| The compiled binary is the sole renderer and server and has no dependency on the removed interpreter stack at runtime | W01,W14,W15,W16,W17,W18,W19,W20,W75,W103,W104,W106 | W01 and W14 own the native Rust implementation, W15 and W16 remove the old renderer and runtimes, W17 to W20 and W104 prove distribution and unsupported-host handling, W75 and W103 cover the build gates, and W106 executes the installed artifact. |

| The memory proof is registered, deterministic and fails on per-field allocation | W117,W90,W07,W47,W91,W92 | W117 owns the counting harness and production invocation; W90 records the two-fixture result and fault injection; W07 owns the single-buffer implementation and W47, W91 and W92 provide the rendered and frozen inputs. |

| The production extractor round-trips through the canonical parser without dropping emitted fields | W102,W02,W05 | W102 produces the canonical serialized state, W02 parses it, and W05 compares every emitted field and value while retaining unknown-field and malformed-input failures. |

| The installed binary is proven not to invoke the removed interpreter stack | W118,W106,W48 | W118 is the automated isolation test for render and serve, W106 exercises the installed path on supported and unsupported hosts, and W48 runs the crate test leg that can fail on an invocation. |

| Windows target selection is tested through the same installer mapping used in production | W104,W119,W106 | W104 owns the mapping, W119 owns the registered contract test, and W106 records execution on the platform matrix. |

| The npm package lifecycle is packed, extracted and executed rather than only documented | W105,W113,W120,W106 | W105 declares package contents and invocation, W113 supplies the final generated installer, W120 runs the repository test, and W106 verifies the platform records. |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W01 | source | `src/plan-overview/src/plan/tree.rs` | `read_plan_tree()` | `N/A` | Read a plan directory into memory: plan description, goals, steps, testing companions, inventory, review, coverage and history, returning one owned structure. Every path is opened as a file; nothing is passed through a process argument. | — | 01-engine-core | 01-step-read-plan-tree |

| W02 | source | `src/plan-overview/src/plan/state.rs` | `parse_state()` | `N/A` | Parse the canonical serialized state produced by W102 into typed values, reporting unknown fields and malformed input instead of dropping or partially presenting them. | W102 | 01-engine-core | 02-step-parse-state |

| W03 | source | `src/plan-overview/src/plan/derive.rs` | `derive_counts()` | `N/A` | Compute the counts and percentages the pages present: goals, steps, units, steps complete, findings total and open and resolved, resolved percentage, review depth against target, and per-goal completion. | W102 | 01-engine-core | 03-step-derive-counts |

| W04 | source | `src/plan-overview/src/plan/derive.rs` | `derive_geometry()` | `N/A` | Compute the donut offset and the three ring values from the derived counts, keeping the circumference constant in one place instead of repeated in markup. | W03 | 01-engine-core | 04-step-derive-geometry |

| W05 | test | `src/plan-overview/tests/state_parse.rs` | `parse_state_fixture` | `N/A` | Pin the production state contract: serialize W102 extraction output, parse it through W02, preserve every emitted field and value, and retain unknown-field and malformed-input failures. | W04,W102,W02 | 01-engine-core | 05-step-test-parse-and-derive |

| W06 | verification | `N/A` | `derive on the size fixture` | `N/A` | Run the binary against the 337 KB codegraph-bash-indexing-v2 state and confirm it produces complete derived values with no argument-length error, where the current renderer exits 126. Record the measured state size and the wall time. | W05,W77,W100,W101,W92 | 01-engine-core | 06-step-verify-size-fixture |

| W07 | source | `src/plan-overview/src/render/shell.rs` | `render_shell()` | `N/A` | Emit the document skeleton through a named production RenderBuffer whose capacity is fixed before rendering. W117 instruments this buffer boundary; the renderer has no runtime dependency on the old interpreter stack. | W04 | 02-page-shell | 01-step-render-shell |

| W08 | source | `src/plan-overview/src/render/router.rs` | `route()` | `N/A` | Map a URL hash to one page and its parameters: overview, goal, unit, finding, test, coverage, history, graph. An unknown hash resolves to the overview with a stated reason rather than a blank page. | W07 | 02-page-shell | 02-step-router |

| W09 | source | `src/plan-overview/src/render/chrome.rs` | `breadcrumbs()` | `N/A` | Render the breadcrumb trail for the routed page from the plan hierarchy, so a deep link shows where it sits without requiring the reader to have walked there. | W08 | 02-page-shell | 03-step-breadcrumbs |

| W10 | source | `src/plan-overview/assets/nav.js` | `backStack` | `N/A` | Maintain a back-stack across hash navigation so browser back and an in-page back control both return to the previous page rather than the previous scroll position. | W08 | 02-page-shell | 04-step-back-stack |

| W11 | source | `src/plan-overview/assets/nav.js` | `peek` | `N/A` | Expand a related item inline from any link without changing the route, so a reader can check a dependency without losing position. Escape closes it and focus returns to the invoking link. | W10 | 02-page-shell | 05-step-peek |

| W12 | test | `src/plan-overview/tests/router.rs` | `route_table` | `N/A` | Pin every route and its parameters, including the unknown-hash fallback. Fault-inject by removing a route arm and by feeding a malformed hash. | W11 | 02-page-shell | 06-step-test-router |

| W13 | verification | `N/A` | `artifact works from disk, served and deep-linked` | `N/A` | Open the same artifact from the filesystem and over HTTP, deep-link straight to a unit page, walk back through the stack, and open a peek without navigating. Record the interaction for each. | W12,W52 | 02-page-shell | 07-step-verify-shell |

| W14 | source | `src/plan-overview/src/serve.rs` | `serve()` | `N/A` | Serve the artifact and the live state from the binary on the loopback interface, printing the bound port. One implementation replaces four runtimes; no HTML is sliced with a regex. | W07 | 03-serve-and-distribution | 01-step-serve |

| W15 | source | `planning/scripts/render-plan-overview.sh` | `file removal` | `N/A` | Delete the jq renderer. It is superseded entirely: its token substitution, its argument passing and its fixed-column output are all replaced, and keeping it would leave a second implementation that can disagree. | W14 | 03-serve-and-distribution | 02-step-remove-jq-renderer |

| W16 | source | `planning/scripts/runtime` | `directory removal` | `N/A` | Delete the four server rungs and the shared handler. Their render-and-slice logic exists three times over and carries both the BrokenPipeError and the sections-endpoint regex. | W14 | 03-serve-and-distribution | 03-step-remove-runtimes |

| W17 | config | `installer/src/50-manifest.sh` | skill_files planning arm artifact entries | `N/A` | Add only the planning-arm artifact entries and remove only the deleted renderer entries from installer/src/50-manifest.sh; W114 owns the dev and prod registration arms in the same file. | W16,W115 | 03-serve-and-distribution | 04-step-manifest |

| W18 | source | `installer/src/20-runtime-tools.sh` | overview availability notice branch | `N/A` | Own only the unavailable-platform notice branch in installer/src/20-runtime-tools.sh; W104 owns artifact selection and placement in the same file. | W17 | 03-serve-and-distribution | 05-step-availability-notice |

| W19 | config | `.github/workflows/render-artifacts.yml` | `artifact matrix` | `N/A` | Build the five declared artifacts, one job per triple: linux musl x86_64 and aarch64, darwin x86_64 and aarch64, and windows msvc x86_64. Each job runs the binary once to prove the artifact executes. | W14 | 03-serve-and-distribution | 06-step-artifact-matrix |

| W20 | verification | `N/A` | `install without an artifact` | `N/A` | Install the planning skill on a host whose platform has no artifact and confirm the stated unavailability, a non-zero exit or an explicit warning, and no partially installed renderer. | W18,W15,W19 | 03-serve-and-distribution | 07-step-verify-no-artifact |

| W21 | source | `src/plan-overview/src/pages/overview.rs` | `render_overview()` | `N/A` | The monitor page: current phase, what moved since the last state, blockers, and the derived dashboard values. Every number links to the page that explains it rather than restating it. | W09 | 04-pages-primary | 01-step-overview-page |

| W22 | source | `src/plan-overview/src/pages/goal.rs` | `render_goal()` | `N/A` | One goal in full: outcome, scope, its owned units as links, its testing requirement, and its handoff. No truncation of goal names anywhere. | W21 | 04-pages-primary | 02-step-goal-page |

| W23 | source | `src/plan-overview/src/pages/unit.rs` | `render_unit()` | `N/A` | One work unit in full: change target, type, instructions, acceptance criteria, handoff, and its edges as links in both directions, dependencies and dependents. | W22 | 04-pages-primary | 03-step-unit-page |

| W24 | source | `src/plan-overview/src/pages/unit.rs` | `render_unit_edges()` | `N/A` | The edge block on a unit page: every relationship rendered as a hop, so traversal continues from the destination without returning to an index. | W23 | 04-pages-primary | 04-step-unit-edges |

| W25 | test | `src/plan-overview/tests/pages_primary.rs` | `primary_pages_render` | `N/A` | Pin that each primary page renders its required fields and that every edge target resolves to a real route. Fault-inject a dangling dependency id and a goal with no units. | W24 | 04-pages-primary | 05-step-test-primary-pages |

| W26 | verification | `N/A` | `walk three hops by edge` | `N/A` | In the browser on the 82-unit fixture, start at a unit, click to a unit it depends on, then to something depending on that, without using an index or the back button. Record each click and the resulting page. | W25 | 04-pages-primary | 06-step-verify-edge-walk |

| W27 | source | `src/plan-overview/src/pages/findings.rs` | `render_finding()` | `N/A` | One finding in full: its evidence, impact, observed contradiction, required correction, the unit it names as a link, and its status. Nothing clipped at a column edge. | W23 | 05-pages-evidence | 01-step-finding-page |

| W28 | source | `src/plan-overview/src/pages/tests.rs` | `render_test()` | `N/A` | A test or verification unit showing what it actually runs: the procedure from its testing companion, the registered command, its status, and the unit it proves. This replaces the see-companion placeholder rows. | W23 | 05-pages-evidence | 02-step-test-page |

| W29 | source | `src/plan-overview/src/pages/coverage.rs` | `render_coverage()` | `N/A` | The definition-of-done coverage mapping, each required outcome beside the units that produce and prove it, with every unit id a link and no truncated lists. | W28 | 05-pages-evidence | 03-step-coverage-page |

| W30 | test | `src/plan-overview/tests/pages_evidence.rs` | `evidence_pages_render` | `N/A` | Pin that a test page contains its companion procedure rather than a reference to it, and that coverage rows render every listed unit id. Fault-inject a companion with no automated-tests section. | W29 | 05-pages-evidence | 04-step-test-evidence-pages |

| W31 | verification | `N/A` | `read a test procedure on the page` | `N/A` | In the browser, open a test page and read its procedure without following any link off-page, then click through to the unit it proves. Confirm no see-companion text and no clipped coverage list. | W30,W27 | 05-pages-evidence | 05-step-verify-evidence |

| W32 | source | `src/plan-overview/src/pages/history.rs` | `render_history()` | `N/A` | The evolution page: status transitions with their timestamps, review cycles, and the current phase, ordered so the most recent change is first. | W21 | 06-pages-history | 01-step-history-page |

| W33 | source | `src/plan-overview/src/pages/history.rs` | `render_superseded()` | `N/A` | Superseded and resolved findings with what replaced them and the recorded reason, so a reader sees why a finding stopped being open rather than only that it did. | W32 | 06-pages-history | 02-step-superseded |

| W34 | source | `src/plan-overview/src/pages/history.rs` | `render_discarded()` | `N/A` | Discarded work: removed units, rejected alternatives from the approach decisions, and corrected paragraphs with what the earlier version said. The reason is presented beside the discard, never separately. | W33 | 06-pages-history | 03-step-discarded |

| W35 | test | `src/plan-overview/tests/pages_history.rs` | `history_pages_render` | `N/A` | Pin that a superseded finding renders its replacement and reason, and that a rejected alternative renders its rationale. Fault-inject a discard with no recorded reason and require it to be shown as missing rather than omitted. | W34 | 06-pages-history | 04-step-test-history |

| W36 | verification | `N/A` | `read one supersession and one discard` | `N/A` | In the browser, find a superseded finding and read both its replacement and its reason; then find a rejected alternative and read why it was rejected. Record both. | W35 | 06-pages-history | 05-step-verify-history |

| W37 | source | `src/plan-overview/src/pages/graph.rs` | `render_graph()` | `N/A` | Units as nodes in dependency order with status as colour and every edge a link, so ordering, orphans and cycles are visible rather than inferred from a table. | W24 | 07-graph-and-coverage-of-data | 01-step-graph-page |

| W38 | source | `src/plan-overview/src/pages/graph.rs` | `render_graph_anomalies()` | `N/A` | Name what the graph reveals: orphaned units, cycles, and verification units with no path to what they grade. Each anomaly links to the unit it concerns. | W37 | 07-graph-and-coverage-of-data | 02-step-graph-anomalies |

| W39 | test | `src/plan-overview/tests/data_coverage.rs` | `every_state_field_presented` | `N/A` | Assert that every field the state document emits is rendered on at least one page, by enumerating the parsed field set and failing on any field no page consumes. Fault-inject by adding a field and confirming the failure names it. | W38 | 07-graph-and-coverage-of-data | 03-step-test-data-coverage |

| W40 | verification | `N/A` | `find an orphan through the graph` | `N/A` | In the browser, use the graph page to locate a deliberately orphaned unit in a fixture and reach it by clicking, then follow one of its edges onward. | W39,W57,W93 | 07-graph-and-coverage-of-data | 04-step-verify-graph |

| W41 | source | `src/plan-overview/src/pages/autoplay.rs` | `active_states()` | `N/A` | Derive the set of currently active states from the served state, not from the progress documents, so autoplay follows what the agent is doing rather than what a tracker was last told. | W14 | 08-autoplay | 01-step-active-states |

| W42 | source | `src/plan-overview/assets/autoplay.js` | `autoplayFollow` | `N/A` | Navigate to the active step when autoplay is on, and keep following as the state changes. It is opt-in, its state is visible, and it never moves the page while it is off. | W41 | 08-autoplay | 02-step-autoplay-follow |

| W43 | source | `src/plan-overview/assets/autoplay.js` | `autoplayTabs` | `N/A` | Present one tab per active state when several are open, let the reader toggle between them, and follow whichever is selected. A tab vanishes as soon as its state stops being active. | W42 | 08-autoplay | 03-step-autoplay-tabs |

| W44 | test | `src/plan-overview/tests/autoplay.rs` | active_state_and_subject_selection | `N/A` | Pin the active-state derivation and the autoplay subject together: one active state yields one tab, several yield one tab each, a state leaving the active set removes its tab, and the subject the mode selects is the one autoplay follows, including the complete-mode case where there is none. Fault-inject a state with no active step, and a complete plan, and require the no-subject result rather than a stale one. | W43,W53 | 08-autoplay | 04-step-test-autoplay |

| W45 | verification | `N/A` | `autoplay follows a changing state` | `N/A` | With the binary serving, toggle autoplay on, change the served state so a different step becomes active, and confirm the page follows. Open a second active state, toggle between tabs, then end one and confirm its tab vanishes. | W44,W53 | 08-autoplay | 05-step-verify-autoplay |

| W46 | verification | `N/A` | `browser stories on the navigation fixture` | `N/A` | Every UI story in the story table has passed with real interaction on the 82-unit fixture, each with its recorded control and action. No story passes on a screenshot or a DOM read alone. | W45,W91,W93,W94,W95,W96,W97,W98 | 09-verification | 01-step-stories-pass |

| W47 | verification | `N/A` | `size fixture renders and serves` | `N/A` | The 337 KB fixture renders to an artifact and serves without error, and its pages are navigable. This is the case that exits 126 today. | W46,W92 | 09-verification | 02-step-size-fixture |

| W48 | verification | `N/A` | `repository gates on both shells` | `N/A` | Join the terminal proof after the removal fixture registration toolchain package and runtime-isolation branches complete. Run both shells with the crate suite and all gates and reject an early or unconfigured run. | W75,W76,W15,W16,W78,W79,W80,W81,W82,W83,W84,W85,W86,W87,W88,W89,W99,W106,W108,W109,W111,W113,W114,W117,W118,W120 | 09-verification | 03-step-repo-gates |

| W49 | verification | `N/A` | `artifact matrix is green` | `N/A` | The five-artifact CI matrix passes, each job having executed its own artifact once, including the windows msvc leg through PowerShell. | W48,W19 | 09-verification | 04-step-artifact-matrix-green |

| W50 | source | `src/plan-overview/src/plan/mode.rs` | `derive_mode()` | `N/A` | Derive the lifecycle mode from the review status and step statuses: planning while the review is pending or no step has started, implementing once it is approved and work has begun, and complete when every step and its verification have passed. An ambiguous combination is reported as ambiguous rather than guessed. | W102 | 02-page-shell | 08-step-derive-mode |

| W51 | source | `src/plan-overview/src/render/shell.rs` | `render_mode_surface()` | `N/A` | Select which surface leads for the derived mode and state the mode on the page: planning leads with soundness, implementing leads with execution, complete leads with the outcome. A surface irrelevant to the current mode is reachable but not primary, never silently absent. | W50 | 02-page-shell | 09-step-mode-surface |

| W52 | test | `src/plan-overview/tests/mode.rs` | `mode_derivation` | `N/A` | Test lifecycle derivation including the exact approved-with-zero-steps empty-approved fixture as ambiguous with the contradiction named while its page remains readable. | W51 | 02-page-shell | 10-step-test-mode |

| W53 | source | `src/plan-overview/src/pages/autoplay.rs` | autoplay_subject() | `N/A` | Autoplay's subject is decided by the mode. In implementing mode it follows the active step. In planning mode it follows the plan being built: units appearing, dependency edges forming. In complete mode there is no active subject, so autoplay is unavailable and states why rather than offering a control that cannot move. An earlier version of this row said autoplay exists in every mode, which adversarial findings AR-08 and AR-24 recorded as contradicting US-64 and plan 8.6. | W50 | 08-autoplay | 06-step-autoplay-availability |

| W54 | source | `src/plan-overview/src/pages/graph.rs` | `layout_nodes()` | `N/A` | Compute stable node positions for the dependency graph so the same plan lays out identically twice, and an added unit displaces the existing layout as little as possible. Position is derived from dependency depth and ordering rather than from a random seed. | W37 | 07-graph-and-coverage-of-data | 05-step-layout-nodes |

| W55 | source | `src/plan-overview/assets/graph.js` | `nodeDetail` | `N/A` | Open the detail for a clicked node beside the graph without leaving the graph page, so a reader can inspect a unit and keep the structure in view, and can continue clicking along edges from there. | W54 | 07-graph-and-coverage-of-data | 06-step-node-detail |

| W56 | source | `src/plan-overview/assets/graph.js` | `animateGrowth` | `N/A` | Animate the difference between two states rather than redrawing: a new node eases in, a new edge draws from its source, a moved node travels to its new position. Honour the reduced-motion preference by applying the same state change without motion. | W55 | 07-graph-and-coverage-of-data | 07-step-animate-growth |

| W57 | test | `src/plan-overview/tests/graph_layout.rs` | `layout_is_stable` | `N/A` | Pin that layout is deterministic for one plan and that adding a unit moves the fewest existing nodes. Fault-inject by reordering the inventory rows and requiring identical positions. | W56 | 07-graph-and-coverage-of-data | 08-step-test-layout |

| W58 | source | `src/plan-overview/src/watch.rs` | `watch_plan_dir()` | `N/A` | Detect changes under the plan directory and report them as coalesced change events. Dependency-free means no filesystem-notification crate: a bounded mtime and size scan over the plan tree, with the interval and the tree size both stated, is the portable mechanism across Linux, macOS and Windows. | W14 | 10-live-updates | 01-step-watch-plan-dir |

| W59 | source | `src/plan-overview/src/watch.rs` | `coalesce_events()` | `N/A` | Collapse a burst of writes into one change event so a helper rewriting several files does not produce several redraws, and state the debounce interval rather than tuning it invisibly. | W58 | 10-live-updates | 02-step-coalesce-events |

| W60 | source | `src/plan-overview/src/serve.rs` | `state_stream()` | `N/A` | Publish state changes to connected pages as a stream, so the page follows without polling the whole artifact. A client that reconnects receives the current state rather than only subsequent changes. | W59 | 10-live-updates | 03-step-state-stream |

| W61 | source | `src/plan-overview/assets/live.js` | `applyStateChange` | `N/A` | Apply an incoming state change to the open page: update the values in place, hand the graph its before and after for animation, and preserve scroll position, expanded sections and the selected autoplay tab. | W60 | 10-live-updates | 04-step-apply-change |

| W62 | test | `src/plan-overview/tests/watch.rs` | `coalescing_and_scan` | `N/A` | Pin the watcher: a single edit yields one event, a burst of edits within the debounce yields one event, and a change to a file outside the plan directory yields none. Fault-inject by touching a file without changing it and requiring no event. | W61 | 10-live-updates | 05-step-test-watch |

| W63 | verification | `N/A` | `edit on disk, page follows` | `N/A` | With the binary serving, edit a plan document through a planning helper and confirm the page follows within the stated delay, that the graph animated rather than redrew, and that scroll position and expanded state survived. | W62 | 10-live-updates | 06-step-verify-live |

| W64 | style | `src/plan-overview/assets/tokens.css` | .tokens | `N/A` | Define the palette, luminance steps, spacing and type scale as tokens on the root, so every page and panel draws from one place and a colour cannot be introduced ad hoc. Neutrals carry a slight hue bias toward the accent rather than being pure grey. | W07 | 11-visual-language | 01-step-tokens |

| W65 | style | `src/plan-overview/assets/depth.css` | .panel | `N/A` | A named depth scale for layered translucent panels over the dark ground: background blur, border luminance and shadow per level, with a stated maximum so layering cannot become soup. Content contrast is computed against the composited background, not the token. | W64 | 11-visual-language | 02-step-depth-scale |

| W66 | style | `src/plan-overview/assets/motion.css` | .motion | `N/A` | Declared durations and easings as tokens with a stated purpose each, so a transition cannot be hand-tuned per element. A single reduced-motion block neutralises duration and transform while leaving every state change applied. | W64 | 11-visual-language | 03-step-motion-system |

| W67 | source | `src/plan-overview/src/render/shell.rs` | `render_transition()` | `N/A` | Emit the markup and classes that let a route change animate as a transition rather than a cut, including the direction of travel so moving deeper and moving back are distinguishable. | W66 | 11-visual-language | 04-step-page-transition |

| W68 | source | `src/plan-overview/assets/ambient.js` | `activityPulse` | `N/A` | Indicate that the page is live and something is happening, readable at a glance from a distance and without reading a number: a pulse tied to real state arrival, quiescent when nothing is arriving, and never a decorative animation that implies activity that is not occurring. | W66 | 11-visual-language | 05-step-activity-pulse |

| W69 | verification | `N/A` | `frame budget during graph growth` | `N/A` | Measure the frame budget while the graph animates growth on the 337 KB fixture, and record both the numbers and the cinematic tier the page settled at. A measured budget miss at the full tier is expected on a slow machine and is evidence the degradation works, not a failure; a miss that does not step the tier down is a finding. | W68,W67 | 11-visual-language | 06-step-verify-frame-budget |

| W70 | verification | `N/A` | `contrast and meaning redundancy` | `N/A` | Check every token pair used for text against its composited background for contrast, and confirm no status, anomaly or activity is conveyed by colour, glow or motion alone. Record the measured ratios. | W65 | 11-visual-language | 07-step-verify-contrast |

| W71 | source | `src/plan-overview/src/render/tiers.rs` | `emit_tier_table()` | `N/A` | Emit one declared table of tiers: the frame-time thresholds for stepping down and up, the hysteresis window, and exactly which effects each tier disables. Thresholds live here rather than being tuned inside the client, so they are reviewable and testable in one place. | W66 | 12-adaptive-cinematics | 01-step-tier-table |

| W72 | source | `src/plan-overview/assets/perf.js` | `frameSampler` | `N/A` | Sample frame times over a rolling window during animation and report a sustained figure rather than a single frame, so one slow frame from an unrelated cause does not change the tier. | W71 | 12-adaptive-cinematics | 02-step-frame-sampler |

| W73 | source | `src/plan-overview/assets/perf.js` | `applyTier` | `N/A` | Apply the selected tier as one attribute on the root so the style layer switches wholesale, and state the current tier where a reader can see it. The reduced-motion preference pins the minimal tier and no measurement overrides that. | W72 | 12-adaptive-cinematics | 03-step-apply-tier |

| W74 | test | `src/plan-overview/tests/tiers.rs` | `tier_selection` | `N/A` | Pin the decision from the table: a sustained miss steps down one tier, sustained headroom steps up one, a value inside the hysteresis window changes nothing, and no sequence of samples can flap between tiers. Fault-inject alternating fast and slow samples and require a stable tier. | W73 | 12-adaptive-cinematics | 04-step-test-tiers |

| W75 | config | `run-tests.sh` | `test discovery` | `N/A` | Discover and run the crate test suite alongside the shell tests, so a failing cargo test fails the repository gate. Today discovery is a single find for test-star.sh at line 87, which means no Rust test can ever redden the suite however many the plan declares. | W05 | 03-serve-and-distribution | 08-step-cargo-test-leg |

| W76 | test | `tests/test-mode-markers.sh` | `exempt()` | `N/A` | Add the exemption arm the shipped prebuilt artifacts need, with a stated reason, and remove the now-dead arm exempting the deleted runtime directory. A compiled file carries no comment syntax, so it can hold no marker; a stale arm exempting paths that no longer exist hides the next real violation. The Cargo.lock arm already exists and is not added here. The crate sources and the stylesheets are not exempt: they carry markers, read by the forms W109 makes available. | W16,W17 | 03-serve-and-distribution | 09-step-marker-exemption |

| W77 | config | `src/plan-overview/Cargo.toml` | `crate manifest` | `N/A` | Create the crate manifest for the renderer with no runtime dependencies and a named test-per-field-buffer feature used only by W117's deterministic mutation test. The crate sits at src/plan-overview, one directory per binary under src, as CODE-STYLE section 1b requires. | -- | 01-engine-core | 07-step-crate-manifest |

| W78 | config | `planning/templates/plan-overview.html.tmpl` | `file removal` | `N/A` | Delete the at-underscore token template. It exists only for the substitution pass the binary replaces, and leaving it behind invites a future reader to wire it back up. | W15 | 13-removal-declared-surface | 01-step-remove-template |

| W79 | config | `planning/scripts/overview-serve.sh` | `file removal` | `N/A` | Delete the serve wrapper that chose a runtime rung and passed the plan directory to it. The binary serves the artifact itself, so the wrapper has nothing left to choose between. | W16 | 13-removal-declared-surface | 02-step-remove-serve-wrapper |

| W80 | config | `planning/requires.tsv` | `overview-server-runtimes group` | `N/A` | Remove the four-row any-of group that declared python3, node, perl and socat for the overview server. A shipped binary asks nothing of the box, so the requirement is not merely satisfied differently, it is gone. | W79 | 13-removal-declared-surface | 03-step-drop-runtime-requirements |

| W81 | config | `planning/PACKAGE-MANIFEST.tsv` | `removed renderer rows` | `N/A` | Remove the rows naming the renderer, its template, the serve wrapper and the runtime directory, and add the rows for the prebuilt artifacts. A manifest that lists a deleted file fails its own freshness check. | W17,W110,W113 | 13-removal-declared-surface | 04-step-manifest-rows |

| W82 | config | `planning/PACKAGE-MAP.tsv` | `removed renderer entries` | `N/A` | Remove the map entries for the same deleted files and record the artifacts in their place, so the map and the manifest agree on what ships. | W81,W110 | 13-removal-declared-surface | 05-step-package-map |

| W83 | docs | `planning/SKILL.md` | `plan overview section` | `N/A` | Correct the instructions that tell an agent to run render-plan-overview.sh or overview-serve.sh, naming the binary and its subcommands instead. This is the contract agents act on, so a stale instruction here is followed rather than noticed. | W82 | 13-removal-declared-surface | 06-step-skill-instructions |

| W84 | docs | `planning/docs/README.md` | `overview section` | `N/A` | Correct the reference documentation for the overview: how it is rendered, how it is served, and what a platform without a prebuilt artifact is told. | W83 | 13-removal-declared-surface | 07-step-docs-readme |

| W85 | test | `planning/tests/test-plan-overview.sh` | `test-plan-overview` | `N/A` | Retire the twelve assertions that drive render-plan-overview.sh directly and replace them with the equivalents against the binary, so the coverage the suite had is not lost with the file it tested. | W15 | 14-removal-test-surface | 01-step-retire-overview-test |

| W86 | test | `planning/tests/test-overview-serve.sh` | `test-overview-serve` | `N/A` | Retire the four assertions that drive overview-serve.sh and replace them with serve-mode assertions against the binary, including the port-printed-before-first-request property. | W79 | 14-removal-test-surface | 02-step-retire-serve-test |

| W87 | test | `planning/tests/test-plan-dir-synonym.sh` | `plan-dir coverage note` | `N/A` | Correct the note that delegates plan-dir coverage to test-overview-serve.sh, and cover the synonym directly rather than by reference to a retired test. | W86 | 14-removal-test-surface | 03-step-plan-dir-synonym |

| W88 | test | `planning/tests/test-duplication-ratchet.sh` | `canonicalisation site count` | `N/A` | Correct the ratchet entry that counts render-plan-overview.sh cells() as a canonicalisation site. The count is the assertion, so removing a site without correcting it reddens the suite. | W15 | 14-removal-test-surface | 04-step-duplication-ratchet |

| W89 | test | `planning/tests/test-portability-contract.sh` | `python3-shipped allowlist arm` | `N/A` | Remove the allowlist arm that exempts overview-serve.sh from the python3-shipped rule. With the wrapper gone the exemption has nothing to exempt, and a stale allowlist arm hides the next real violation. | W80 | 14-removal-test-surface | 05-step-portability-allowlist |

| W90 | verification | `N/A` | memory ceiling across both fixtures | `N/A` | Consume W117 on W91 and W92 with exact rustc 1.86.0 and fixture checksums. Record peak resident memory total bytes total allocation count RenderBuffer allocation count and growth count. Require one RenderBuffer allocation and zero growth and require the per-field mutation to fail the crate test. | W47,W92,W91,W117 | 09-verification | 05-step-memory-ceiling |

| W91 | data | `planning/tests/fixtures/overview/navigation` | `navigation fixture snapshot` | `N/A` | Check in a frozen snapshot of the 82-unit plan the navigation stories are recorded against, replacing the live plans-root directory as the evidence source. A live plan can be changed or deleted by a plans-root helper, so evidence recorded against it is not reproducible. | -- | 15-fixture-corpus | 01-step-navigation-fixture |

| W92 | data | `planning/tests/fixtures/overview/size` | `size fixture snapshot` | `N/A` | Check in a frozen snapshot of the 337 KB state that fails to render today, so the size claim and the memory ceiling are measured against a fixed input rather than a moving one. | -- | 15-fixture-corpus | 02-step-size-fixture |

| W93 | data | `planning/tests/fixtures/overview/anomalies` | `structural anomalies fixture` | `N/A` | A fixture carrying the structural edge cases no real plan happens to have: a deliberately orphaned work unit, a single-unit goal with a recorded size exception, and a goal whose testing requirement is no. | -- | 15-fixture-corpus | 03-step-anomalies-fixture |

| W94 | data | `planning/tests/fixtures/overview/evidence-gaps` | `evidence gaps fixture` | `N/A` | A fixture carrying the evidence edge cases: a finding with a blank work-unit cell, a finding with a gated fix key, a coverage outcome with no proving unit, and a review cycle that recorded no findings. | -- | 15-fixture-corpus | 04-step-evidence-gaps-fixture |

| W95 | data | `planning/tests/fixtures/overview/complete` | `complete plan fixture` | `N/A` | A fixture in which every step and every verification has passed, which is the completed state no live plan in the plans root is currently in. | -- | 15-fixture-corpus | 05-step-complete-fixture |

| W96 | data | `planning/tests/fixtures/overview/empty-approved` | `approved with no steps fixture` | `N/A` | Create exactly one approved-with-zero-steps fixture. It is deliberately ambiguous for mode derivation and must render as a readable empty plan with no alternative fixture or escape hatch. | -- | 15-fixture-corpus | 06-step-empty-approved-fixture |

| W97 | data | `planning/tests/fixtures/overview/fresh` | `fresh plan fixture` | `N/A` | A fixture with no findings, no completed steps and no review cycles, which is what a plan looks like on its first day and what the page must not present as a failure. | -- | 15-fixture-corpus | 07-step-fresh-fixture |

| W98 | data | `planning/tests/fixtures/overview/malformed-state` | `malformed state fixture` | `N/A` | A fixture whose state document is truncated or invalid and whose history carries a transition with no recorded time, so the page's behaviour on damaged input is observed rather than assumed. | -- | 15-fixture-corpus | 08-step-malformed-state-fixture |

| W99 | test | `planning/tests/test-overview-fixtures.sh` | `fixture corpus contract` | `N/A` | Pin that each fixture still carries the edge case it exists for, so a fixture edited for another reason cannot silently stop covering the story that depends on it. | W91,W92,W93,W94,W95,W96,W97,W98 | 15-fixture-corpus | 09-step-fixture-contract-test |

| W100 | source | `src/plan-overview/src/main.rs` | `module tree` | `N/A` | The crate root: the module declarations that make plan, render, pages, serve and watch reachable, and nothing else. Without it every other source unit names a file the compiler never sees. | W77 | 01-engine-core | 08-step-crate-root |

| W101 | source | `src/plan-overview/src/main.rs` | `parse_args()` | `N/A` | The command-line surface the binary exposes: --plan-dir, --out, --refresh, --watch, --serve and --port, with the same meanings the removed wrapper and its runtime servers gave them. This is the contract the skill contract and the documentation both name, so it is decided here rather than discovered at execution. | W100 | 01-engine-core | 09-step-cli-surface |

| W102 | source | `src/plan-overview/src/plan/extract.rs` | `extract_state()` | `N/A` | Extract the canonical typed state from the plan tree and serialize that exact state for the parser contract; never invoke the old shell extractor at runtime. | W01 | 01-engine-core | 10-step-state-extraction |


| W104 | source | `installer/src/20-runtime-tools.sh` | artifact platform selection and placement branch | `N/A` | Implement one normalize_platform function for production uname and PROCESSOR_ARCHITECTURE inputs plus test-only PLAN_OVERVIEW_TEST_MODE inputs PLAN_OVERVIEW_TEST_OS and PLAN_OVERVIEW_TEST_ARCH. Use it for artifact selection and placement and route unmatched keys to W18. | W17,W110,W115,W18 | 16-toolchain-and-packaging | 02-step-platform-selection |

| W105 | config | package.json and planning/tests/fixtures/overview/npm-package-baseline.tsv | files field and npm package baseline fixture | `N/A` | Include all five target artifacts and install.sh in the npm package and generate or refresh the single repository-owned baseline at planning/tests/fixtures/overview/npm-package-baseline.tsv. The baseline is tab-separated with a fixed header, one sorted tarball path and byte-size row per packaged file, and one final tarball-bytes row; W105 owns its generation and refresh after npm pack. W120 reads this exact file and creates no alternate expectations. | W17,W113 | 16-toolchain-and-packaging | 03-step-npm-packaging |

| W106 | verification | `N/A` | `install on each declared platform` | `N/A` | Install from the repository and from the packed npm tarball on each declared platform and on one unsupported pair, execute the selected artifact, and run the installed artifact in render and serve modes with the prohibited interpreter stack unavailable or trapped. | W17,W18,W19,W104,W105,W115,W118,W119 | 16-toolchain-and-packaging | 04-step-verify-platforms |


| W108 | verification | `N/A` | `marker pair across the crate` | `N/A` | Confirm every file under src/plan-overview carries the marker its kind requires, on the first two lines: MODE: DEV and PACKAGE: PROD on the manifest and the sources, MODE: DEV alone on the toolchain file, and an exemption reason for the generated lock and the built artifacts. Each source unit writes its own markers; this unit is where the whole crate is checked once, after the sources exist. | W74,W107,W109 | 09-verification | 06-step-crate-markers |

| W109 | test | `tests/test-mode-markers.sh` | `marker_of()` | `N/A` | Read a CSS block-comment marker form so a stylesheet can declare its audience. The reader accepts a hash comment, an HTML comment and a double-slash comment; none is valid CSS, so the three stylesheets the crate embeds can hold no readable marker and would fail the gate with no way to pass it. | W76 | 03-serve-and-distribution | 10-step-css-marker-form |

| W110 | config | `planning/binaries.tsv` | `shipped binaries registry` | `N/A` | Declare per target which artifact the skill ships: the uname pair, the artifact path and its checksum. CODE-STYLE section 1b names this file as how a skill declares what ships; it does not exist in the repository, and this plan is the first thing to ship a binary, so this unit is where the declared mechanism becomes real. | W19 | 16-toolchain-and-packaging | 06-step-binaries-registry |

| W111 | test | `tests/test-shipped-binaries.sh` | `shipped binaries contract` | `N/A` | Validate planning/binaries.tsv against the committed artifacts and the regenerated installer: every declared target has an artifact and checksum match, every artifact is declared, and the installer exposes exactly the declared artifact paths. | W110,W113,W115 | 16-toolchain-and-packaging | 07-step-binaries-contract-test |


| W113 | generated | `install.sh` | `regenerated installer` | `N/A` | Regenerate and commit install.sh after the declarations it is generated from have changed. It is a tracked generated file whose freshness gate diffs it against its source, so leaving it uncommitted reddens two gates while no unit owns the file. This is the inseparable generated-file exception: build.sh writes the whole file in one pass, so individual review is impossible. | W80,W17,W110,W114,W104,W18 | 16-toolchain-and-packaging | 09-step-regenerate-installer |

| W114 | config | `installer/src/50-manifest.sh` | skill_files dev and prod registration arms | `N/A` | Register the checked-in fixture corpus in the dev arm and planning/binaries.tsv in the prod arm of installer/src/50-manifest.sh; W17 owns the planning-arm artifact entries, so these scopes do not overlap. | W99,W110 | 16-toolchain-and-packaging | 10-step-register-dev-files |

| W115 | data | `planning/bin` | `committed artifacts` | `N/A` | Commit the five prebuilt artifacts the matrix builds, one per target, named by uname pair. The manifest gate requires every file skill_files() promises to exist on disk, so the artifacts must be in the tree rather than fetched at install time, and this is where the repository takes on their committed size as a stated cost rather than an accident. | W19,W110 | 16-toolchain-and-packaging | 11-step-commit-artifacts |

| W103 | config | `flake.nix` | `rust toolchain input` | `N/A` | Pin Rust 1.86.0 in the existing flake toolchain declaration and require the targets and components needed by the artifact matrix. | W77 | 17-build-toolchain | 01-step-toolchain-floor |

| W107 | config | `src/plan-overview/rust-toolchain.toml` | `toolchain pin` | `N/A` | Pin src/plan-overview/rust-toolchain.toml to Rust 1.86.0 with the artifact matrix targets and components and mark it MODE: DEV. | W77 | 17-build-toolchain | 02-step-toolchain-pin |

| W112 | config | `.github/workflows/ci.yml` | `rust toolchain step` | `N/A` | Install Rust 1.86.0 and the artifact matrix targets in CI before the crate leg and make a missing or mismatched toolchain fail rather than degrade. | W107,W75 | 17-build-toolchain | 03-step-ci-toolchain |

| W116 | verification | `N/A` | `one compiler in three places` | `N/A` | Verify rustc 1.86.0 resolves identically in flake dev shell crate build and CI and record a non-zero CI crate test count and required target components. | W103,W107,W112 | 17-build-toolchain | 04-step-verify-one-compiler |

| W117 | test | `src/plan-overview/tests/memory.rs` | `memory_harness()` | `N/A` | Register memory.rs against RenderBuffer::new and RenderBuffer::write_str in render/shell.rs. Record separate allocation and growth counters for that production buffer. Run cargo test --test memory and cargo test --features test-per-field-buffer --test memory on W91 and W92; the first passes with one allocation and zero growth and the mutation exits non-zero from per-field String allocation. | W07,W91,W92 | 09-verification | 07-step-memory-harness |

| W118 | test | `src/plan-overview/tests/runtime_isolation.rs` | `installed_binary_has_no_prohibited_children()` | `N/A` | Register runtime_isolation.rs to install the artifact in a temporary root and run bounded render and serve lifecycles under strace on Linux sandbox-exec on macOS and a Windows Job Object. Require readiness termination and zero prohibited child or overview-script executions. | W14,W104,W115 | 09-verification | 08-step-runtime-isolation |

| W119 | test | planning/tests/test-platform-selection.sh | platform_selection_contract | `N/A` | Test W104 normalize_platform through the installer path with PLAN_OVERVIEW_TEST_MODE=1 and explicit supported OS and architecture values including Windows_NT AMD64 and one unsupported Windows architecture. Assert normalized keys selected paths and no artifact plus the unavailable notice. | W104,W110 | 16-toolchain-and-packaging | 12-step-platform-selection-test |

| W120 | test | `planning/tests/test-npm-package.sh` | `npm_package_install_flow` | `N/A` | Pack and extract the npm tarball in a clean temporary directory and compare it with the single W105-owned planning/tests/fixtures/overview/npm-package-baseline.tsv record. Invoke the generated installer through W104 test-only platform inputs and assert the supported and unsupported results without defining a second package file list or byte-size baseline. | W105,W104,W113 | 16-toolchain-and-packaging | 13-step-npm-package-test |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
