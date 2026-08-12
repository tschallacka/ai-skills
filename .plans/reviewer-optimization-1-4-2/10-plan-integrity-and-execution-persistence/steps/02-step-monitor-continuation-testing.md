# Verification

Exercise fixtures for a live subprocess that emits a status-only message before completion, progress after one steering action, a true nonzero failure, retry-budget exhaustion, and interruption with descendant cleanup. Pass only when status-only output does not terminate the run, steering remains bounded and evidence-backed, terminal states are classified correctly, and failed or interrupted fixtures are never reported as successful.
