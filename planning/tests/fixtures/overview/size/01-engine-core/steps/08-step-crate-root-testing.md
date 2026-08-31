# Verification: 08-step-crate-root

## Automated tests

§ 2.1
Build the crate offline from a clean checkout and confirm it compiles with no dependencies resolved; record the reported dependency count. Then add a source file under the crate without declaring it and confirm the build fails or reports it unreachable, rather than compiling clean while the file is ignored. Run the marker gate over the new file.