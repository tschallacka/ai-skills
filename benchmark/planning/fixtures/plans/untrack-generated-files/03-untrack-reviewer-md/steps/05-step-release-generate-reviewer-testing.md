# Verification: 05-step-release-generate-reviewer

## Automated tests

§ 2.1
Run installer/build-release.sh without REVIEWER.md: it is generated before collect and its pinned hash matches the packaged SKILL.md; with the file present, mtime is unchanged.