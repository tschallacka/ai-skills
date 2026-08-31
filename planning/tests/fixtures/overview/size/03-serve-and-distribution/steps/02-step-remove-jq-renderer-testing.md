# Verification: 02-step-remove-jq-renderer

## Automated tests

§ 2.1
Confirm the renderer file is absent and that the binary produces the page for the fixture the renderer used to handle. Then enumerate the files that still name the removed script and check each against the inventory: every one must be the change target of a later unit in the removal goals, which own the manifest, the map, the contract, the documentation and the five tests. A hit in a file no inventory row owns is the failure this step reports, and it is reported rather than fixed here. An earlier version of this companion treated any remaining reference as a failure of this step; that wording predates the ownership the removal goals now carry.