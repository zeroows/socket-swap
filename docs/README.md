# SocketSwap Documentation

## Guides

| Document | Description |
|----------|-------------|
| [Main README](../README.md) | Project overview, features, configuration, and usage |
| [TCP Quick Start](QUICK_START_TCP.md) | Step-by-step guide for using SocketSwap via TCP from any Pod |
| [TCP Architecture](TCP_SUPPORT.md) | TCP implementation details, cross-namespace access, and security |

## Deployment

| Document | Description |
|----------|-------------|
| [Kubernetes Manifests](../k8s/README.md) | Kustomize-based deployment with dev/production overlays |

## Examples

| Document | Description |
|----------|-------------|
| [Examples README](../examples/README.md) | Python client examples and test deployments |
| [TCP Client Example](../examples/tcp-client-example.py) | Complete Python script for TCP access |
| [Test Deployment](../examples/test-deployment.yaml) | Kubernetes Job for verifying SocketSwap connectivity |

## Configuration Reference

All settings are configured via environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `DOCKER_SOCKET_PATH` | Path to Unix socket | `/var/run/docker.sock` |
| `DOCKER_TCP_ADDR` | TCP listen address (optional) | None (disabled) |
| `KUBE_NAMESPACE` | Kubernetes namespace for Jobs | `default` |
| `JOB_TTL_SECONDS` | Seconds before completed Job cleanup | `300` |
| `JOB_ACTIVE_DEADLINE_SECONDS` | Max seconds a Job can run before termination | `3600` |
| `DEFAULT_CPU_LIMIT` | CPU limit for spawned Jobs | `500m` |
| `DEFAULT_MEMORY_LIMIT` | Memory limit for spawned Jobs | `512Mi` |
| `DEFAULT_CPU_REQUEST` | CPU request for spawned Jobs | `100m` |
| `DEFAULT_MEMORY_REQUEST` | Memory request for spawned Jobs | `128Mi` |

## Job Cleanup Behavior

SocketSwap configures three mechanisms on every Job it creates:

| Mechanism | Field | Default | Purpose |
|-----------|-------|---------|---------|
| TTL cleanup | `ttlSecondsAfterFinished` | 300s (5 min) | Auto-deletes completed/failed Jobs |
| No retries | `backoffLimit` | 0 | Prevents retry Pods on failure |
| Active deadline | `activeDeadlineSeconds` | 3600s (1 hour) | Terminates long-running Jobs |

## Project Structure

```
SocketSwap/
├── src/
│   ├── main.rs              # Entry point, listener setup
│   ├── config.rs             # Environment variable configuration
│   ├── server.rs             # HTTP request router
│   ├── error.rs              # Error types
│   ├── docker/               # Docker API types and protocol
│   │   ├── types.rs          # Request/response structures
│   │   └── multiplex.rs      # Docker log multiplex encoder
│   ├── handlers/             # Request handlers
│   │   ├── containers.rs     # Container lifecycle (create, start, stop, etc.)
│   │   ├── images.rs         # Image operations
│   │   ├── info.rs           # Ping and version
│   │   ├── logs.rs           # Log streaming with follow/tail
│   │   └── volumes.rs        # Volume metadata
│   └── kubernetes/           # Kubernetes client layer
│       ├── jobs.rs           # JobManager (create, build, delete, list)
│       └── pods.rs           # PodManager (logs, status)
├── k8s/                      # Kustomize manifests
│   ├── base/                 # Base resources
│   └── overlays/             # Dev and production overrides
├── devops/
│   └── Dockerfile            # Multi-arch distroless container image
├── docs/                     # Documentation
├── examples/                 # Python examples and test manifests
└── .github/workflows/        # CI/CD pipelines
    ├── ci.yaml               # Tests, clippy, fmt on PR
    ├── dev.yaml              # Dev image build (manual trigger)
    └── release.yaml          # Production build + GitHub Release on tag
```
