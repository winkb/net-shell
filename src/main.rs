use ai_demo::RemoteExecutor;
use tracing_subscriber;

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

    // 执行流水线
    let result = config.execute_pipeline("install_docker").await;

    match result {
        Ok(result) => {
            println!("Pipeline execution completed!");
            println!("Overall success: {}", result.overall_success);
            println!("Total execution time: {}ms", result.total_execution_time_ms);

            for step_result in result.step_results {
                println!("\nStep: {}", step_result.step_name);
                println!("Step success: {}", step_result.overall_success);
                println!("Step execution time: {}ms", step_result.execution_time_ms);

                for (server, exec_result) in step_result.server_results {
                    println!("  Script: {}", exec_result.script);
                    println!("  Server {}: exit_code={}, success={}",
                        server, exec_result.exit_code, exec_result.success);
                    if !exec_result.stdout.is_empty() {
                        println!("    Stdout: {}", exec_result.stdout.trim());
                    }
                    if !exec_result.stderr.is_empty() {
                        println!("    Stderr: {}", exec_result.stderr.trim());
                    }
                }
            }
        },
        Err(e) => {
            println!("Pipeline execution failed: {:#?}", e);
        },
    }
} 