# TCP Support

SocketSwap supports TCP/HTTP access in addition to the Unix socket, allowing remote access from any Pod in the Kubernetes cluster without the sidecar pattern.

## How It Works

When `DOCKER_TCP_ADDR` is set, SocketSwap binds a TCP listener alongside the Unix socket. Both listeners share the same request router and handler logic.

```
┌─────────────────────────────┐
│         SocketSwap Pod      │
│  ┌────────┐   ┌──────────┐ │
│  │ Agent  │──▶│  Shim    │ │
│  └────────┘   │          │ │
│       unix     │  :2375   │ │
│                └─────┬────┘ │
└──────────────────────┼──────┘
                       │
          ┌────────────▼────────────┐
          │  Service (ClusterIP)    │
          │  socket-swap-svc:2375   │
          └────────────┬────────────┘
                       │
         ┌─────────────┴─────────────┐
         │                           │
    ┌────▼────┐                 ┌────▼────┐
    │ Pod A   │                 │ Pod B   │
    │ (ns-1)  │                 │ (ns-2)  │
    └─────────┘                 └─────────┘
```

## Configuration

```bash
# Enable TCP alongside Unix socket
export DOCKER_TCP_ADDR=0.0.0.0:2375

# TCP only (Unix socket still created but optional)
export DOCKER_TCP_ADDR=0.0.0.0:2375
```

The Kustomize base deployment enables TCP by default with `DOCKER_TCP_ADDR=0.0.0.0:2375`.

## Accessing from Different Namespaces

**Same namespace:**
```
http://socket-swap-svc:2375
```

**Different namespace (short form):**
```
http://socket-swap-svc.<namespace>:2375
```

**Different namespace (FQDN):**
```
http://socket-swap-svc.<namespace>.svc.cluster.local:2375
```

**ExternalName alias** (create in your namespace):
```yaml
apiVersion: v1
kind: Service
metadata:
  name: docker
  namespace: my-namespace
spec:
  type: ExternalName
  externalName: socket-swap-svc.default.svc.cluster.local
  ports:
    - port: 2375
```

## Security Considerations

The TCP endpoint has **no authentication**. Recommended measures:

1. **NetworkPolicy** - Restrict which Pods can connect:
   ```yaml
   apiVersion: networking.k8s.io/v1
   kind: NetworkPolicy
   metadata:
     name: socket-swap-access
   spec:
     podSelector:
       matchLabels:
         app: socket-swap
     policyTypes:
       - Ingress
     ingress:
       - from:
           - podSelector:
               matchLabels:
                 allow-docker-api: "true"
         ports:
           - protocol: TCP
             port: 2375
   ```

2. **Service Mesh** - Use Istio/Linkerd for mTLS between services.

3. **Namespace Isolation** - Keep SocketSwap in a trusted namespace.

4. **RBAC** - The underlying ServiceAccount still restricts what Jobs can be created.
