# Verification

Test valid manifests, missing files, malformed assignments, command substitutions, symlink escapes, wrong roots, weak permissions, unexpected keys, and a trusted temporary script that sources valid manifests. Also inspect the applicable planning/helper scripts and verify each either consumes the shared variables or has a documented standalone exception. Pass only when valid inputs load the expected variables, every unsafe case fails before execution, and no applicable script needlessly repeats canonical path derivation.
