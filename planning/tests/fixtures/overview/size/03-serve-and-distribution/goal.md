# Goal: The binary serves, and its absence degrades honestly

## Current state and prior-goal handoffs

§ 2.1
Confirmed: four runtime rungs exist under planning/scripts/runtime for serving the overview, each duplicating render-and-slice logic, and the jq renderer is still present. The user directed that the jq fallback be scrapped entirely rather than kept as a rung. Depends on goal 01 and 02 for something to serve.

## Outcome and definition of done

§ 3.1
The Rust binary compiles from the pinned toolchain, serves the artifact itself and is the only renderer: the four runtime rungs, their duplicated render-and-slice logic, and render-plan-overview.sh are deleted. The running binary requires none of Bash, jq, Python, Node, Perl, socat, or the overview shell scripts. Where no prebuilt artifact matches the host, the installer states that the plan overview is unavailable on this platform rather than installing a renderer that cannot run. Demonstrated by compiling and serving with the binary, by a build that produces artifacts for the declared platforms, and by an install on a platform with no artifact reporting the feature as unavailable with a clear message rather than silence.

## Why this goal is needed

§ 4.1
Two implementations of one feature diverge, and the four rungs already differ in behaviour. Deleting them and serving from the binary makes the served page and the file-opened page the same artifact by construction rather than by discipline.

## Scope

§ 5.1
In scope: serving from the binary, deleting the four runtimes and the jq renderer, the platform artifact matrix, the manifest change, and the message shown where no artifact matches the host. Out of scope: the live change stream over that connection, which goal 10 owns.

## Affected files, systems, data, and interfaces

§ 6.1
planning/scripts/runtime and render-plan-overview.sh are removed; requires.tsv loses the runtime rows; the installer manifest and the generated install.sh change; a CI workflow builds and exercises one artifact per declared platform.

## Dependencies and handoffs

§ 7.1
Depends on goals 01 and 02. Hands to goal 10 the connection the change stream rides on, and hands to goal 09 the artifact matrix run to confirm green.

## Implementation approach, risks, and edge cases

§ 8.1
Approach: bind loopback only and print the bound port before accepting, so a caller never races the socket. Risk: removing the runtimes strands a platform with no prebuilt artifact, so the installer states the feature is unavailable rather than installing a renderer that cannot run. Edge case: a partially installed renderer must not remain after that path, and Windows is a declared target, so the matrix invokes the artifact under PowerShell rather than assuming a shell.

## Owned work units

§ 9.1
`W14` — Serve the artifact and the live state from the binary on the loopback interface, printing the bound port. One implementation replaces four runtimes; no HTML is sliced with a regex.

§ 9.2
`W15` — Delete the jq renderer. It is superseded entirely: its token substitution, its argument passing and its fixed-column output are all replaced, and keeping it would leave a second implementation that can disagree.

§ 9.3
`W16` — Delete the four server rungs and the shared handler. Their render-and-slice logic exists three times over and carries both the BrokenPipeError and the sections-endpoint regex.

§ 9.4
`W17` — Add only the planning-arm artifact entries and remove only the deleted renderer entries from installer/src/50-manifest.sh; W114 owns the dev and prod registration arms in the same file.

§ 9.5
`W18` — Own only the unavailable-platform notice branch in installer/src/20-runtime-tools.sh; W104 owns artifact selection and placement in the same file.

§ 9.6
`W19` — Build the five declared artifacts, one job per triple: linux musl x86_64 and aarch64, darwin x86_64 and aarch64, and windows msvc x86_64. Each job runs the binary once to prove the artifact executes.

§ 9.7
`W20` — Install the planning skill on a host whose platform has no artifact and confirm the stated unavailability, a non-zero exit or an explicit warning, and no partially installed renderer.

§ 9.8
`W75` — Discover and run the crate test suite alongside the shell tests, so a failing cargo test fails the repository gate. Today discovery is a single find for test-star.sh at line 87, which means no Rust test can ever redden the suite however many the plan declares.

§ 9.9
`W76` — Add the exemption arm the shipped prebuilt artifacts need, with a stated reason, and remove the now-dead arm exempting the deleted runtime directory. A compiled file carries no comment syntax, so it can hold no marker; a stale arm exempting paths that no longer exist hides the next real violation. The Cargo.lock arm already exists and is not added here. The crate sources and the stylesheets are not exempt: they carry markers, read by the forms W109 makes available.

§ 9.10
`W109` — Read a CSS block-comment marker form so a stylesheet can declare its audience. The reader accepts a hash comment, an HTML comment and a double-slash comment; none is valid CSS, so the three stylesheets the crate embeds can hold no readable marker and would fail the gate with no way to pass it.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | Serving, four deletions, the shipped file list and the artifact matrix all change what a user receives; each is verified by a gate, a real install, or an executed artifact. |

## Goal-size exception
