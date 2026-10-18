# Package platforms

Date: 2026-10-17. Status: accepted.

## Context

Published packages are built on GitHub-hosted runners. `ubuntu-22.04` is
deprecated and reaches full removal on 2027-04-17, with brownouts before
that. `ubuntu-latest` starts moving from 24.04 to 26.04 on 2026-10-19.
`macos-15-intel` is the last Intel macOS image and remains available through
August 2027. macOS 13 is outside Apple's security updates. The runners that
build and smoke the macOS packages are `macos-15` and `macos-15-intel`.

## Decision

Linux jobs name `ubuntu-24.04` instead of `ubuntu-latest`. Published Linux
packages therefore link glibc 2.39. CI also runs a locked workspace check on
`ubuntu-26.04`, so the next runner image is exercised before any later move.

The Intel macOS package stays. Apple silicon and Intel packages are both
built and smoked on macOS 15. `MACOSX_DEPLOYMENT_TARGET` is 15.0, which is
macOS 15 Sequoia: the oldest release Apple still updates that those runners
actually build. The Intel runner ends in August 2027. PLAY.md and VERIFY.md
state these floors.

## Consequences

A tag during the `ubuntu-latest` migration does not change the Linux package
floor by surprise. Players on macOS 14 and earlier are outside the published
packages. Dropping the Intel package is a later decision, dated by that
runner's retirement.
