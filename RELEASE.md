# Releases

Release artifacts for the AnyAlignment fork of agentgateway are published to
[`ghcr.io/yetanotheruseless/agentgateway`](https://github.com/yetanotheruseless/agentgateway/pkgs/container/agentgateway).

See [`infra/oss-forks.md`](https://github.com/yetanotheruseless/anyalignment-deploy/blob/main/infra/oss-forks.md)
in `anyalignment-deploy` for the policy and consumption pattern.

## Tag classes

| Tag                              | Trigger                                | Mutability                  |
|----------------------------------|----------------------------------------|------------------------------|
| `vX.Y.Z` (and `vX.Y.Z-<suffix>`) | git tag matching `v*.*.*`              | Immutable                    |
| `main-<short-sha>`               | push to `main`                          | Immutable in practice; 30d retention |
| `branch-<sanitized>-<short-sha>` | push to any non-`main` branch           | Immutable in practice; 30d retention |

## Versioning convention

Our fork has substantial divergence from upstream `agentgateway/agentgateway`.
The fork's git history inherits all of upstream's release tags (`v0.0.1`
through `v0.11.x`), so we cannot reuse upstream's version namespace without
collisions.

Our releases use a SemVer pre-release suffix anchored to the upstream version
we last synced/forked from:

```
v<upstream-base>-anyalignment.<n>
```

For example, `v0.11.1-anyalignment.1` reads as "based on upstream `v0.11.1`,
our first release on top." `<n>` increments per release on the same upstream
base. When we next sync from upstream (say to `v0.12.0`), the suffix counter
resets: `v0.12.0-anyalignment.1`.

The suffix is a valid SemVer pre-release identifier, and matches the
workflow's `v*.*.*` tag glob (since `-anyalignment.N` adds no extra dots
that would break the segment count).

## Releases

### v0.11.1-anyalignment.1 (2026-05-03)

First proper release of the AnyAlignment fork. Cut from
`feature/tool-algebra-cleanup`, which carries our virtual-tools execution
work (scatter-gather, pipelines, registry types, runtime hooks) — 192
commits past upstream `v0.11.1`.

This is what consuming services in `yetanotheruseless/anyalignment-deploy`
should pin to, per [`infra/oss-forks.md`](https://github.com/yetanotheruseless/anyalignment-deploy/blob/main/infra/oss-forks.md)
and [ADR 0006](https://github.com/yetanotheruseless/anyalignment-deploy/blob/main/docs/adr/0006-codebase-structure.md).

Multi-arch (`linux/amd64`, `linux/arm64`), public, `debian:bookworm-slim`
runtime base, OCI source labels populated.

```sh
docker pull ghcr.io/yetanotheruseless/agentgateway:v0.11.1-anyalignment.1
```

Note: upstream's `main` branch (and consequently our fork's `main`, which
mirrors it) is not currently buildable — see the prior failed CI run from
PR #9's merge for the `unresolved import 'super::types::DependencyType'`
compile error. That divergence is out of scope for this release;
`v0.11.1-anyalignment.1` builds cleanly because it comes from the working
branch where the type lives in the right place.
