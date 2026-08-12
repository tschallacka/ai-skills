# Brainstorm: limiting duplicated planning context v27

## One authority model

V27 is the sole replacement design; backward compatibility is forbidden.

The package root is `planning/context-v27/`. Its signed `ROOT` record is the
only authority for package, plan, lease, recovery, and fixture state. The root
record contains contract digest, manifest digest, closure digest, external
boot-fencing digest, authoritative snapshot, journal head, writer epoch, and
state digest. Every event, lease, capability, envelope, benchmark record, and
oracle contains the root digest and package ID.

`RECOVERY_DECISION` is a normal hash-chained journal event. It is not a second
authority. Recovery may create only `RECOVERY_DECISION`, missing publication
events for one named transaction, and a new ROOT record. No other event or
artifact may be invented during recovery.

The external boot store is a fencing witness, not an authority. Its canonical
record is `{package_id, writer_epoch, boot_id, latch_state, root_digest,
sequence}`. `latch_state` is one of `PREPARED`, `COMMITTED`, or `ABORTED`.
Only the writer store may establish a committed ROOT, snapshot, journal head,
lease, nonce ledger, or state digest. A `COMMITTED` external record is valid
only when its package ID, epoch, and root digest equal the signed writer-store
ROOT and its sequence is the latest accepted sequence. A `PREPARED` record is
never readable state and never authorizes a writer; it names only a pending
ROOT transition. `ABORTED` records are retained as fencing history and cannot
be reused.

Every ROOT transition uses this conditional two-phase protocol while holding
the writer lock. First, the writer verifies the current signed ROOT and the
external record, then conditionally writes `PREPARED` with the expected prior
sequence, prior root digest, target writer epoch, target root digest, and a
strictly greater sequence. The conditional write succeeds only if the
external package ID, prior epoch, prior root digest, and sequence still match
the values read by the writer. Second, the writer durably commits the complete
writer-store transaction, including the target ROOT and a commit marker that
contains the prepared record digest. Third, it conditionally changes that
exact prepared record to `COMMITTED`. The commit marker and the final external
record are both required before startup leaves the transition state.

The admissibility invariant is: `COMMITTED.sequence` is strictly monotonic,
`COMMITTED.writer_epoch` equals `ROOT.writer_epoch`,
`COMMITTED.root_digest` equals the signed committed ROOT digest, and the ROOT
contains the digest of that exact external record. No ROOT transition is
accepted without this invariant. A crash before `PREPARED` leaves the old
state unchanged; after `PREPARED` but before the writer commit, startup must
verify the absence of the matching commit marker and conditionally write
`ABORTED`; after the writer commit but before external `COMMITTED`, startup
must verify the matching commit marker and conditionally finalize `COMMITTED`.
Any mismatch, missing record, sequence rollback, epoch rollback, duplicate
boot ID with a different payload, unknown durability, or failed conditional
operation leaves the package mutation-blocked and `recovery-required`.

At startup, the writer store is authoritative for the last committed ROOT,
but mutation is permitted only after the external `COMMITTED` witness is
verified against it. During an external-store outage, no write, lease change,
nonce consumption, repair, or ROOT change is permitted. A reader may serve
only the last locally verified committed snapshot, without claiming a newer
root or state, when no unresolved prepared transition or recovery condition
exists; otherwise it returns no state and remains recovery-blocked. Readers
must never serve a prepared, aborted, unverified, or rollback-paired state.

## Recovery state machine

```text
normal
 -> recovery-required (unknown flush, corruption, gap, conflict)
 -> evidence-built (all existing records verified)
 -> operator-approved (signed decision names one transaction/root)
 -> repaired (missing publication events created for that transaction)
 -> normal (new ROOT durably committed)
```

If existing records cannot verify, recovery remains required. If the external
boot-fencing store is unavailable, startup remains mutation-blocked and no
ROOT may change. The external store contains signed `{package_id, writer_epoch,
boot_id, latch_state, root_digest, sequence}` and uses the same root trust key;
rollback, replay, and unknown durability fail closed.

Recovery may be entered only by a verified startup/reconciliation process when
the local journal, ROOT, snapshot, pointer, nonce ledger, lease state, or
external fencing witness has an unknown flush, corruption, gap, conflict, or
unfinalized two-phase transition. The process records the observed evidence as
canonical bytes and computes one `evidence_digest`; it must not repair from an
unverified or partially selected record set. The signed `RECOVERY_DECISION`
binds `decision_id`, package ID, target transaction ID, evidence digest, prior
ROOT digest, prior journal head, exact intent digest, ordered event IDs and
canonical event-body digests, candidate snapshot and pointer digests, target
state digest, resulting ROOT digest, target external sequence/epoch, operator
identity and key fingerprint, authorization scope, and the admissibility
result. The decision also names whether each missing publication event is
derived from the verified intent and gives its complete canonical body; no
field may be inferred during repair.

The evidence digest covers the sorted, duplicate-free set of verified record
digests, verification errors, external-store observations, and crash-boundary
observation. The decision ID and the tuple of prior head, transaction ID,
evidence digest, ordered event digests, candidate snapshot digest, and target
ROOT digest are unique. A second decision with the same decision ID but any
different bound value, or two admissible decisions for the same prior head and
transaction, is a conflict and leaves recovery required. Operator approval is
valid only when the signer is authorized for the package and recovery scope,
the signature covers the complete decision bytes, and the approval has not
already been consumed.

Admissibility requires: all referenced existing records verify under the prior
ROOT; the intent belongs to the named transaction; event order extends the
prior journal head without a gap or duplicate; every missing event is the
deterministic publication of that intent; candidate snapshot/pointer/state
digests agree; no lease, epoch, nonce, or capability rule is violated; the
resulting ROOT is the sole hash-chain successor; and its external fencing
record is the exact conditional successor required by the authority protocol.

Under the writer lock, the repair durably appends the signed decision and the
named missing publication events as one writer-store transaction, then commits
the resulting ROOT through the authority protocol above. The recovery commit
point is the durable target ROOT plus its matching external `COMMITTED` record.
Before that point every restart remains `recovery-required`; after it, the
new ROOT, snapshot, journal head, and fencing witness are the only admissible
state. A crash during repair therefore either deterministically resumes the
same bound transition or fails closed; it cannot authorize a second repair.

## Lease-event-binding: transaction-replay

The protected writer store is the one transaction participant. It owns ROOT,
journal, leases, nonce ledger, snapshots, pointers, and latches. A transaction
identity is the digest, in the `tx-id` domain, of canonical fields
`{package_id, transaction_id, command_digest, lease_id, writer_epoch,
fencing_token, nonce, prior_root_digest, prior_journal_head,
target_snapshot_digest}`. The same identity must be used in the intent,
publication, commit, and recovery records. A changed command, lease, epoch,
token, nonce, prior head, or target is a different transaction, never a variant
of an idempotent retry.

The writer lock is the commit serialization point. Immediately before any
mutation it verifies the committed ROOT and matching external `COMMITTED`
witness, the lease/owner/boot/epoch/token, capability signature and command
digest, nonce state, prior journal head, and all target digests. It then applies
this order:

1. Reserve the nonce and durably append `INTENT` containing the complete
   transaction identity, prior ROOT/head, command, capability, and target
   digests. A reserved nonce is not reusable, even if the transaction later
   aborts.
2. Build publication event bodies, candidate snapshot, and pointer in private
   staging. Their canonical bytes and digests are fixed before any publication
   becomes visible. The staged values include the exact next journal head and
   target ROOT digest.
3. Conditionally prepare the external fencing successor as specified by the
   authority protocol. The writer store then durably commits one transaction
   containing the consumed nonce, all lifecycle and publication events in
   order, the candidate snapshot and pointer, the new journal head, the new
   ROOT, and a `COMMIT` marker containing the transaction identity, every
   component digest, and the prepared external-record digest.
4. Conditionally finalize the prepared external record as `COMMITTED`. Only
   after this succeeds may normal mutation resume; readers validate this same
   pair before serving the new snapshot.

`INTENT` is the only durable state before the commit transaction. A crash
before intent flush consumes nothing visible and leaves the old ROOT. A crash
after intent flush but before external `PREPARED` leaves a replayable intent;
startup either retries the exact identity or marks the nonce abandoned, and
never allocates it to another transaction. A crash after `PREPARED` is
resolved by the authority protocol: absence of the matching commit marker
aborts the prepared fence, while its presence finalizes the exact commit. A
crash after writer-store commit but before external `COMMITTED` blocks
mutation and hides the new snapshot until finalization. A crash after external
commit resumes normally only if every digest and ROOT invariant verifies.

Replay scans the journal from the verified head and indexes complete canonical
transaction identities. An existing `COMMIT` with the same identity and
component digests is an idempotent success and returns its recorded outcome;
an existing identity with any differing body or digest is a conflict requiring
recovery. An `INTENT` without a matching commit is never published: it is
replayed only through the fixed identity and staged digests, or remains
recovery-required when the required evidence is unavailable. No snapshot,
pointer, publication event, or reader response may expose a transaction until
its `COMMIT`, ROOT, and external `COMMITTED` witness all verify.

## Lease-event-binding: lifecycle-invariants

There is at most one active lease per package and writer epoch. A lease key is
`{package_id, lease_id}` and its immutable owner, boot ID, capability scope,
and acquisition transaction are recorded in the journal. A new boot must use a
unique boot ID and an epoch strictly greater than the last committed epoch;
the external fencing sequence and ROOT must satisfy the authority invariant
before the lease can become active. A fencing token is unique within an epoch
and strictly increases for every accepted lease acquisition or renewal. Zero,
reused, skipped-backward, or ambiguous tokens are rejected.

Lease acquisition requires no active lease, a verified current ROOT, a valid
boot-fence witness, and a capability authorizing the requested command scope.
Renewal requires the exact lease ID, owner, boot ID, epoch, current token,
expected journal head, and an unexpired lease; it issues one greater token and
extends expiry by the bounded contract interval. Expiry is determined by the
canonical monotonic clock value recorded with the lease. An expired lease
cannot renew or mutate. Revocation is a durable lifecycle event that names the
lease, prior token, reason, and revocation transaction; it is terminal and
cannot be undone by replay or a later message.

Every acquisition, renewal, expiry observation that changes state, revocation,
restart fence, and lease release is journaled in the same writer transaction
as its nonce, journal-head, and ROOT update. Lifecycle events carry lease ID,
owner, boot ID, epoch, token, nonce, transaction ID, prior ROOT/head, resulting
ROOT/head, and event digest. A capability signature covers package ID, command
digest, lease ID, boot ID, epoch, token, nonce, transaction ID, and expiry
bound; a capability for one command, target, lease, or transaction cannot be
replayed for another.

The writer rejects every mutation whose lease is missing, duplicated, expired,
revoked, owner/boot-mismatched, epoch-stale, token-stale, nonce-reused, or
capability-mismatched. It also rejects a validly signed request whose expected
ROOT or journal head is stale. After restart, the old boot's leases are stale
until explicitly proven against the committed ROOT; the new boot must acquire
a new epoch and lease. A duplicate identical lifecycle transaction is
idempotent, while a conflicting lifecycle body or two active leases is a
recovery condition, never a last-writer-wins choice.

## Closed wire and error model

Every request is a canonical object `{schema, package_id, request_id, command,
arguments, capability_or_absent}`. The command grammar requires one known
command followed by sorted unique flags, each flag at most once, with no
positional identifiers. Unknown, duplicate, missing, malformed, or
non-canonical arguments are rejected before authorization or state lookup.
Unknown object fields, duplicate JSON keys, null, invalid UTF-8, trailing
bytes, invalid digest syntax, and schema values other than `27` are contract
invalid. A request whose framing cannot be parsed returns usage; a parsed
request with invalid contract fields returns contract-invalid. Neither path
mutates state or consumes a nonce.

Every response is exactly the canonical object
`{schema, package_id, root_digest, command, request_id, status,
data_or_absent, error_or_absent, output_digest}`. `status` is one of
`success`, `blocked`, `stale`, `suspect`, or `failure`. A successful response
contains typed `data_or_absent` and omits `error_or_absent`; every other status
omits data and contains one error. Optional values are absent by omission,
never JSON null. `root_digest` is the verified committed ROOT used for the
decision, or the last locally verified ROOT for a read-only outage response.
The output digest is SHA-256 in the `response` domain over the canonical
response with only `output_digest` omitted; stderr is never included.

Every error is exactly `{code, subcode, retryable, state, root_digest,
snapshot_or_absent, details, journal_event_or_absent}`. `details` is
`{items: sorted text[], expected_or_absent, current_or_absent}` with omitted
optional fields. The error code set and exit mapping are closed by the v27
contract: contract-invalid 12, authorization 14, recovery 17, lease 15,
platform 16, suspect 11, stale 10, conflict 13, invalid 12, budget 19,
usage 21, not-found 20, internal 22, legacy-rejected 78, and
replacement-required 23. The response status is derived from the selected
error (`blocked` for recovery/authorization/lease/platform, `stale` for stale,
`suspect` for suspect, and `failure` for all remaining errors).

Error selection evaluates all applicable conditions, assigns each its contract
precedence rank, and returns the lowest-ranked condition. The implementation
must evaluate request syntax and schema first, then authorization, recovery
and fencing state, lease/capability, platform, trust/suspect, expected-root
staleness, transaction conflict, command/input validity, budget, usage,
not-found, and internal conditions in that order. It must not expose a lower
ranked condition when a higher-ranked condition is present, and equal-ranked
conditions are ordered by the contract's stable subcode and sorted detail
items. Missing external fencing always selects recovery, emits no data, and
forbids mutation, lease changes, nonce consumption, repair, and ROOT changes.

During a verified recovery-required or unresolved two-phase state, reads
return recovery with no snapshot/data. During a fencing-store outage, a
read-only command may return only the last locally verified committed snapshot
when no unresolved transition exists; it is marked `suspect`, contains the
last verified ROOT, and cannot be used as a mutation precondition. A prepared,
aborted, rollback-paired, or otherwise unverified state is never served.

## Legacy rejection

Legacy detection occurs before package-root selection, signature verification,
fencing, lease acquisition, nonce allocation, or any writer-store mutation.
The input is legacy when it contains `context_schema_version=1`,
`context_result_schema_version=1`, a `planning-context-v1` or
`planning-context-v26` package marker, an unsigned or rootless package, or an
old command/schema marker listed by the v27 contract. Detection is based on
the raw package marker and does not infer a v27 default from an absent ROOT.

Every detected legacy input returns one canonical response with status
`failure`, error code `legacy-rejected`, subcode identifying the marker class,
state `legacy-rejected`, no data, `retryable=false`, no journal event, and
exit 78. The response output digest uses the normal v27 response domain and
the input is not allowed to consume a nonce, create a lease, write a journal
event, update a snapshot or pointer, or change ROOT. A package that cannot be
classified because its framing is unreadable follows the usage/contract-invalid
rules instead; it is never treated as v27.

No adapter, translator, migration mode, compatibility flag, inferred schema,
or legacy repair path exists. The four legacy fixture cases are
`LEGACY-V1`, `LEGACY-UNSIGNED`, `LEGACY-ROOTLESS`, and `LEGACY-OLDSCHEMA`;
each maps bijectively to one oracle and expected-outcome record and is required
to prove the no-mutation rule.

## Package, benchmark, and oracle closure

The signed ROOT manifest lists the exact contract, canonicalizer, trust,
schemas, closure graph, fixture bodies, platform records, and external-store
record digests. No excluded object exists; self-reference is avoided by hashing
the ROOT envelope without only its root digest field.

The benchmark manifest lists the five category IDs, exact invocation IDs,
ten repetitions, pair IDs, baseline/candidate helper digests, model/provider/
tokenizer/platform/cache/settings digests, token accounting, thresholds, and
invalid-data behavior. The oracle schema lists every expected exit/state/root/
snapshot value, all safety/correctness/completeness/review/recovery assertions,
transcript digest, signer key, and signature domain.

## Acceptance

V27 is implementable when the single signed ROOT package contains these closed
records and fixtures, and all recovery, writer, replay, platform, benchmark,
oracle, and legacy-rejection tests pass. No implementation output is required
to approve the design.
