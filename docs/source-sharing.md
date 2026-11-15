# Source-first publication

Status: agreed publication sequence; this review does not create a public remote
or enable synchronization. Stage one shares source; stage two, distributing APKs,
is a later decision. This is independent of the proposed connection UX work.

## Stage one: public source

Gitea remains the working repository and Buildbot remains CI. The intended public
surface is a GitHub repository synchronized from Gitea, with source, license,
build instructions and an honest experimental/support statement. No Android
Developer account or release signing key is needed merely to share Git source.

Before enabling synchronization, prepare a reviewable publication configuration:

- Identify the GitHub owner, repository name and visibility; retain Gitea as the
  authoritative write/PR destination and explain that in the public README.
- Review tracked files and the history actually being shared for credentials,
  identity state, keystores, private logs/device inventories and generated APKs.
  Public-source review includes branches/tags selected by the mirror, not just
  the latest tree. Do not publish private diagnostic artifacts or local test keys.
- Verify application and bundled-dependency license notices against pinned
  sources. The current application is MIT; bundled upstream core licensing is
  distinct. Do not imply upstream sponsorship or product support.
- Document private-network scope, Android permission use, current limitations,
  reproducible build steps, and how to report issues without posting identities,
  credentials or packet contents. Device acceptance is not a reliability promise.
- Choose synchronization direction, branch/tag scope and failure visibility.
  Prefer a native repository push mirror over another CI build job; credentials
  stay in the service credential store, never repository files or Nix outputs.
  Prepare source-hosting protection and review settings before making it public.
- Keep public source synchronization separate from CI artifacts, APK releases,
  signing, app-store registration and installation. Source sharing alone neither
  signs nor updates installed applications.

The UI review needs no remote operations. Actual repository creation/mirror
activation must use the owner's approved destination and publication scope.
If infrastructure reconciliation is managed by the infrastructure repository,
follow its Terranix/OpenTofu ownership rules rather than inventing a new sync job.

## Stage two: APK distribution, deferred

Decide stable application identity/versioning, signing-key custody and recovery,
upgrade compatibility with existing test installations, download/update channel,
privacy/license material and release verification before distributing APKs.
Existing phones use a development signing key; do not assume a differently signed
release can update them without migration. No key rotation, new signing job,
GitHub Release, store submission or public APK is part of stage one.

## Cost and evidence

This documentation changes no CI work. A source push mirror is expected to need
an initial Git transfer plus incremental syncs, not Android compilation; transfer
size, service behavior and timings have not been measured. No extra build/cache
cost is approved by this proposal. If the selected integration adds CI jobs or
builds, agree on their costs and cache invalidation before implementation.

See [CI and publication](ci.md), [connection state](state-machine.md) and
[architecture](architecture.md).
