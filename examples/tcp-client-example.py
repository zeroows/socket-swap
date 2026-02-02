#!/usr/bin/env python3
# /// script
# requires-python = ">=3.14"
# dependencies = [
#     "docker>=7.1.0",
# ]
# ///
"""
Example: Using SocketSwap via TCP from anywhere in the Kubernetes cluster

This demonstrates remote Docker API access without needing the sidecar pattern.
Perfect for AI agents, cron jobs, or any workload that needs to spawn containers.
"""

import docker
import time
import os
import sys

def main():
    # Connect to SocketSwap
    # Default: Service DNS (works inside K8s)
    # For local testing, use environment variable: SOCKET_SWAP_URL=http://localhost:2375
    base_url = os.environ.get('SOCKET_SWAP_URL', 'http://socket-swap.default.svc:2375')
    
    print(f"Connecting to SocketSwap at: {base_url}")
    try:
        client = docker.DockerClient(base_url=base_url)
        info = client.version()
        print(f"Connected! Version: {info['Version']}, API: {info['ApiVersion']}")
    except Exception as e:
        print(f"Error: Could not connect to SocketSwap at {base_url}")
        print(f"Details: {e}")
        if 'socket-swap.default.svc' in base_url:
            print("\nTIP: If running locally, port-forward the service first:")
            print("  kubectl port-forward svc/socket-swap 2375:2375")
            print("Then run with:")
            print("  SOCKET_SWAP_URL=http://localhost:2375 uv run tcp-client-example.py")
        sys.exit(1)
    
    # print("\n--- Example 1: Run a simple command ---")
    # container = client.containers.run(
    #     'busybox',
    #     'echo Hello from Kubernetes!',
    #     detach=True,
    #     remove=False  # Keep for logs
    # )
    # print(f"Created container: {container.id}")
    
    # # Wait a bit for job to complete
    # time.sleep(5)
    
    # # Get logs
    # logs = container.logs()
    # print(f"Logs: {logs.decode()}")
    
    # print("\n--- Example 2: Run with environment variables ---")
    # container2 = client.containers.run(
    #     'busybox',
    #     'sh -c "echo Name: $NAME, Value: $VALUE"',
    #     environment={
    #         'NAME': 'MyApp',
    #         'VALUE': '12345'
    #     },
    #     detach=True
    # )
    # print(f"Created container: {container2.id}")
    
    # time.sleep(5)
    # logs2 = container2.logs()
    # print(f"Logs: {logs2.decode()}")
    
    # print("\n--- Example 3: Inspect container (Job) status ---")
    # container_info = client.api.inspect_container(container.id)
    # print(f"Container ID: {container_info['Id']}")
    # print(f"State: {container_info['State']['Status']}")
    # print(f"Exit Code: {container_info['State']['ExitCode']}")
    
    # print("\n--- Example 4: Stream logs in real-time ---")
    # container3 = client.containers.run(
    #     'busybox',
    #     'sh -c "for i in 8 9 10 11 12 13 14 15 16 17 18 19 20; do echo Line $i; sleep 1; done"',
    #     detach=True,
    #     remove=True
    # )
    
    # print(f"Streaming logs for {container3.id}...")
    # for line in container3.logs(stream=True, follow=True):
    #     print(f"  {line.decode().strip()}")
    
    print("\n--- Example 5: Image Management (Mocked) ---")
    image_name = "alpine:latest"
    print(f"Pulling image: {image_name}...")
    # This calls POST /images/create
    pull_log = client.images.pull(image_name)
    print(f"Pull response: {pull_log}")
    
    print(f"Inspecting image: {image_name}...")
    # This calls GET /images/{name}/json
    image_info = client.images.get(image_name)
    print(f"Image ID: {image_info.id}")
    print(f"Tags: {image_info.tags}")
    
    print("\nAll tests completed!")

if __name__ == '__main__':
    main()

