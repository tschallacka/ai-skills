# Verification: 03-step-remove-runtimes

## Automated tests

§ 2.1
Confirm the runtime directory is gone and that no shipped script resolves a rung name. Run the suite and record which failures are the expected consequence of requirement rows this step no longer removes, distinguishing them from any failure this step caused. The probe-versus-declaration gate is not run here: it belongs to the later unit that removes those declarations, because they are still present at this point. An earlier version of this companion asserted the rows were already absent.