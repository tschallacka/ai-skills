# Verification: 06-step-release-build-libs

## Automated tests

§ 2.1
Run installer/build-release.sh on a tree without libs: succeeds and the tarball contains them; remove a non-generated listed file and expect the hard error message.