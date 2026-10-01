# Portable route creations and kept questions

Date: 2026-09-30.

## Context

The Route Lab workbench already carries a bounded editing session between
requests. That snapshot includes undo and a search cursor. A creation has a
different lifetime: a player chooses a network and delivery order worth keeping,
then reopens or remixes it in another visit. The existing project chain already
owns the chosen question, evidence links, revisions, and defensive local writes.

Studio expression capsules do not represent street networks. Reusing their
format would change their meaning and historical identities. A second route
project store would duplicate the existing question and persistence owners.

## Decision

Core owns a separate versioned `NUMINOUS_ROUTE 1` creation, bounded to 8 KiB.
It preserves the authored junctions, explicit two-way road costs and closures,
required stops with the depot first, and selected delivery order. A remix carries
its parent's content identity. Identity describes canonical content and lineage;
it does not attest authorship or independent custody.

Opening validates the network and begins a fresh workbench. Undo, local revision,
and search playback belong to the editing session and are not imported as a
creation. Structurally valid disconnected networks remain keepable. Comparisons
and recorded solver decisions are calculated from the reopened roads, rather
than accepted as saved proof. Caller-carried workbench snapshots retain their
existing full-session continuation contract.

The existing project chain accepts a closed choice of Studio or route creation.
It keeps the question and evidence beside that creation. Route-containing
portable documents use `NUMINOUS_PROJECT 2`; route-containing chains use
`numinous-project-v2`. Existing Studio-only documents and chains retain their
version 1 bytes and identities. The readers admit both supported versions and
refuse unsupported versions. Old readers refuse the new version.

Route project next calls remain closed, followable `route_lab` calls with a
capsule string and an explicit open or remix action. Preview does not execute
those calls or write a workspace. Keeping and importing use the existing locked,
atomic project persistence, with its existing capacity and file bounds. There
is no route-only database or implicit journal write.

Native authoring owns its core workbench directly. Focus, draft controls, and
schematic node positions are presentation state. Screen distance never supplies
a road cost. An atomic network edit uses the same validation, checked revision,
bounded undo, and trace invalidation as smaller workbench edits. Failed edits
retain the current state. Leaving the native editor preserves its in-process
session; keeping is a separate deliberate act.

## Consequences

A kept route can reopen in every face without substituting the opening four-stop
example. A project question and a route creation travel together, while a remix
can leave its parent unchanged. An unfinished disconnected network can be saved
and repaired later. Session history and authored content have explicit limits
and lifetimes rather than an implied promise that every transient state persists.

Compatibility, resource bounds, canonical identity, refusal without mutation,
and followable resume calls need regression evidence. Native controls and custom
network rendering need composed default and compact checks. Those tests establish
functional behavior, not participant comprehension or enjoyment.
