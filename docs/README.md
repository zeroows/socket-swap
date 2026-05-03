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
| `JOB_TTL_SECONDS` | Kubernetes-side TTL for completed Jobs (`0` = rely on socket-swap's active cleanup loop) | `0` |
| `JOB_ACTIVE_DEADLINE_SECONDS` | Max seconds a Job can run before termination | `3600` |
| `DEFAULT_CPU_LIMIT` | CPU limit for spawned Jobs | `500m` |
| `DEFAULT_MEMORY_LIMIT` | Memory limit for spawned Jobs | `512Mi` |
| `DEFAULT_CPU_REQUEST` | CPU request for spawned Jobs | `100m` |
| `DEFAULT_MEMORY_REQUEST` | Memory request for spawned Jobs | `128Mi` |

## Job Cleanup Behavior

SocketSwap configures the following mechanisms on every Job it creates:

| Mechanism | Field | Default | Purpose |
|-----------|-------|---------|---------|
| TTL cleanup | `ttlSecondsAfterFinished` | 0s | Belt-and-suspenders auto-delete (only effective if the cluster's TTL controller is enabled) |
| No retries | `backoffLimit` | 0 | Prevents retry Pods on failure |
| Active deadline | `activeDeadlineSeconds` | 3600s (1 hour) | Terminates long-running Jobs |
| Termination message | `terminationMessagePolicy` | `FallbackToLogsOnError` | Captures the last lines of stdout/stderr into the Pod's `terminated.message` field on failure, so failures are debuggable even after the runtime garbage-collects log files |

### Active cleanup loop

In addition to `ttlSecondsAfterFinished`, socket-swap runs an **active background cleanup task** every 60 seconds (the first sweep fires immediately on startup). This is the primary cleanup mechanism — it works regardless of whether your cluster's TTL controller is enabled.

| Pod state | First sweep | Next sweep |
|-----------|-------------|------------|
| Succeeded | Job deleted (cascades to pod) | — |
| Failed    | Marked with `socket-swap/pending-cleanup=true` | Job deleted |
| Running   | Skipped | Skipped |

The two-phase deletion of failed jobs gives you a ~60s inspection window before the failure is removed:

```bash
# Inspect failed jobs that are pending cleanup
kubectl get jobs -n <ns> -l socket-swap/pending-cleanup=true

# Read the captured failure message (works even after log files are GC'd)
kubectl get pod <name> -n <ns> -o jsonpath='{.status.containerStatuses[0].state.terminated.message}'
```

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
