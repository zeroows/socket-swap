# SocketSwap Examples

This directory contains examples and test cases for using SocketSwap to interact with the Docker API in a Kubernetes environment.

## Project Setup

This is a [uv](https://github.com/astral-sh/uv) managed project. To get started:

1.  **Install uv** (if you haven't already):
    ```bash
    curl -LsSf https://astral-sh/uv/install.sh | sh
    ```

2.  **Install dependencies**:
    ```bash
    uv sync
    ```

## Running SocketSwap Locally

When running the Rust server locally for development, you may encounter a `Permission denied` error if it tries to bind to `/var/run/docker.sock`. You can avoid this by using a local path or enabling the TCP listener:

```bash
# Option 1: Use a local Unix socket path
DOCKER_SOCKET_PATH=./docker.sock cargo run

# Option 2: Enable TCP listener for local testing
DOCKER_TCP_ADDR=127.0.0.1:2375 cargo run
```

## Examples

### 1. TCP Client Example (`tcp-client-example.py`)

This script demonstrates how to connect to SocketSwap via TCP/HTTP. It works both inside the Kubernetes cluster and locally via port-forwarding.

#### Running Locally (via Port-Forward)

1.  **Terminal 1**: Port-forward the SocketSwap service:
    ```bash
    kubectl port-forward svc/socket-swap 2375:2375
    ```

2.  **Terminal 2**: Run the example script:
    ```bash
    SOCKET_SWAP_URL=http://localhost:2375 uv run tcp-client-example.py
    ```

#### Running Inside Kubernetes

The script defaults to the internal Kubernetes DNS name (`http://socket-swap.default.svc:2375`). You can deploy it as a Job or Pod to test cluster-wide access.

### 2. Kubernetes Test Deployment (`test-deployment.yaml`)

A Kubernetes Job manifest that runs a Python client inside your cluster to verify that SocketSwap is correctly configured and accessible via its Service.

```bash
kubectl apply -f test-deployment.yaml
kubectl logs job/docker-api-test -f
```

## Key Features Demonstrated

- **Remote Access**: Connecting to Docker API via TCP without a local Unix socket.
- **Container Lifecycle**: Creating, starting, and inspecting containers (which become Kubernetes Jobs).
- **Log Streaming**: Fetching and streaming logs from the spawned containers.
- **Environment Variables**: Passing configuration to containers.

## Troubleshooting

If you encounter `NameResolutionError` or `ConnectionError`:
- Ensure the `socket-swap` service is running: `kubectl get svc socket-swap`
- Verify you are using the correct namespace in the URL (default is `default`).
- If running locally, check that your `kubectl port-forward` is active.
