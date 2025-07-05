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
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    // 创建执行器
    let executor = RemoteExecutor::from_yaml_file("config.yaml")?;
    
    // 定义实时输出回调函数
    let output_callback = Arc::new(|event: models::OutputEvent| {
        match event.output_type {
            models::OutputType::Stdout => {
                println!("[STDOUT] {}@{}@{}: {}", 
                        event.pipeline_name,
                        event.step_name,
                        event.server_name, 
                        event.content);
            }
            models::OutputType::Stderr => {
                eprintln!("[STDERR] {}@{}@{}: {}", 
                         event.pipeline_name,
                         event.step_name,
                         event.server_name, 
                         event.content);
            }
            models::OutputType::Log => {
                println!("[LOG] {}@{}@{}: {}", 
                        event.pipeline_name,
                        event.step_name,
                        event.server_name, 
                        event.content);
            }
        }
    });

    // 执行所有流水线
    let results = executor.execute_all_pipelines_with_realtime_output(Some(output_callback.clone()), Some(output_callback)).await?;
    
    // 打印执行结果摘要
    println!("\n=== 执行结果摘要 ===");
    for result in &results {
        println!("\n流水线: {} ({})", result.pipeline_name, 
                 if result.overall_success { "成功" } else { "失败" });
        println!("总执行时间: {}ms", result.total_execution_time_ms);
        println!("步骤结果:");
        
        for step_result in &result.step_results {
            let status = if step_result.execution_result.success { "✅" } else { "❌" };
            println!("  {} [{}:{}] {} - {}ms", 
                     status,
                     result.pipeline_name,
                     step_result.step_name,
                     step_result.server_name,
                     step_result.execution_result.execution_time_ms);
        }
    }
    
    // 统计总体结果
    let total_pipelines = results.len();
    let successful_pipelines = results.iter().filter(|r| r.overall_success).count();
    let total_steps = results.iter().map(|r| r.step_results.len()).sum::<usize>();
    let successful_steps = results.iter()
        .flat_map(|r| &r.step_results)
        .filter(|r| r.execution_result.success)
        .count();
    
    println!("\n=== 总体统计 ===");
    println!("流水线: {}/{} 成功", successful_pipelines, total_pipelines);
    println!("步骤: {}/{} 成功", successful_steps, total_steps);
    
    Ok(())
} 