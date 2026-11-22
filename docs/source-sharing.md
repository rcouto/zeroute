# Source-first publication

## Current state

Public source is available at [rcouto/zeroute](https://github.com/rcouto/zeroute).
Gitea remains the private development source and Buildbot remains CI. GitHub
accepts public contribution PRs; see [CONTRIBUTING.md](../CONTRIBUTING.md).
Contributors need no access to private services.

The initial publication retained existing `main` history and author metadata.
Public updates currently use the required maintainer step in
[repository steering](../AGENTS.md#publish-github-after-an-approved-gitea-merge):
after an approved Gitea main merge, push main to GitHub without force and verify
matching commits.
No continuous push mirror, automatic PR import or GitHub CI is configured; a
failed publication must be reported separately from a successful private merge.
Maintainers review selected public contributions privately, report sanitized
validation results on the public PR, merge in Gitea, and publish accepted commits
back to GitHub before closing the original PR with public commit links.

Source sharing is separate from distributing APKs. The project is experimental,
maintained for personal use, with no support or compatibility commitment.

## Continuous synchronization, not activated

Manage Gitea repository settings and native push mirrors through Gitea
UI or supported `tea` API, not OpenTofu. Native synchronization is blocked by
[upstream NAT64 rejection](https://github.com/go-gitea/gitea/issues/40142).
Review and approve the settings before enabling one-way updates to GitHub.
Native Gitea push mirrors copy branches and tags and can force-update the
destination; the reviewed API exposes no main-only filter. Store the scoped
GitHub credential through Gitea mirror authorization, outside source and logs.
Do not create a mirror merely to leave its timer off: creation itself can trigger
publication.

Review the files and history included by the selected scope, including future
branches/tags, for identities, credentials and diagnostic artifacts. Keep
credentials outside Git and Nix outputs. Use Gitea's native mechanism rather
than adding a separate synchronization engine or another CI compilation job.

A source mirror alone would not import public PRs, expose private CI to public
branches, report checks on GitHub, or synchronize PR discussion and closure.
Those behaviors require separate design and authorization. The current manual
contribution workflow remains the policy until a replacement is documented.

## APK distribution, deferred

Before public APK distribution, decide stable versioning/application identity,
signing-key custody and recovery, upgrade compatibility, the update channel,
complete third-party notices and corresponding-source information, and release
verification. Existing test installs use a development key; a differently signed
release cannot be assumed to update them in place. No public APK, signing job,
GitHub Release or app-store submission is part of source publication.

Application-owned code is MIT; the pinned upstream ZeroTier core is separately
MPL-2.0. Other bundled components retain their respective licenses. Source
sharing does not imply upstream sponsorship or product support.

## Cost boundary

This documentation adds no CI job, dependency or build target. Proposed continuous
source synchronization would add Git transfer, with unmeasured timings, rather
than Android compilation. Discuss any added build jobs, architectures or tooling
cost before implementing automation.

See [CI and publication](ci.md) and [architecture](architecture.md).
