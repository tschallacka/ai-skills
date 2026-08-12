# Verification

Create a temporary isolated plans root and run the normal plan-creation flow twice. Verify both manifests exist, contain only the required quoted assignments, use canonical paths, have mode `600`, remain byte-stable on refresh, and do not remove unrelated files. Verify no secret or arbitrary inherited variable is emitted.
