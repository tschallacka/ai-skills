# Plan: Plan overview: edge-navigable pages and a limit-free renderer

## Current state

§ 2.1
The overview renderer fails on large plans. render-plan-overview.sh line 436 passes the whole plan state to jq as an --arg; on the codegraph-bash-indexing-v2 plan the state is about 337 KB and the call dies with Argument list too long, exit 126, producing a zero-byte page. A second jq call then substitutes about 35 keys with reduce and gsub, rebuilding a 180 KB template once per key.

§ 2.2
Serve mode was separately broken and has since been fixed in the repository. As recorded when this plan was written: with the python3 rung on the xcore-paymentmethod-code-aliases plan, /state.json returned 173 KB correctly while / returned zero bytes and the server logged BrokenPipeError on every request, while the same renderer invoked directly exited 0. Re-measured on the current tree, that no longer reproduces: state.json returns 172956 bytes and the page route returns HTTP 200 with 181335 bytes, with nothing logged. The direct render still exits 0 and writes 181673 bytes. The fix landed in the B51 to B59 batch, outside this plan. This paragraph is kept rather than deleted because 8.2 and the scope boundary were both argued from it.

§ 2.3
The rendered page is 176 KB and 1239 lines with one nav element, zero in-page anchor links, and 336 details elements with no index over them. Measured in Chrome on a real 82-unit plan: the three content columns are fixed width and clip their own text, so coverage work-unit lists cut off mid-list and finding text runs under the adjacent panel. Data present in the DOM is invisible on screen.

§ 2.4
The Tests column is 20 rows reading see companion for details, none of them links. Plan identity dumps all twelve goals as unformatted prose at about 11px in a narrow column, while the goal panel above truncates the same goal names to about 20 characters. The dashboard header itself works: donut, per-goal bars, three radials and the cycle chart are readable and correct.

## Desired outcome

§ 3.1
The overview becomes a set of pages that flow into each other rather than one page of panels. Each page has one job: an overview page that monitors live state, a goal page, a work-unit page, a finding page, a tests page, a coverage page, a history page, and a graph page.

§ 3.2
Relationships are the navigation. Every dependency, ownership and grading relationship is a link, so a reader moves from a unit to the units it depends on, from there to what those block, and from a finding to the unit it names, without returning to an index. Traversal never dead-ends: each page shows the next set of edges.

§ 3.3
It serves both purposes at once. Monitoring answers what is happening now and updates without a reload. Reviewing answers whether the work is actually right, with nothing truncated, nothing clipped, and no dead-end placeholder text.

§ 3.4
Tests are visible as content, not as a pointer. A test or verification unit shows what it runs, the command, its status, and the unit it proves. The testing companion content is rendered rather than referenced.

§ 3.5
The plan's evolution is visible: status transitions, what is in progress now, which findings were superseded and by what, which work units or approaches were discarded and the recorded reason. A reader can see what the plan used to say and why it changed.

§ 3.6
No plan size makes the page fail. The renderer handles the largest existing plan, currently 242 files and about 337 KB of state, without an argument-length limit and without rebuilding the template once per key.

§ 3.7
Every field the state stores is presented somewhere deliberate. overview-state.sh emits identity, goals, steps, edges, testingMarks, coverage, findings, cycles and reviewTarget; nothing is extracted and then dropped, and each field appears on the page whose job it is rather than in a catch-all panel.

§ 3.8
An autoplay mode follows the work. When enabled, the page navigates itself to the step the agent is currently working on and moves as that changes, so an unattended screen tracks execution without a human driving it. It is opt-in, visibly on or off, and never hijacks navigation while the reader is browsing.

§ 3.9
An autoplay tab exists only while its state is active. When a state stops being active its tab vanishes rather than lingering for dismissal, so the tab strip is always a truthful picture of what is currently open.

§ 3.10
The pages distinguish planning state from implementing state, because the two ask different questions of the same plan. In planning state the reader is judging whether the plan is sound: decomposition, coverage of the definition of done, open findings, goal sizes, dependency order and the review verdict. Zero progress is the correct reading there and must not be presented as failure. In implementing state the reader is following execution: what is done, what is in progress, what is blocked, what verification passed, and where the plan and the repository have diverged. The mode is derived from the review status and the step statuses, is stated on the page, and decides which surface leads.

§ 3.11
The page is meant to look and feel exceptional while it is working, in the register of a control surface rather than a document: layered translucent panels over a dark ground, luminance carrying emphasis, values that move rather than jump, page changes that transition rather than cut, and an ambient sense that something live is happening. This is stated as testable properties rather than a mood: motion has declared durations and easings, depth is a named layer scale, emphasis is luminance plus one accent, transitions between pages are animated, and activity is visible without reading a number. The aesthetic never carries meaning alone and never costs legibility, which is the defect being fixed.

§ 3.12
The cinematics degrade gracefully with measured framerate rather than being fixed. The page samples its own frame times and selects a tier: full effects, reduced, or minimal. A sustained budget miss steps down; sustained headroom steps back up, with hysteresis so the tier cannot flap. A tier removes effect only, never information, and the reduced-motion preference pins the minimal tier regardless of how fast the machine is.

§ 3.13
The shipped overview has no runtime dependency on Bash, jq, Python, Node, Perl, socat, or any renderer shell script. The Rust binary is compiled from the pinned toolchain, reads the plan tree or supplied state through its own code, renders the artifact, and serves it; the compiler is a build-time dependency only.

## Approach

§ 4.1
Replace the rendering pipeline rather than patching it. A dependency-free Rust binary reads the plan tree, computes the derived values, and writes the pages directly. It takes its input from files and stdin, so no argument-length limit applies, and it emits the page in one pass instead of rebuilding a template once per key.

§ 4.2
Existing infrastructure that hinders the new design is removed, not preserved. The at-underscore token template and its reduce-and-gsub substitution go, because the new pages are generated rather than filled in. The sections endpoint that slices HTML with a regex against a hardcoded id list goes, because pages replace panels. The bash panel builders go where they produce the defects being fixed: fixed-width columns that clip, the twenty identical see-companion rows, and the goal prose dump.

§ 4.3
The four server rungs and the jq renderer all go. The Rust binary is the only renderer and the only server, so the python, node, perl and socat servers, their three duplicate implementations of render-and-slice, and render-plan-overview.sh itself are removed rather than kept as a second path. There is no degraded mode to maintain, and no second implementation that can disagree with the first.

§ 4.4
Deliver the pages as one hash-routed artifact with the state embedded, so the same file works opened from disk, served over HTTP, and live-updating when served. Deep links are URLs; the overview is the default landing page; breadcrumbs and a back-stack both exist; a peek expands a related item inline without losing position.

## Scope

§ 5.1
In scope: a Rust binary that parses the plan tree, renders the new pages, and serves them; removal of the token template, the sections-slicing endpoint, the three duplicate server runtimes and the panel builders that produce the current defects; the new page structure with edge navigation; visible test content; visible plan evolution including discards and their reasons; autoplay; and browser verification of every page.

§ 5.2
Out of scope: changing what the plan documents look like, and changing the plan helper scripts other than removing the renderer and server scripts this plan replaces. An earlier version of this paragraph said overview-state.sh is read for its field contract but its extraction is not redesigned here, and left unsaid which input the binary actually uses at run time. That ambiguity is now resolved, because it had to be: the binary derives the state from the plan tree itself, reading files directly as W01 specifies, and it invokes no shell script at run time. Windows is a declared target and has no bash, so a runtime dependency on overview-state.sh would mean shipping an artifact that cannot work on the platform it is built for. overview-state.sh therefore serves two narrower purposes: its emitted document is the field contract the binary reproduces, so other consumers keep working, and its output is a fixture the parser is tested against. The runtime chain is W01 to W102 to W03 and W50: the tree is read, extraction produces the state values, and every derived number and the lifecycle mode follow from that. W02 parses a state document where one is supplied as input, whether that is the document W102 emits or one a caller hands in, and it is the reader behind the served state endpoint rather than a second producer. A further version of this paragraph left the graph contradicting this sentence, with W03 and W50 depending on W02 while W02 and W102 were siblings; adversarial findings AR-03 and AR-18 recorded it, and the dependencies now match the chain stated here.

§ 5.3
Deliberately not preserved: the at-underscore token contract, the reduce-and-gsub substitution, the sections-slicing endpoint, the four runtime rungs, the fixed three-column layout, the see-companion placeholder rows, the goal prose dump, and the jq renderer entirely. Each is named because a reader of the old code would otherwise assume it is a requirement.

§ 5.4
The dependency-removal boundary is explicit: the old renderer's Bash and jq path, the four runtime server implementations, and their interpreter requirements are removed rather than wrapped or retained as fallback. Rust and the host platform's normal socket and filesystem APIs are the only requirements of the running binary.

## Affected areas

§ 6.1
planning/scripts/render-plan-overview.sh, its template, planning/scripts/overview-state.sh as a reader only, planning/scripts/overview-serve.sh and planning/scripts/runtime/ for the served path, planning/requires.tsv for the soft declaration, installer/build.sh and install.sh for the generated manifest, and planning/tests/ for the gates that cover them.

§ 6.2
New: a Rust crate for the renderer at src/plan-overview inside this repository — one directory per binary under src, as CODE-STYLE section 1b requires, holding the crate's own Cargo.toml, rust-toolchain.toml and src tree. Its sources are marked MODE: DEV and PACKAGE: PROD, the pair used for library sources that are never delivered but whose content ends up inside something that is, and the prebuilt per-platform artifacts are marked PROD. An earlier version of this paragraph placed the crate at a path outside the repository, carried over from a spike, and left the real location unresolved; that was reversed on instruction — everything lives in this repository, nothing external. A later version placed it at planning/overview, inside the skill; adversarial finding AR-13 recorded that section 1b puts a binary beside the skill directories rather than inside one, because several skills may consume one tool while a skill directory is what the installer copies. That version also cited chat as the precedent, with a binaries.tsv registry declaring what ships and a marker exemption covering files that cannot carry a comment; adversarial finding AR-19 recorded that there is no src/chat, no binaries.tsv and no test-shipped-binaries.sh anywhere in the tree, so the precedent was a description of an intention. This plan is the first thing in the repository to ship a binary, so W110 and W111 create the declaration and its validator rather than inheriting them, and CODE-STYLE section 1b names files that exist once they have run.

## Constraints and decisions

§ 7.1
ai-skills ships bash and jq only, and install.sh copies files, so a compiled artifact is a new kind of shipped file for this repository and the manifest, marker and packaging consequences are real work this plan must own. An earlier version of this paragraph concluded from that premise that the binary is therefore optional and additive and that the jq path must keep working; that conclusion was reversed on instruction, because two renderers can disagree and the jq path carries the argument-length limit this plan exists to remove. What remains true is the premise: no install may fail silently, so a host with no prebuilt artifact is told the overview is unavailable rather than being given a renderer that cannot run.

§ 7.2
The bash 3.2 floor and the BSD userland target still apply to every shell file this plan touches. The two-shell harness and the portability catalogue remain the gates.

§ 7.3
Plan directories under the plans root are user data. The renderer may write its own output file into a plan directory because that is its existing contract, and must not write anything else there.

§ 7.4
The user has decided: a full redesign with a new information architecture; pages that flow with edge-following navigation; overview-first landing plus deep links, with breadcrumbs and a back-stack; both page navigation and inline peek; both static and served operation; visible tests, evolutions and discarded items with their reasons; autoplay driven by served state with vanishing tabs per active state; a file monitor on the plan directory; and cinematics that degrade gracefully with framerate. An earlier version of this paragraph recorded the decision as an optional binary with a bash fallback, with Rust covering only substitution and state parsing; that was reversed on instruction. The binary is the only renderer and the only server, and the jq renderer and its four runtime rungs are deleted rather than kept as a fallback.

§ 7.5
The runtime dependency decision is strict: the installed overview MUST run without Bash, jq, Python, Node, Perl, socat, or overview shell scripts. The Rust compiler and Cargo are development and build dependencies, not runtime dependencies, and no fallback renderer is permitted.

## Risks and open questions

§ 8.1
Browser verification needs the Chrome extension to have site permission for the loopback host. Without it neither the served page nor a file URL can be opened, and the UI stories cannot be exercised. Recorded as an environment prerequisite rather than a plan risk once granted.

§ 8.2
Serve mode's failure on a large plan is measured, and the measurement has moved twice. Two early versions of this paragraph got the cause wrong: the first hypothesised, and marked unverified, that render-plan-overview.sh does not support the --out flag the server passes it, which is false because it declares and parses both --out and --serve; the second concluded the cause was different and unidentified. The third named the real one: runtime/overview-server.py ran the renderer without inspecting its exit status, so a render that died with jq Argument list too long left the temp file empty and the empty body was sent under HTTP 200. That was filed as B51 and has since been fixed. Re-measured on the current tree, the 341088-byte codegraph-bash-indexing-v2 state returns HTTP 500 with a fourteen-byte body reading render failed, while state.json returns all 341088 bytes; the exit status 126 and the jq Argument list too long text appear on the server's stderr and never reach the client. The 173 KB xcore plan serves its page whole. An earlier version of this sentence said the 500 carried a body naming the cause; the same session's own measurement had recorded fourteen bytes, and adversarial finding AR-39 recorded the overstatement. So the serve-mode symptom is now an honest error status rather than a silent empty page, though a client still learns only that the render failed and an operator must read the log for why. Adversarial finding AR-23 recorded that the third mechanism explained the 341 KB case but not 2.2's 173 KB observation; the answer is that the two are different cases and the second had already been fixed. The consequence for this plan is unchanged: the existing server cannot be used to judge whether a renderer works when served, so goal 03 serves with the new binary and goal 09 verifies it there.

§ 8.3
The optional binary has no signing or notarisation obstacle on macOS, measured on an arm64 runner: curl does not set the quarantine attribute, an explicitly quarantined CLI binary still ran, an unsigned binary ran, and cargo already ad-hoc signs through the linker. That last measurement is the load-bearing one and it carries a condition the earliest version of this paragraph did not state: the linker ad-hoc signs only when the build runs on macOS. Apple Silicon kills an arm64 binary that carries no signature at all, so a darwin artifact cross-compiled from a Linux runner would be killed on the target machine while every check here still passed. W19 therefore builds and runs each artifact on its own platform, and that is a requirement rather than a convenience. Cross-compiling a darwin target would need an explicit ad-hoc signing step, which needs no Apple certificate or account. Where the artifacts live is decided rather than open: they are committed under planning/bin, one file per target named by its uname pair, declared per target in planning/binaries.tsv and pinned by the checksum recorded there, with W115 owning the commit. An earlier version of this paragraph ended by calling that the one remaining distribution question while eight units already presumed an answer; adversarial finding AR-31 recorded it as AR-02 one level out, since cycle 1 had blocked on two units depending on a crate location no unit decided, that was fixed, and the artifacts' location was not.

§ 8.4
With no fallback, platform coverage is the risk that replaces the size limit. The prebuilt set must cover the platforms the skill claims to support, and the install must be explicit where it does not. A missing artifact is now a missing feature rather than a slower path, which is a sharper failure and easier to notice.

§ 8.5
Resolved: autoplay follows the served state, not the progress trackers, so it moves as the state does without re-reading plan documents. When several steps are active at once it presents one tab per active state, each showing that state, and the reader toggles between them; autoplay follows whichever tab is selected. An earlier version of this paragraph left the source and the multiple-active case as open questions.

§ 8.6
Decided: in complete mode autoplay has no subject and says so, rather than presenting a control that cannot move. Replaying the plan history as an animation is attractive and the data supports it, but it was not requested, and inventing a subject so that a control stays available is how a page comes to look busy while telling the reader nothing. An earlier version of this paragraph left this open while US-64 already graded it and W53 already asserted that autoplay exists in every mode; adversarial findings AR-08 and AR-24 recorded that a story cannot be an acceptance criterion for a question the plan calls open, and that offers no toggle is the direct negation of exists in every mode. The decision resolves it in favour of US-64: planning and implementing are the two modes with an active subject, and complete is the third state, where the honest answer is that there is nothing to follow.

## Environment facts

§ 9.1
Verification runs against a locally served overview on the loopback interface. Start it with the binary in serve mode, pointed at a plan directory with an explicit port, using the flag surface W101 fixes; the process prints the bound port before it accepts a request, which is what makes the port safe to connect to as soon as the line is read. For a static artifact, render to a file and serve that directory with any static server on the loopback host. The Chrome extension must hold site permission for that host before any browser story can pass. An earlier version of this paragraph said to start planning/scripts/overview-serve.sh with a --port; W79 deletes that wrapper, and adversarial finding AR-22 recorded that every browser verification unit reads this paragraph for how to bring the page up — W13, W26, W31, W36, W40, W45, W46, W63, W69 and W70 — so the stale instruction would have been followed rather than noticed.

§ 9.2
The fixtures are checked into the repository under planning/tests/fixtures/overview, and goal 15 owns building them. Two are frozen snapshots of real plans: the navigation fixture from xcore-paymentmethod-code-aliases, 12 goals, 82 work units, 100 steps and 59 findings with about 173 KB of state, and the size fixture from codegraph-bash-indexing-v2, 242 files and about 337 KB of state, which is the case that fails to render at all today. Six more carry the states no real plan happens to have: structural anomalies, evidence gaps, a completed plan, an approved plan with no steps, a fresh plan, and a plan with a damaged state document. An earlier version of this paragraph named the two live plans in the plans root as the fixtures; adversarial finding AR-10 recorded that a plans-root helper can change or delete them, so evidence recorded against them was not reproducible, and that about a dozen stories needed a state neither of them had.

§ 9.3
No authentication is involved: the overview is a local artifact served on loopback with no login route.

§ 9.4
Windows is a declared target. The prebuilt set is five artifacts, each produced by one CI job: x86_64-unknown-linux-musl and aarch64-unknown-linux-musl for Linux, x86_64-apple-darwin and aarch64-apple-darwin for macOS, and x86_64-pc-windows-msvc for Windows. msvc names the ABI flavour of the Windows target rather than a build service; the gnu flavour is not shipped because the runner default and the platform default are both msvc. Feasibility is already measured in a separate repository: a dependency-free Rust binary of this shape built and ran on ubuntu, macos and windows runners plus a static musl build, with the Windows leg invoked through PowerShell.

## Approach decisions

§ 10.1
One hash-routed artifact rather than a file per page. It satisfies static, served and deep-linked operation with one build, and keeps the contract that the renderer produces a single output file. A file per page would multiply the write surface inside user plan directories and break opening the artifact from disk.

§ 10.2
Rejected: a master-detail rail with a tree beside a detail pane. It reads relationships as a hierarchy, and the plan data is a graph. The chosen design makes each relationship a hop between pages so traversal follows edges rather than a tree.

§ 10.3
Rejected: keeping the renderer in bash and only fixing the argument passing with slurpfile or stdin. It removes the limit but leaves the per-key rebuild, the clipping layout and the placeholder rows, all of which the user has asked to be gone rather than repaired.

§ 10.4
The binary is the only renderer and server. The consequence is accepted and must be surfaced rather than hidden: a host with no prebuilt artifact for its platform has no plan overview at all, so the installer states that the overview is unavailable on that platform instead of installing a feature that cannot run. An earlier version of this plan kept the jq renderer as a degraded fallback and declared the binary a soft requirement; that was reversed on instruction, because two renderers can disagree and the fallback carried the size limit the plan exists to remove.

§ 10.5
An earlier version of this plan said the bash panel builders would stay and that only substitution and state parsing would move. That was reversed on instruction: infrastructure the previous implementation added is to be scrapped where it hinders, and the panel builders are the direct source of the clipping, the placeholder rows and the prose dump.

§ 10.6
Runtime dependency removal is verified by construction and by execution: the crate builds the renderer and server as one binary, the old interpreter requirements and implementations are deleted, and the install and served-artifact checks execute the binary without invoking any of those tools.

## UI classification

- UI affected: yes
- Rationale: The entire deliverable is a browsable visual surface: twelve page types, a dependency graph, an autoplay monitor, a motion system and a tier system that degrades it. Every claim the plan makes is about what a reader sees and can navigate to, so nothing here can be verified without driving a real browser.

## UI validation

- Required: yes
- Browser target: the overview served on the loopback host, driven with the Chrome MCP tools: click a relationship link, use keyboard navigation, toggle autoplay
- Story artifact: `ui-user-stories.md`

## Adversarial review

- Artifact: `adversarial-review.md`
- Status: ✅ approved
