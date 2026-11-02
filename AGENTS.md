# Glint Blade agent instructions

## Static scenery: ground-contact outlines

Kyle's 2026-11-02 visual direction: preserve top and side outlines, but remove
the dark bottom outline where a static object meets the landscape. A column
should keep its original solid stone volume while its contact pixels continue
into the ground without a separate ink border.

- Prefer correcting contact ink over adding bases, plinths, sockets, shoulders,
  or floor beds to disguise it. Reassess staged replacement shapes against this
  direction; do not discard useful earlier corrections or other owners' work.
- Identify the actual authored ground-support contour, not the PNG's last row
  or every column's lowest opaque pixel. Arch undersides, overhangs, cavity edges,
  raised tiers, and cliff edges are not automatically ground contacts.
- Replace only dark contact ink with appropriate adjacent material pixels.
  Preserve native pixel edges and the rest of the outline. Keep alpha, anchors,
  occupied support/collision, emission, and normals unchanged unless a separate
  correction is justified. This instruction concerns static scenery, not actor
  silhouette outlines.
- Check runtime contact painting too: Bloomwild base contact and pillar moss,
  Summit contact AO, and the distributor apron can recreate a dark underline.
  Retain physically coherent cast shadows; avoid a uniform replacement border.
- Audit actual native gameplay views and integer-enlarged contact crops under
  representative lighting. Compare original art, staged geometry work, and the
  contact-ink correction. Success must be visible at normal gameplay scale.
- Keep edits in scoped asset sources/exporters and agreed ownership. Do not use
  an umbrella exporter or reopen completed threads to apply this direction.

## Thread coordination

Kyle's 2026-11-02 instruction applies to currently running work and future work.

- Settled, archived, snoozed, completed, cancelled, interrupted, failed, and idle
  threads are not coordination recipients. Read their history and artifacts
  without sending messages. Never reopen them for an FYI, acknowledgment, audit
  finding, install notice, or completion handoff.
- A task-level COMPLETE or final handoff closes that owner's task, even if
  subsequent coordination traffic makes its UI status appear active. Only a new
  direct request from Kyle reopens it. SOURCE STABLE while requested verification
  is still running does not mean the whole task is complete.
- Immediately before every message, refresh thread metadata. Require
  `settled: false`, an actually running task, and no later completion handoff.
  Do not reuse a launch-time roster or infer liveness from a thread's title.
  Skip delivery when the current lifecycle cannot be established.
- Use `steer` only for verified live work. It is not a no-revival guard by itself.
  Do not use `auto`, `queue`, `restart`, or `unsettle` for routine coordination.
- The Glasswater cavern/ravine task is closed. Do not send it further messages
  unless Kyle explicitly requests new work there.
- Send only actionable ownership conflicts, shared-file changes, blockers, or
  stable install boundaries to directly affected live owners. Avoid project-wide
  FYI broadcasts, courtesy replies, repeated status reports, and acknowledgment
  chains. Do not request acknowledgments unless work genuinely depends on one.
- Apply these rules to active internal workers and cancel any planned messages
  to closed peers. If the old owner is closed, route an unresolved issue to a
  relevant live owner or the live coordinator instead.
- Finish with durable evidence: file/asset scope, source and asset snapshot,
  verification results, known remaining issues, and artifact paths. Send at most
  one necessary final handoff to the coordinator, after verifying it is live,
  then stop coordination. Do not start a new turn solely to report completion.

## Concurrent implementation

- Agree on exact shared-file or asset ownership before overlapping edits.
- Preserve other owners' dirty changes and install complete sprite/metadata/
  normal batches. Do not run an umbrella exporter over another owner's install.
- Use private build targets, frozen renderers, and distinct audit output paths.
  Build against the intended asset snapshot explicitly; avoid mixed staged and
  active assets when interpreting failures.
- An independent audit does not reopen finished tasks or authorize edits to
  their files. Report current defects with reproducible evidence to a live owner.
