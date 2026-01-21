#!/usr/bin/env python3
"""
Example: Using SocketSwap via TCP from anywhere in the Kubernetes cluster

This demonstrates remote Docker API access without needing the sidecar pattern.
Perfect for AI agents, cron jobs, or any workload that needs to spawn containers.
"""

import docker
import time

def main():
    # Connect to SocketSwap via Service DNS
    # Format: http://<service-name>.<namespace>.svc.cluster.local:<port>
    client = docker.DockerClient(base_url='http://socket-swap.default.svc:2375')
    
    print("Testing connection...")
    info = client.version()
    print(f"Connected to: {info['Version']}")
    print(f"API Version: {info['ApiVersion']}")
    
    print("\n--- Example 1: Run a simple command ---")
    container = client.containers.run(
        'busybox',
        'echo Hello from Kubernetes!',
        detach=True,
        remove=False  # Keep for logs
    )
    print(f"Created container: {container.id}")
    
    # Wait a bit for job to complete
    time.sleep(5)
    
    # Get logs
    logs = container.logs()
    print(f"Logs: {logs.decode()}")
    
    print("\n--- Example 2: Run with environment variables ---")
    container2 = client.containers.run(
        'busybox',
        'sh -c "echo Name: $NAME, Value: $VALUE"',
        environment={
            'NAME': 'MyApp',
            'VALUE': '12345'
        },
        detach=True
    )
    print(f"Created container: {container2.id}")
    
    time.sleep(5)
    logs2 = container2.logs()
    print(f"Logs: {logs2.decode()}")
    
    print("\n--- Example 3: Inspect container (Job) status ---")
    container_info = client.api.inspect_container(container.id)
    print(f"Container ID: {container_info['Id']}")
    print(f"State: {container_info['State']['Status']}")
    print(f"Exit Code: {container_info['State']['ExitCode']}")
    
    print("\n--- Example 4: Stream logs in real-time ---")
    container3 = client.containers.run(
        'busybox',
        'sh -c "for i in 1 2 3 4 5; do echo Line $i; sleep 1; done"',
        detach=True
    )
    
    print(f"Streaming logs for {container3.id}...")
    for line in container3.logs(stream=True, follow=True):
        print(f"  {line.decode().strip()}")
    
    print("\nAll tests completed!")

if __name__ == '__main__':
    main()

