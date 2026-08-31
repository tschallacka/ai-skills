# Verification: 09-step-marker-exemption

## Automated tests

§ 2.1
Run the marker gate and confirm it passes with the artifacts present. Remove the new exemption arm and confirm it fails naming the artifact. Plant a marker-less ordinary script and confirm it still fails. Then list every exemption arm's pattern against the tracked file list and confirm none matches nothing, which is what catches the next arm that outlives its files. The crate-source and stylesheet mutations are not run here: those files do not exist at this point, and the unit that owns that proof runs after they do.