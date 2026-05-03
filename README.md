# SocketSwap: Docker-to-Kubernetes Socket Shim

A Rust-based proxy that translates Docker API calls into Kubernetes Job operations, enabling legacy applications and AI agents to use familiar Docker commands while running on Kubernetes.

## Architecture

SocketSwap listens on both a Unix socket (`/var/run/docker.sock`) and TCP port (`:2375`), accepts Docker API requests, and translates them into native Kubernetes Job resources.

**Deployment Patterns:**
- **Sidecar Pattern**: Share Unix socket via `emptyDir` volume (same Pod)
- **Service Pattern**: Access via TCP from anywhere in the cluster (remote Pods)

```
                                 ┌─────────────┐
                                 │  Remote Pod  │
                                 │  (any ns)    │
                                 └──────┬───────┘
                                        │ TCP :2375
┌───────────────────────────────────────┼──────────┐
│          SocketSwap Pod               │          │
│  ┌──────────────┐   ┌────────────────┐│          │
│  │  AI Agent /  │   │  SocketSwap   ◀┘          │
│  │  Legacy App  │──▶│               │            │
│  └──────────────┘   └───────┬───────┘            │
│   DOCKER_HOST=              │                    │
│   unix:///var/run/          │                    │
│        docker.sock          │                    │
└─────────────────────────────┼────────────────────┘
                              │
                              ▼
                    ┌─────────────────┐
                    │ Kubernetes API  │
                    └────────┬────────┘
                             │
                             ▼
                    ┌─────────────────┐
                    │   Jobs / Pods   │
                    └─────────────────┘
```

## Features

- **Zero Migration**: Existing Docker-based code works without modifications
- **Secure**: No privileged containers or host Docker socket access required
- **Namespace-scoped RBAC**: Limited permissions for Job management
- **Automatic Cleanup**: Jobs cleaned up via TTL, backoff limit, and active deadline
- **Docker API Compatible**: Supports common Docker CLI operations
- **Log Streaming**: Full support for Docker's multiplexed log format with follow and tail
- **Multi-arch**: Pre-built binaries and images for amd64 and arm64

## Supported Docker API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/_ping` | GET | Health check |
| `/version` | GET | API version info (reports 1.41) |
| `/containers/create` | POST | Creates a Kubernetes Job |
| `/containers/{id}/start` | POST | No-op (Jobs auto-start) |
| `/containers/{id}/stop` | POST | Deletes the Job |
| `/containers/{id}/json` | GET | Inspects Job/Pod state |
| `/containers/{id}/logs` | GET | Streams Pod logs (`?follow=true&tail=N`) |
| `/containers/{id}/wait` | POST | Waits for Job completion |
| `/containers/{id}` | DELETE | Removes the Job |
| `/images/create` | POST | Returns 200 (images pulled at Job creation) |
| `/images/{name}/json` | GET | Returns minimal image metadata |
| `/volumes/{name}` | GET | Returns minimal volume metadata |

API version prefixes (`/v1.41`, `/v1.40`, `/v1.39`) are supported and stripped before routing.

## Building

### Prerequisites

- Rust 1.75 or later
- Kubernetes cluster (for deployment)

### Build Binary

```bash
cargo build --release
```

### Build Docker Image

The project uses a multi-arch Dockerfile at `devops/Dockerfile` with a Chainguard distroless base image:

```bash
# Build binaries first (or let CI handle it)
cargo build --release --target x86_64-unknown-linux-musl
cp target/x86_64-unknown-linux-musl/release/socket-swap ./socket-swap-amd64

docker build -f devops/Dockerfile -t socket-swap:latest --build-arg BINARY_NAME=socket-swap .
```

## Deployment

SocketSwap uses [Kustomize](https://kustomize.io/) for Kubernetes manifests. See [k8s/README.md](k8s/README.md) for full details.

### Quick Start

```bash
# Development
kubectl apply -k k8s/overlays/dev

# Production
kubectl apply -k k8s/overlays/production
```

### Verify

```bash
kubectl get all -l app=socket-swap
kubectl run -it --rm test --image=curlimages/curl --restart=Never -- \
  curl http://socket-swap-svc:2375/_ping
```

### Configure Your Application

**Unix Socket (Sidecar pattern):**

```yaml
env:
  - name: DOCKER_HOST
    value: "unix:///var/run/docker.sock"
```

**TCP/HTTP (Remote access):**

```yaml
env:
  - name: DOCKER_HOST
    value: "tcp://socket-swap-svc:2375"
```

## Configuration

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

### Listener Modes

```bash
# Unix Socket only (default)
export DOCKER_SOCKET_PATH=/var/run/docker.sock

# Unix Socket + TCP
export DOCKER_SOCKET_PATH=/var/run/docker.sock
export DOCKER_TCP_ADDR=0.0.0.0:2375

# TCP only
export DOCKER_TCP_ADDR=0.0.0.0:2375
```

## Usage Examples

### From Python (docker-py)

```python
import docker

# Sidecar (Unix socket)
client = docker.DockerClient(base_url='unix:///var/run/docker.sock')

# Remote (TCP) - same namespace
client = docker.DockerClient(base_url='http://socket-swap-svc:2375')

# Remote (TCP) - different namespace
client = docker.DockerClient(base_url='http://socket-swap-svc.production.svc.cluster.local:2375')

# Create and run a container (creates a K8s Job)
container = client.containers.run(
    'busybox',
    'echo Hello from Kubernetes!',
    detach=True
)

# Get logs
logs = container.logs()
print(logs.decode())
```

### From Docker CLI

```bash
# Via TCP
export DOCKER_HOST=tcp://socket-swap-svc:2375
docker run busybox echo "Hello"
docker logs <container-id>

# Via Unix socket (sidecar)
docker -H unix:///var/run/docker.sock run busybox echo "Hello"
```

### From curl

```bash
# Ping
curl http://socket-swap-svc:2375/_ping

# Create a container
curl -X POST http://socket-swap-svc:2375/v1.41/containers/create \
  -H "Content-Type: application/json" \
  -d '{"Image": "busybox", "Cmd": ["echo", "Hello"]}'
```

## How It Works

### Docker to Kubernetes Translation

| Docker Concept | Kubernetes Resource |
|----------------|---------------------|
| Container | Job with single Pod |
| Container ID | Job label: `container-id` |
| Image | Pod container image |
| Env vars | Pod container env |
| Command / Entrypoint | Pod container args / command |
| Logs | Pod logs (via K8s API) |

### Job Lifecycle

1. **Create** (`POST /containers/create`): Generates a unique container ID, creates a Kubernetes Job with labels, returns the ID.

2. **Start** (`POST /containers/{id}/start`): No-op since Kubernetes Jobs start automatically.

3. **Logs** (`GET /containers/{id}/logs`): Finds the Pod for the Job, streams logs using Docker's multiplex protocol. Supports `?follow=true` and `?tail=N`.

4. **Wait** (`POST /containers/{id}/wait`): Polls the Job until completion and returns the exit code.

5. **Cleanup**: Automatic via three mechanisms:
   - `ttlSecondsAfterFinished` deletes completed Jobs (default: 300s)
   - `backoffLimit: 0` prevents retry Pods on failure
   - `activeDeadlineSeconds` terminates long-running Jobs (default: 3600s)

### Job Name Sanitization

Kubernetes resource names are limited to 63 characters. SocketSwap prefixes names with `ss-` and truncates long names using a CRC32 hash suffix to ensure uniqueness.

## Security

- No privileged containers required
- No host Docker socket access
- Namespace-scoped RBAC (Jobs, Pods, Pods/log only)
- Resource limits enforced on all spawned Jobs
- TCP endpoint has **no authentication** - use NetworkPolicies to restrict access

## Limitations

- **Volumes**: Host volume mounts not supported
- **Networking**: Port publishing not implemented
- **Interactive**: No TTY or stdin support
- **Images**: Must be accessible from the cluster's container runtime

## Troubleshooting

### Socket not accessible

```bash
ls -l /var/run/docker.sock
# Should show: srw-rw-rw-
```

### Jobs not appearing

```bash
kubectl auth can-i create jobs --as=system:serviceaccount:<ns>:socket-swap-sa
```

### Logs not streaming

```bash
kubectl logs -l app=socket-swap -f
```

### TCP connection refused

```bash
kubectl get svc socket-swap-svc
kubectl get endpoints socket-swap-svc
kubectl get pods -l app=socket-swap
```

## Documentation

- **[Kubernetes Manifests](k8s/README.md)** - Kustomize-based deployment guide
- **[TCP Quick Start](docs/QUICK_START_TCP.md)** - Using SocketSwap via TCP from anywhere in the cluster
- **[TCP Architecture](docs/TCP_SUPPORT.md)** - TCP implementation details and security considerations
- **[Examples](examples/README.md)** - Python client examples and test deployments
- **[Documentation Index](docs/README.md)** - Full documentation overview

## Development

### Run locally (requires kubeconfig)

```bash
export DOCKER_SOCKET_PATH=/tmp/test-docker.sock
export KUBE_NAMESPACE=default
cargo run
```

### Run tests

```bash
cargo test
cargo clippy -- -D warnings
cargo fmt -- --check
```

## License

MIT

## Contributing

Contributions welcome! Please open an issue or PR.
