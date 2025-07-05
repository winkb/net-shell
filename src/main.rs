// 模块声明
pub mod config;
pub mod executor;
pub mod models;
pub mod ssh;

// 重新导出主要类型，方便外部使用
pub use executor::RemoteExecutor;
pub use models::*;

use std::sync::Arc;
use tracing_subscriber;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_parsing() {
        let yaml_content = r#"
clients:
  server1:
    name: "server1"
    execution_method: ssh
    ssh_config:
      host: "192.168.1.100"
      port: 22
      username: "user"
      password: "password"
      timeout_seconds: 30
pipelines:
  - name: "test_pipeline"
    steps:
      - name: "test_step"
        script: "echo 'test'"
        weight: 1
        servers:
          - server1
default_timeout: 60
"#;

        let executor = RemoteExecutor::from_yaml_str(yaml_content).unwrap();
        assert_eq!(executor.get_available_clients().len(), 1);
        assert!(executor.client_exists("server1"));
        assert_eq!(executor.get_available_pipelines().len(), 1);
        assert!(executor.pipeline_exists("test_pipeline"));
    }
}

// 主函数用于演示实时输出功能
#[tokio::main]
async fn main() {
    // 初始化日志
    tracing_subscriber::fmt::init();

    // 读取config.yaml
    let config = match RemoteExecutor::from_yaml_file("config.yaml") {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load configuration: {:#?}", e);
            return;
        }
    };
    
    // 显示可用的流水线
    println!("Available pipelines: {:?}", config.get_available_pipelines());
    
    // 创建实时输出回调函数
    let output_callback = Box::new(|event: OutputEvent| {
        let output_type = match event.output_type {
            OutputType::Stdout => "STDOUT",
            OutputType::Stderr => "STDERR",
        };
        
        println!("[{}] {}@{}@{}: {}", 
                output_type, 
                event.pipeline_name,
                event.step_name,
                event.server_name, 
                event.content);
    });

    // 执行所有流水线（支持实时输出）
    let results = config.execute_all_pipelines_with_realtime_output(
        Some(Arc::new(output_callback))
    ).await;



    match results {
        Ok(pipeline_results) => {
            println!("\n=== All Pipelines Execution Summary ===");
            println!("Total pipelines executed: {}", pipeline_results.len());
            
            for pipeline_result in pipeline_results {
                println!("\n--- Pipeline: {} ---", pipeline_result.pipeline_name);
                println!("Overall success: {}", pipeline_result.overall_success);
                println!("Total execution time: {}ms", pipeline_result.total_execution_time_ms);
                
                for step_result in pipeline_result.step_results {
                    println!("  Step: {}", step_result.step_name);
                    println!("  Server: {}", step_result.server_name);
                    println!("  Step success: {}", step_result.overall_success);
                    println!("  Step execution time: {}ms", step_result.execution_time_ms);
                    println!("  Script: {}", step_result.execution_result.script);
                    println!("  Exit code: {}", step_result.execution_result.exit_code);
                    println!("  Success: {}", step_result.execution_result.success);
                    
                    if !step_result.execution_result.stdout.is_empty() {
                        println!("  Final stdout: {}", step_result.execution_result.stdout.trim());
                    }
                    if !step_result.execution_result.stderr.is_empty() {
                        println!("  Final stderr: {}", step_result.execution_result.stderr.trim());
                    }
                }
            }
        },
        Err(e) => {
            println!("Pipeline execution failed: {:#?}", e);
        },
    }
} 