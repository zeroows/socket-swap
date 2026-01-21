# SocketSwap: Docker-to-Kubernetes Socket Shim

A Rust-based proxy that translates Docker API calls into Kubernetes Job operations, enabling legacy applications and AI agents to use familiar Docker commands while running on Kubernetes.

## Architecture

SocketSwap acts as a proxy that listens on both a Unix socket (`/var/run/docker.sock`) and TCP port (`:2375`), accepts Docker CLI commands, and translates them into native Kubernetes Job resources.

**Deployment Patterns:**
- **Sidecar Pattern**: Share Unix socket via `emptyDir` volume (same Pod)
- **Service Pattern**: Access via TCP from anywhere in the cluster (remote Pods)

```
┌─────────────────────────────────────────┐
│          Agent Pod                      │
│  ┌──────────────┐   ┌───────────────┐  │
│  │  AI Agent /  │   │ Rust Socket   │  │
│  │ Legacy App   │──▶│    Shim       │  │
│  │              │   │               │  │
│  └──────────────┘   └───────┬───────┘  │
│   DOCKER_HOST=              │          │
│   unix:///var/run/          │          │
│        docker.sock          │          │
└─────────────────────────────┼──────────┘
                              │
                              ▼
                    ┌─────────────────┐
                    │ Kubernetes API  │
                    └────────┬────────┘
                             │
                             ▼
                    ┌─────────────────┐
                    │   Jobs/Pods     │
                    └─────────────────┘
```

## Features

- **Zero Migration**: Existing Docker-based code works without modifications
- **Secure**: No privileged containers or host Docker socket access required
- **Namespace-scoped RBAC**: Limited permissions for Job management
- **Automatic Cleanup**: Jobs are deleted after completion (configurable TTL)
- **Docker API Compatible**: Supports common Docker CLI operations
- **Log Streaming**: Full support for Docker's multiplexed log format

## Supported Docker API Endpoints

| Endpoint | Method | Status |
|----------|--------|--------|
| `/_ping` | GET | ✅ Implemented |
| `/version` | GET | ✅ Implemented |
| `/containers/create` | POST | ✅ Implemented |
| `/containers/{id}/start` | POST | ✅ Implemented (no-op) |
| `/containers/{id}/stop` | POST | ✅ Implemented |
| `/containers/{id}/json` | GET | ✅ Implemented |
| `/containers/{id}/logs` | GET | ✅ Implemented |
| `/containers/{id}/wait` | POST | ✅ Implemented |
| `/containers/{id}` | DELETE | ✅ Implemented |

## Building

### Prerequisites

- Rust 1.75 or later
- Docker (for building container image)
- Kubernetes cluster (for deployment)

### Build Binary

```bash
cargo build --release
```

### Build Docker Image

```bash
docker build -t socket-shim:latest .
```

## Deployment

### 1. Apply RBAC Configuration

```bash
kubectl apply -f k8s/rbac.yaml
```

This creates:
- ServiceAccount: `job-manager-sa`
- Role: `job-manager-role` (with permissions for jobs, pods, logs)
- RoleBinding: `job-manager-binding`

### 2. Deploy as Sidecar

Choose one of the deployment options:

**Option A: Single Pod (for testing)**
```bash
kubectl apply -f k8s/pod.yaml
```

**Option B: Deployment + Service (for production)**
```bash
kubectl apply -f k8s/deployment.yaml
kubectl apply -f k8s/service.yaml
```

Edit the deployment to replace `your-agent:latest` with your actual AI agent or legacy application image.

### 3. Configure Your Application

**Option A: Unix Socket (Sidecar pattern)**

Set the `DOCKER_HOST` environment variable in your application container:

```yaml
env:
  - name: DOCKER_HOST
    value: "unix:///var/run/docker.sock"
```

**Option B: TCP/HTTP (Remote access)**

Access from any Pod in the cluster:

```yaml
env:
  - name: DOCKER_HOST
    value: "tcp://socket-swap.default.svc:2375"
```

Or from Python:

```python
import docker
# From within the same namespace
client = docker.DockerClient(base_url='http://socket-swap.default.svc:2375')

# From a different namespace
client = docker.DockerClient(base_url='http://socket-swap.mynamespace.svc.cluster.local:2375')
```

## Configuration

The shim is configured via environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `DOCKER_SOCKET_PATH` | Path to Unix socket | `/var/run/docker.sock` |
| `DOCKER_TCP_ADDR` | TCP address to listen on (optional) | None (disabled) |
| `KUBE_NAMESPACE` | Kubernetes namespace for Jobs | `default` |
| `JOB_TTL_SECONDS` | Seconds before Job cleanup | `300` |
| `DEFAULT_CPU_LIMIT` | Default CPU limit for spawned Jobs | `500m` |
| `DEFAULT_MEMORY_LIMIT` | Default memory limit for spawned Jobs | `512Mi` |
| `DEFAULT_CPU_REQUEST` | Default CPU request for spawned Jobs | `100m` |
| `DEFAULT_MEMORY_REQUEST` | Default memory request for spawned Jobs | `128Mi` |

### Listener Modes

**Unix Socket Only (Default):**
```bash
export DOCKER_SOCKET_PATH=/var/run/docker.sock
# DOCKER_TCP_ADDR not set
```

**Unix Socket + TCP:**
```bash
export DOCKER_SOCKET_PATH=/var/run/docker.sock
export DOCKER_TCP_ADDR=0.0.0.0:2375
```

**TCP Only:**
```bash
export DOCKER_TCP_ADDR=0.0.0.0:2375
# Unix socket will still be created but not required
```

## Usage Examples

> 📖 **For detailed TCP/HTTP usage, see the [TCP Quick Start Guide](docs/QUICK_START_TCP.md)**

### From Docker CLI

```bash
# Ping the daemon
docker -H unix:///var/run/docker.sock ps

# Run a container (creates a Kubernetes Job)
docker -H unix:///var/run/docker.sock run busybox echo "Hello"

# View logs
docker -H unix:///var/run/docker.sock logs <container-id>
```

### From Python (docker-py)

**Using Unix Socket (Sidecar):**
```python
import docker

client = docker.DockerClient(base_url='unix:///var/run/docker.sock')

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

**Using TCP/HTTP (Remote):**
```python
import docker

# From anywhere in the cluster
client = docker.DockerClient(base_url='http://socket-swap.default.svc:2375')

# Same API works!
container = client.containers.run(
    'busybox',
    'echo Hello from Kubernetes!',
    detach=True
)

logs = container.logs()
print(logs.decode())
```

### From curl

```bash
# Create a container
curl --unix-socket /var/run/docker.sock \
  -X POST \
  http://localhost/v1.41/containers/create \
  -H "Content-Type: application/json" \
  -d '{"Image": "busybox", "Cmd": ["echo", "Hello"]}'
```

## How It Works

### Docker → Kubernetes Translation

| Docker Concept | Kubernetes Resource |
|----------------|---------------------|
| Container | Job with single Pod |
| Container ID | Job label: `container-id` |
| Image | Pod container image |
| Env vars | Pod container env |
| Command | Pod container args |
| Logs | Pod logs (via K8s API) |

### Job Lifecycle

1. **Create**: Client sends `POST /containers/create`
   - Shim generates unique container ID
   - Creates Kubernetes Job with labels
   - Returns container ID to client

2. **Start**: Client sends `POST /containers/{id}/start`
   - No-op (Jobs start automatically)
   - Returns success

3. **Logs**: Client sends `GET /containers/{id}/logs`
   - Finds Pod for Job
   - Streams logs with Docker multiplex protocol

4. **Cleanup**: Automatic after completion
   - Job deleted after `ttlSecondsAfterFinished`
   - Or manually via DELETE endpoint

## Security Considerations

✅ **No privileged containers required**  
✅ **No host Docker socket access**  
✅ **Namespace-scoped RBAC**  
✅ **Limited to Job creation/deletion**  

Optional security enhancements:
- Image allowlist validation
- Resource limits on created Jobs
- Network policies for spawned Pods
- Pod Security Standards enforcement

## Limitations

- **Volumes**: Host volume mounts not supported (use PVCs in Job spec)
- **Networking**: Port publishing not implemented
- **Interactive**: No TTY or stdin support
- **Images**: Must be accessible from cluster

## Troubleshooting

### Socket not accessible

Check permissions on `/var/run/docker.sock`:
```bash
ls -l /var/run/docker.sock
# Should show: srw-rw-rw-
```

### Jobs not appearing

Check RBAC permissions:
```bash
kubectl auth can-i create jobs --as=system:serviceaccount:default:job-manager-sa
```

### Logs not streaming

Check pod logs for errors:
```bash
kubectl logs <pod-name> -c rust-shim
```

### TCP connection refused

Check if the Service exists and matches the selector:
```bash
kubectl get svc socket-swap
kubectl get pods -l app=socket-shim
```

Test connectivity from another Pod:
```bash
kubectl run -it --rm debug --image=busybox --restart=Never -- \
  wget -O- http://socket-swap.default.svc:2375/_ping
```

## Documentation

📚 **Guides and References:**

- **[TCP Support Guide](docs/TCP_SUPPORT.md)** - Complete TCP/HTTP implementation details and architecture
- **[TCP Quick Start](docs/QUICK_START_TCP.md)** - Step-by-step guide for using TCP from anywhere in the cluster
- **[Python TCP Example](examples/tcp-client-example.py)** - Complete working Python client
- **[Test Deployment](examples/test-deployment.yaml)** - Kubernetes Job for testing TCP connectivity

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
```

## License

MIT

## Contributing

Contributions welcome! Please open an issue or PR.

