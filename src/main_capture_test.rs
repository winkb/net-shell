use ai_demo::{
    executor::RemoteExecutor,
    models::{OutputEvent, OutputType},
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    // 创建执行器
    let executor = RemoteExecutor::from_yaml_file("config_capture_groups_test.yaml")?;
    
    // 定义实时输出回调函数
    let output_callback = Arc::new(|event: ai_demo::models::OutputEvent| {
        let step_info = match &event.step {
            Some(step) => format!("{}", step.name),
            None => "system".to_string(),
        };
        
        match event.output_type {
            ai_demo::models::OutputType::Stdout => {
                println!("[STDOUT] {}@{}@{}: {}", 
                        event.pipeline_name,
                        step_info,
                        event.server_name, 
                        event.content);
            }
            ai_demo::models::OutputType::Stderr => {
                eprintln!("[STDERR] {}@{}@{}: {}", 
                         event.pipeline_name,
                         step_info,
                         event.server_name, 
                         event.content);
            }
            ai_demo::models::OutputType::Log => {
                println!("[LOG] {}@{}@{}: {}", 
                        event.pipeline_name,
                        step_info,
                        event.server_name, 
                        event.content);
            }
        }
        
        // 显示当前变量状态
        if !event.variables.is_empty() {
            println!("[VARS] Current variables: {:?}", event.variables);
        }
        
        // 显示步骤详细信息（如果有）
        if let Some(step) = &event.step {
            println!("[STEP] Step details: name={}, script={}, servers={:?}, timeout={:?}, extract_rules={:?}", 
                    step.name, step.script, step.servers, step.timeout_seconds, step.extract);
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