# =========================================
# SocketSwap - Multi-Architecture Container Image
# =========================================
# 
# Base Image: Chainguard glibc-dynamic
# - Minimal attack surface with only glibc runtime
# - Non-root user by default for security
# - CVE-free and distroless design
# 
# Binary Targets: x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu
# =========================================

FROM cgr.dev/chainguard/glibc-dynamic:latest

# Accept build arguments from Docker build context
# BINARY_NAME: Name of the compiled Rust binary (e.g., socket-swap)
# TARGETARCH: Docker automatically sets to "amd64" or "arm64" for multi-platform builds
ARG BINARY_NAME=socket-swap
ARG TARGETARCH
ENV BINARY_NAME=${BINARY_NAME}

# Set the working directory for the application
WORKDIR /app

# =========================================
# Copy Application Assets
# =========================================

# Copy the compiled Rust binary for the target architecture
# Architecture-specific binaries are built in CI workflow and selected here
# Example: socket-swap-amd64 or socket-swap-arm64
# --chmod=755 makes it executable (required since distroless has no chmod command)
COPY --chmod=755 ./${BINARY_NAME}-${TARGETARCH} /app/service-app

# =========================================
# Security & Runtime Configuration
# =========================================

# Switch to non-root user (Chainguard images include 'nonroot' user by default)
# This follows security best practices and reduces attack surface
USER nonroot

# Set the container entry point to run the socket-swap server
ENTRYPOINT ["/app/service-app"]
