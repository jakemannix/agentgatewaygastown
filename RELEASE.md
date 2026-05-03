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

## Releases

### v0.0.0-test (2026-05-03)

Smoke test of the release workflow. Validates the multi-arch native build matrix
(`linux/amd64` + `linux/arm64`), the OCI label set, the manifest-combine job, and
the same-org GHCR publish path. Cut from `feature/tool-algebra-cleanup`. Not for
production consumption — purely an end-to-end pipeline check.
