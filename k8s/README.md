# SocketSwap Kubernetes Manifests

This directory contains Kubernetes manifests for deploying SocketSwap using Kustomize.

## Directory Structure

```
k8s/
├── base/                    # Base manifests
│   ├── deployment.yaml
│   ├── service.yaml
│   ├── serviceaccount.yaml
│   ├── role.yaml
│   ├── rolebinding.yaml
│   └── kustomization.yaml
├── overlays/
│   ├── dev/                # Development environment
│   │   └── kustomization.yaml
│   └── production/         # Production environment
│       └── kustomization.yaml
└── README.md
```

## Prerequisites

- Kubernetes cluster (1.24+)
- kubectl installed
- kustomize (or use `kubectl apply -k`)

## Quick Start

### Deploy to Development Environment

```bash
# Apply all manifests for dev
kubectl apply -k k8s/overlays/dev

# Verify deployment
kubectl get all -n dev -l app=socket-swap

# Check the service
kubectl get svc -n dev socket-swap-svc
```

### Deploy to Production Environment

```bash
# Apply all manifests for production
kubectl apply -k k8s/overlays/production

# Verify deployment
kubectl get all -n production -l app=socket-swap

# Check the service
kubectl get svc -n production socket-swap-svc
```

## Usage from Applications

Once deployed, applications in the same namespace can connect to SocketSwap using the Docker API:

### Python Example

```python
import docker

# Connect to SocketSwap service
client = docker.DockerClient(base_url='http://socket-swap-svc:2375')

# Run a container
container = client.containers.run(
    'busybox',
    'echo Hello from Kubernetes!',
    detach=True
)

# Get logs
print(container.logs().decode())

# Clean up
container.remove()
```

### Environment Variables

```bash
export DOCKER_HOST=http://socket-swap-svc:2375
docker ps
docker run busybox echo "Hello!"
```

## Configuration

### Base Configuration

All base resources are in `k8s/base/`:
- **ServiceAccount**: `socket-swap-sa` with RBAC permissions
- **Role**: Permissions to create/manage Jobs and Pods
- **Service**: ClusterIP service exposing port 2375
- **Deployment**: Single replica running SocketSwap

### Environment Variables

The deployment supports the following environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `DOCKER_SOCKET_PATH` | `/var/run/docker.sock` | Path to Docker socket |
| `DOCKER_TCP_ADDR` | `0.0.0.0:2375` | TCP address to listen on |
| `KUBE_NAMESPACE` | (from fieldRef) | Kubernetes namespace |
| `JOB_TTL_SECONDS` | `0` | Kubernetes-side TTL for completed Jobs (socket-swap's active cleanup loop is the primary mechanism; set to a positive value to also use the cluster's TTL controller) |
| `JOB_ACTIVE_DEADLINE_SECONDS` | `3600` | Max seconds a Job can run |
| `DEFAULT_CPU_LIMIT` | `500m` | Default CPU limit for Jobs |
| `DEFAULT_MEMORY_LIMIT` | `512Mi` | Default memory limit for Jobs |
| `DEFAULT_CPU_REQUEST` | `100m` | Default CPU request for Jobs |
| `DEFAULT_MEMORY_REQUEST` | `128Mi` | Default memory request for Jobs |

### Resource Limits

**SocketSwap Pod:**
- Requests: 100m CPU, 128Mi RAM, 512Mi ephemeral storage
- Limits: 500m CPU, 256Mi RAM, 1Gi ephemeral storage

**Production** (with overlay):
- Limits increased to: 1000m CPU, 512Mi RAM
- Replicas: 2

## Customization with Kustomize

### Override Image Tag

```yaml
# my-overlay/kustomization.yaml
apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization
resources:
  - ../../base

images:
  - name: socket-swap-image
    newName: ghcr.io/your-org/socket-swap
    newTag: v1.2.3
```

### Add Image Pull Secrets

```yaml
# Add to overlay kustomization.yaml
patches:
  - target:
      kind: Deployment
      name: socket-swap-deployment
    patch: |-
      - op: add
        path: /spec/template/spec/imagePullSecrets
        value:
          - name: ghcr-secret
```

### Scale Replicas

```yaml
# Add to overlay kustomization.yaml
replicas:
  - name: socket-swap-deployment
    count: 3
```

## RBAC Permissions

SocketSwap requires the following permissions in its namespace:

- **Jobs/CronJobs**: create, get, list, watch, delete
- **Pods**: create, get, list, watch, delete
- **Pods/log**: get

These are defined in `base/role.yaml` and bound via `base/rolebinding.yaml`.

## Troubleshooting

### Check Pod Status

```bash
kubectl get pods -n dev -l app=socket-swap
kubectl describe pod -n dev -l app=socket-swap
kubectl logs -n dev -l app=socket-swap
```

### Test Connection

```bash
# Port forward to test locally
kubectl port-forward -n dev svc/socket-swap-svc 2375:2375

# In another terminal
docker -H tcp://localhost:2375 version
docker -H tcp://localhost:2375 run busybox echo "test"
```

### Check RBAC

```bash
# Verify service account exists
kubectl get sa -n dev socket-swap-sa

# Check role and rolebinding
kubectl get role,rolebinding -n dev | grep socket-swap
```

## Migration from Old Manifests

If you were using the old flat manifests (`deployment.yaml`, `service.yaml`, `rbac.yaml`), you can safely delete them:

```bash
# Old files (deprecated)
rm k8s/deployment.yaml
rm k8s/service.yaml
rm k8s/rbac.yaml
rm k8s/pod.yaml
```

The new kustomize-based structure provides:
- Better organization
- Environment-specific configurations
- Easier customization
- Version control for different deployments

## Links

- [SocketSwap Documentation](../README.md)
- [Quick Start Guide](../docs/QUICK_START_TCP.md)
- [TCP Support](../docs/TCP_SUPPORT.md)
