# Progress: 05-update-consumers

**Progress:** `100%  ####################  100%` ✅

| Goalname | Stepname | Description | Completion status |
|---|---|---|---|
| 05-update-consumers | 01-step-install-sh | Update the installer SOURCE — installer/src/50-manifest.sh (and installer/src/05-config.sh) — so... | ✅ completed |
| 05-update-consumers | 02-step-requires-tsv | Rewrite chat/requires.tsv: remove the bash-hard row and the python3/node/perl/socat soft server-runt... | ✅ completed |
| 05-update-consumers | 03-step-skill-doc | Rewrite chat/SKILL.md: describe the rust server start, rust client send/read-delta/tail/discover com... | ✅ completed |
| 05-update-consumers | 04-step-readme | Rewrite chat/docs/README.md to present the rust server + rust client (build, run, discover, send/rea... | ✅ completed |
| 05-update-consumers | 05-step-root-readme | Update the README.md skills-table row for Chat to describe the rust server + rust client, removing t... | ✅ completed |
| 05-update-consumers | 06-step-package-json | Update package.json files entry for chat: include the released chat/bin/* binaries (built by the rel... | ✅ completed |
| 05-update-consumers | 07-step-rewrite-test | Rewrite chat/tests/test-chat.sh to drive only the rust server and rust client: run under `nix develo... | ✅ completed |
| 05-update-consumers | 09-step-portability-fix | Update tests/test-portability-contract.sh allowlist rows that reference the now-deleted chat scripts... | ✅ completed |
| 05-update-consumers | 10-step-ship-bin | Update the release build so it SYNTHESIZES the built rust binaries into the release tarball rather t... | ✅ completed |
