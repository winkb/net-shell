# net-shell

A powerful script execution and variable extraction framework with SSH remote execution and local execution support, pipeline orchestration, and flexible variable extraction via regex.

## Features

- **Hybrid Execution**: Execute scripts both locally and remotely via SSH
- **Variable Extraction**: Extract variables from command outputs using regex patterns
- **Pipeline Orchestration**: Chain multiple steps with variable passing between them
- **Real-time Output**: Stream command outputs in real-time
- **Flexible Configuration**: YAML-based configuration with support for multiple servers
- **Error Handling**: Robust error handling and logging

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
net-shell = "0.2.0"
```

Or install the binary:

```bash
cargo install net-shell
```

## Quick Start

### Basic Configuration

Create a `config.yaml` file:

```yaml
# Global variables
variables:
  master_ip: "192.168.0.199"
  app_name: "myapp"
  version: "1.0.0"

# SSH client configurations
clients:
  mac_server:
    name: "mac_server"
    execution_method: ssh
    ssh_config:
      host: "{{ master_ip }}"
      port: 22
      username: "li"
      private_key_path: "/Users/li/.ssh/id_rsa"
      timeout_seconds: 2 

# Pipeline definitions
pipelines:
  - name: "deploy_app"
    steps:
      - name: "get_system_info"
        script: "./scripts/get_system_info.sh"
        timeout_seconds: 5
        servers:
          - mac_server
        extract:
          - name: "os_version_num"
            patterns: 
              - "OS Version: (.+)"
              - "(\\d+\\.\\d+\\.\\d+)"
            source: "stdout"
          - name: "os_version"
            patterns: ["OS Version: (.+)"]
            source: "stdout"
          - name: "hostname"
            patterns: ["Hostname: (.+)"]
            source: "stdout"
          - name: "current_user"
            patterns: ["Current user: (.+)"]
            source: "stdout"
      
      - name: "deploy_application"
        script: "./scripts/deploy.sh"
        timeout_seconds: 10
        servers:
          - mac_server
        extract:
          - name: "deploy_path"
            patterns: ["Deployed to: (.+)"]
            source: "stdout"
          - name: "deploy_status"
            patterns: ["Status: (.+)"]
            source: "stdout"
      
      - name: "verify_deployment"
        script: "./scripts/verify.sh"
        timeout_seconds: 5
        servers:
          - mac_server
        extract:
          - name: "service_status"
            patterns: ["Service Status: (.+)"]
            source: "stdout"
          - name: "verification_time"
            patterns: ["Verification completed at (.+)"]
            source: "stdout"

  - name: "install_docker"
    steps:
      - name: "mock install docker"
        script: "./scripts/mock_install_docker.sh"
        timeout_seconds: 3
        servers:
          - mac_server
        extract:
          - name: "docker_version"
            patterns: ["Docker version: (.+)"]
            source: "stdout"
          - name: "install_path"
            patterns: ["Installed to: (.+)"]
            source: "stdout"
      
      - name: "start_docker"
        script: "./scripts/mock_start_docker.sh"
        timeout_seconds: 3
        servers:
          - mac_server
        extract:
          - name: "docker_status"
            patterns: ["Docker status: (.+)"]
            source: "stdout"
          - name: "docker_pid"
            patterns: ["Docker PID: (\\d+)"]
            source: "stdout"

  - name: "install_docker_compose"
    steps:
      - name: "mock install docker compose"
        script: "./scripts/mock_install_docker_compose.sh"
        servers:
          - mac_server
        extract:
          - name: "compose_version"
            patterns: ["Docker Compose version: (.+)"]
            source: "stdout"
          - name: "compose_path"
            patterns: ["Compose installed to: (.+)"]
            source: "stdout"

default_timeout: 60
```

### Local Execution

For local execution, simply omit the `servers` field or leave it empty:

```yaml
steps:
  - name: "local_check"
    script: |
      echo "Running locally on: $(hostname)"
      echo "Current user: $(whoami)"
      echo "Working directory: $(pwd)"
    extract:
      - name: "local_hostname"
        patterns: ["Running locally on: (.+)"]
        source: "stdout"
```

### Mixed Local and Remote Execution

You can mix local and remote steps in the same pipeline:

```yaml
steps:
  - name: "local_prep"
    script: |
      echo "Preparing locally..."
      echo "Timestamp: $(date)"
    extract:
      - name: "timestamp"
        patterns: ["Timestamp: (.+)"]
        source: "stdout"

  - name: "remote_execution"
    script: |
      echo "Executing remotely with timestamp: ${timestamp}"
      echo "Remote host: $(hostname)"
    servers:
      - mac_server
```

## Usage

### Command Line

Run with default configuration (`config.yaml`):

```bash
cargo run
```

Or specify a custom configuration file:

```bash
cargo run config_custom.yaml
```

### Programmatic Usage

```rust
use net_shell::{RemoteExecutor, models::*};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create variables
    let mut variables = HashMap::new();
    variables.insert("new_master_ip".to_string(), "192.168.1.100".to_string());

    // Create executor
    let executor = RemoteExecutor::from_yaml_file("config.yaml", Some(variables))?;
    
    // Execute all pipelines
    let results = executor.execute_all_pipelines().await?;
    
    Ok(())
}
```

## Configuration Reference

### Global Variables

Define global variables that can be referenced throughout the configuration:

```yaml
variables:
  master_ip: "192.168.0.199"
  app_name: "myapp"
  version: "1.0.0"
```

### Client Configuration

Define SSH clients for remote execution:

```yaml
clients:
  server_name:
    name: "server_name"
    execution_method: ssh
    ssh_config:
      host: "{{ master_ip }}"  # Can reference variables
      port: 22
      username: "user"
      password: "password"      # Or use private_key_path
      private_key_path: "/path/to/key"
      timeout_seconds: 30
```

### Pipeline Configuration

Each pipeline contains multiple steps:

```yaml
pipelines:
  - name: "pipeline_name"
    steps:
      - name: "step_name"
        script: "/path/to/script.sh"
        timeout_seconds: 30
        servers:
          - server_name
        extract:
          - name: "variable_name"
            patterns: ["Pattern: (.+)"]
            source: "stdout"  # or "stderr"
```

### Variable Extraction

Variables are extracted using regex patterns. Multiple patterns can be chained:

```yaml
extract:
  - name: "extracted_value"
    patterns: 
      - "Value: (.+)"
      - "(\\d+\\.\\d+\\.\\d+)"  # Extract version number
    source: "stdout"
```

## Examples

### Complex Variable Extraction

```yaml
steps:
  - name: "extract_multiple"
    script: |
      echo "CPU: $(nproc) cores"
      echo "Memory: $(free -h | grep Mem | awk '{print $2}')"
      echo "Disk: $(df -h / | tail -1 | awk '{print $4}') available"
    extract:
      - name: "cpu_cores"
        patterns: ["CPU: (.+) cores"]
        source: "stdout"
      - name: "memory_total"
        patterns: ["Memory: (.+)"]
        source: "stdout"
      - name: "disk_available"
        patterns: ["Disk: (.+) available"]
        source: "stdout"
```

### Pipeline with Variable Passing

```yaml
steps:
  - name: "step1"
    script: |
      echo "Step 1 output"
      echo "Value: 42"
    extract:
      - name: "step1_value"
        patterns: ["Value: (.+)"]
        source: "stdout"

  - name: "step2"
    script: |
      echo "Using value from step1: ${step1_value}"
      echo "Processing..."
    servers:
      - remote_server
```

## Real-time Output

The framework provides real-time output streaming with detailed event information:

```
🚀 [STEP_STARTED] deploy_app@get_system_info@mac_server: Starting step
[STDOUT] deploy_app@get_system_info@mac_server: System: Darwin
[STDOUT] deploy_app@get_system_info@mac_server: Hostname: macbook-pro
[VARS] Current variables: {"os_version": "macOS 13.0", "hostname": "macbook-pro"}
✅ [STEP_COMPLETED] deploy_app@get_system_info@mac_server: Step completed successfully
```

## Error Handling

The framework provides comprehensive error handling and logging:

- Connection timeouts
- Script execution failures
- Variable extraction errors
- Pipeline orchestration issues

All errors are logged with detailed context and stack traces for debugging.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## License

This project is licensed under the MIT License.
