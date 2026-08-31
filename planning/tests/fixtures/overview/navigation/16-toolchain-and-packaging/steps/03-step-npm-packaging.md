# Step: 03-step-npm-packaging

## Ownership

- Goal: `16-toolchain-and-packaging`
- Work unit: `W105`
- Type: `config`

## Change target

- File: package.json and planning/tests/fixtures/overview/npm-package-baseline.tsv
- Primary symbol or file scope: files field and npm package baseline fixture
- Subscope: `N/A`

## Objective

§ 4.1
Include all five target artifacts and install.sh in the npm package. W105 owns the single repository-owned machine-readable baseline at planning/tests/fixtures/overview/npm-package-baseline.tsv. The baseline uses the fixed TSV schema package_path<TAB>byte_size with a header row and one final tarball_bytes<TAB>byte_size row; package_path rows are sorted lexicographically and include every file emitted by npm pack. W105 generates or refreshes this file immediately after npm pack and before W120 consumes it. The exact consumer lifecycle is npm pack followed by clean extraction and invocation of bash install.sh from the extracted package root; Windows uses the same install.sh under the supported Bash environment and runs the selected .exe from PowerShell.

## Instructions

§ 5.1
Include all five target artifacts and install.sh in package.json. Generate or refresh planning/tests/fixtures/overview/npm-package-baseline.tsv from the npm pack output as W105's owned record. Write the fixed header package_path<TAB>byte_size then one lexicographically sorted row for each tarball path with its byte size then the final tarball_bytes<TAB>byte_size row. Record the exact sorted file list and byte size only in this file; do not duplicate the expectation in package.json or W120. The exact consumer lifecycle is npm pack followed by clean extraction and invocation of bash install.sh from the extracted package root. W104 test mode supplies deterministic platform inputs in the package test; Windows runs the selected .exe from PowerShell.

## Acceptance criteria

§ 6.1
The packed tarball contains all five declared artifacts and install.sh. The W105-owned baseline exists at planning/tests/fixtures/overview/npm-package-baseline.tsv with the fixed header and schema; its package paths are sorted and complete and its final tarball_bytes row matches the npm pack byte size. Regenerating the baseline from the same package produces no unexplained change. Running the extracted install.sh from the package root selects one runnable artifact on each supported host including the Windows .exe path and reaches the explicit unavailable notice on an unsupported host.

## Handoff

§ 7.1
W120 reads planning/tests/fixtures/overview/npm-package-baseline.tsv as its only package file-list and byte-size expectation. W106 and goal 09 receive a reproducible package baseline from this owned record rather than an undocumented value.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
