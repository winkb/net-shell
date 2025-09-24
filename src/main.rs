// 模块声明
pub mod config;
pub mod executor;
pub mod models;
pub mod ssh;
pub mod vars;

// 重新导出主要类型，方便外部使用
pub use executor::RemoteExecutor;
pub use models::*;


use std::{collections::HashMap, sync::Arc};
use tracing_subscriber;
use std::env;

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

        let executor = RemoteExecutor::from_yaml_str(yaml_content, None).unwrap();
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

    // 解析命令行参数，支持指定配置文件路径
    let args: Vec<String> = env::args().collect();
    let config_path = if args.len() > 1 {
        &args[1]
    } else {
        "config.yaml"
    };

    let mut variables = HashMap::new();
    variables.insert("new_master_ip".to_string(), "192.168.1.100".to_string());

    // 创建执行器
    let mut executor = RemoteExecutor::from_yaml_file(config_path, Some(variables))?;
    
    // 定义实时输出回调函数
    let output_callback = Arc::new(|event: models::OutputEvent| {
        let step = event.step.clone();
        
        match event.output_type {
            models::OutputType::Stdout => {
                println!("[STDOUT] {}@{}@{}: {}", 
                        event.pipeline_name,
                        step.name,
                        event.server_name, 
                        event.content);
            }
            models::OutputType::Stderr => {
                eprintln!("[STDERR] {}@{}@{}: {}, script:[{}]", 
                         event.pipeline_name,
                         step.name,
                         event.server_name, 
                         event.content,
                         event.step.script
                        );
            }
            models::OutputType::Log => {
                println!("[LOG] {}@{}@{}: {}", 
                        event.pipeline_name,
                        step.name,
                        event.server_name, 
                        event.content);
            }
            models::OutputType::StepStarted => {
                println!("🚀 [STEP_STARTED] {}@{}@{}: {}", 
                        event.pipeline_name,
                        step.name,
                        event.server_name, 
                        event.content);
            }
            models::OutputType::StepCompleted => {
                println!("✅ [STEP_COMPLETED] {}@{}@{}: {}", 
                        event.pipeline_name,
                        step.name,
                        event.server_name, 
                        event.content);
            }
        }
        
        // 显示当前变量状态
        if !event.variables.is_empty() {
            println!("[VARS] Current variables: {:?}", event.variables);
        }
        
    });

    // 执行所有流水线
    let res= executor.execute_all_pipelines_with_realtime_output(Some(output_callback.clone()), Some(output_callback)).await?;
    let results = res.pipeline_results;
 
    // 打印执行结果摘要
    println!("\n=== 执行结果摘要 ===");
    for result in &results {
        println!("\n流水线: {} ({})", result.title, 
                 if result.overall_success { "成功" } else { "失败" });
        println!("总执行时间: {}ms", result.total_execution_time_ms);
        println!("步骤结果:");
        
        for step_result in &result.step_results {
            let status = if step_result.execution_result.success { "✅" } else { "❌" };
            println!("  {} [{}:{}] {} - {}ms, {}", 
                     status,
                     result.title,
                     step_result.title,
                     step_result.server_name,
                     step_result.execution_result.execution_time_ms,
                     step_result.execution_result.error_message.clone().unwrap_or_default(),
                    );
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

    if !res.success {
        println!("执行失败: {}", res.reason);
        return Ok(());
    }
    
    Ok(())
} 