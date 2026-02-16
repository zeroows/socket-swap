# Quick Start: TCP/HTTP Access to SocketSwap

This guide shows how to use SocketSwap from **any Pod in your Kubernetes cluster** without the sidecar pattern.

## Prerequisites

1. Deploy SocketSwap with TCP support:
   ```bash
   kubectl apply -f k8s/rbac.yaml
   kubectl apply -f k8s/deployment.yaml
   kubectl apply -f k8s/service.yaml
   ```

2. Verify the Service is running:
   ```bash
   kubectl get svc socket-swap
   kubectl get pods -l app=socket-shim
   ```

## Usage Examples

### 1. From Python (Local or Cluster)

The example script `examples/tcp-client-example.py` supports both local testing (via port-forward) and cluster usage.

```bash
# Local testing (Terminal 1)
kubectl port-forward svc/socket-swap 2375:2375

# Local testing (Terminal 2)
export SOCKET_SWAP_URL=http://localhost:2375
uv run examples/tcp-client-example.py

# Inside K8s (default behavior)
uv run examples/tcp-client-example.py
```

### 2. From Python (Same Namespace)

```python
import docker

# Connect via Service DNS
client = docker.DockerClient(base_url='http://socket-swap.default.svc:2375')

# Use Docker API normally
container = client.containers.run('busybox', 'echo Hello!', detach=True)
print(container.logs().decode())
```

### 3. From Python (Different Namespace)

```python
import docker

# Use fully qualified service name
client = docker.DockerClient(
    base_url='http://socket-swap.default.svc.cluster.local:2375'
)

container = client.containers.run('alpine', 'ls -la', detach=True)
print(container.logs().decode())
```

### 4. From curl (Testing)

```bash
# Ping endpoint
kubectl run -it --rm test --image=curlimages/curl --restart=Never -- \
  curl http://socket-swap.default.svc:2375/_ping

# Get version
kubectl run -it --rm test --image=curlimages/curl --restart=Never -- \
  curl http://socket-swap.default.svc:2375/version

# Create container
kubectl run -it --rm test --image=curlimages/curl --restart=Never -- \
  curl -X POST http://socket-swap.default.svc:2375/containers/create \
  -H "Content-Type: application/json" \
  -d '{"Image":"busybox","Cmd":["echo","hello"]}'
```

### 5. From Go

```go
package main

import (
    "context"
    "fmt"
    "io"
    "os"

    "github.com/docker/docker/client"
    "github.com/docker/docker/api/types/container"
)

func main() {
    // Connect to SocketSwap
    cli, err := client.NewClientWithOpts(
        client.WithHost("tcp://socket-swap.default.svc:2375"),
        client.WithAPIVersionNegotiation(),
    )
    if err != nil {
        panic(err)
    }

    // Create container (becomes K8s Job)
    resp, err := cli.ContainerCreate(context.Background(),
        &container.Config{
            Image: "busybox",
            Cmd:   []string{"echo", "Hello from Go!"},
        },
        nil, nil, nil, "",
    )
    if err != nil {
        panic(err)
    }

    fmt.Printf("Container created: %s\n", resp.ID)

    // Get logs
    logs, err := cli.ContainerLogs(context.Background(), resp.ID,
        container.LogsOptions{ShowStdout: true})
    if err != nil {
        panic(err)
    }
    defer logs.Close()

    io.Copy(os.Stdout, logs)
}
```

### 6. From Node.js

```javascript
const Docker = require('dockerode');

// Connect via TCP
const docker = new Docker({
  host: 'socket-swap.default.svc',
  port: 2375
});

async function main() {
  // Create and run container
  const container = await docker.createContainer({
    Image: 'busybox',
    Cmd: ['echo', 'Hello from Node.js!']
  });

  await container.start();
  
  // Get logs
  const logs = await container.logs({
    stdout: true,
    stderr: true
  });
  
  console.log(logs.toString());
}

main();
```

## Environment Variables

Set `DOCKER_HOST` in your application:

```yaml
env:
  - name: DOCKER_HOST
    value: "tcp://socket-swap.default.svc:2375"
```

Or in your code:

```bash
export DOCKER_HOST=tcp://socket-swap.default.svc:2375
docker ps  # Now uses SocketSwap!
```

## Accessing from Different Namespaces

### Option 1: Full DNS Name

```python
client = docker.DockerClient(
    base_url='http://socket-swap.default.svc.cluster.local:2375'
)
```

### Option 2: ExternalName Service

Create a service alias in your namespace:

```yaml
apiVersion: v1
kind: Service
metadata:
  name: docker
  namespace: my-namespace
spec:
  type: ExternalName
  externalName: socket-swap.default.svc.cluster.local
  ports:
    - port: 2375
```

Then use:

```python
client = docker.DockerClient(base_url='http://docker.my-namespace.svc:2375')
```

### Option 3: NetworkPolicy (Recommended)

Allow your namespace to access SocketSwap:

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: allow-socket-swap
  namespace: my-namespace
spec:
  podSelector: {}
  policyTypes:
    - Egress
  egress:
    - to:
        - namespaceSelector:
            matchLabels:
              name: default
        - podSelector:
            matchLabels:
              app: socket-shim
      ports:
        - protocol: TCP
          port: 2375
```

## Security Considerations

⚠️ **Important**: The TCP endpoint has no authentication. Consider:

1. **Network Policies**: Restrict which Pods can connect
2. **Service Mesh**: Use Istio/Linkerd for mTLS
3. **RBAC**: The ServiceAccount still limits what Jobs can be created
4. **Namespace Isolation**: Keep SocketSwap and clients in trusted namespaces

## Troubleshooting

### Connection Refused

```bash
# Check service
kubectl get svc socket-swap -n default

# Check endpoints
kubectl get endpoints socket-swap -n default

# Test from debug pod
kubectl run -it --rm debug --image=busybox -- \
  telnet socket-swap.default.svc 2375
```

### DNS Not Resolving

```bash
# Test DNS
kubectl run -it --rm debug --image=busybox -- \
  nslookup socket-swap.default.svc
```

### Logs Not Working

Check SocketSwap pod logs:
```bash
kubectl logs -l app=socket-shim -f
```

## Example: Run Test Job

Deploy the test job to verify everything works:

```bash
kubectl apply -f examples/test-deployment.yaml
kubectl logs job/docker-api-test -f
```

Expected output:
```
🔌 Connected to SocketSwap!
API Version: 1.41
🚀 Creating container...
Container ID: abc123...
📋 Fetching logs...
Hello from nested Kubernetes Job!
✅ Test completed successfully!
```

## Next Steps

- Check out `examples/tcp-client-example.py` for more examples
- Read the full README for configuration options
- See the deployment manifests in `k8s/` for production setup

