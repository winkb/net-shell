use anyhow::{Context, Result};
use futures::future::join_all;
use std::clone;
use std::path::Path;
use std::sync::Arc;
use tracing::{error, info};

use crate::config::ConfigManager;
use crate::models::{
    ClientConfig, ExecutionMethod, ExecutionResult, PipelineExecutionResult, 
    RemoteExecutionConfig, Step, StepExecutionResult, OutputCallback, OutputEvent
};
use crate::ssh::SshExecutor;

/// 远程执行器
pub struct RemoteExecutor {
    config: RemoteExecutionConfig,
}

impl RemoteExecutor {
    /// 从YAML文件创建执行器
    pub fn from_yaml_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let config = ConfigManager::from_yaml_file(path)?;
        ConfigManager::validate_config(&config)?;
        Ok(Self { config })
    }

    /// 从YAML字符串创建执行器
    pub fn from_yaml_str(yaml_content: &str) -> Result<Self> {
        let config = ConfigManager::from_yaml_str(yaml_content)?;
        ConfigManager::validate_config(&config)?;
        Ok(Self { config })
    }

    /// 执行指定的流水线（支持实时输出）
    pub async fn execute_pipeline_with_realtime_output(
        &self, 
        pipeline_name: &str,
        output_callback: Option<OutputCallback>,
        log_callback: Option<OutputCallback>
    ) -> Result<PipelineExecutionResult> {
        let pipeline = self.config.pipelines.iter()
            .find(|p| p.name == pipeline_name)
            .ok_or_else(|| anyhow::anyhow!("Pipeline '{}' not found", pipeline_name))?;

        let start_time = std::time::Instant::now();
        let mut all_step_results = Vec::new();

        // 发送开始执行流水线的日志
        if let Some(callback) = &log_callback {
            let event = OutputEvent {
                pipeline_name: pipeline.name.clone(),
                server_name: "system".to_string(),
                step_name: "pipeline_start".to_string(),
                output_type: crate::models::OutputType::Log,
                content: format!("开始执行流水线: {}", pipeline.name),
                timestamp: std::time::Instant::now(),
            };
            callback(event);
        }

        info!("Starting pipeline: {}", pipeline.name);

        // 按顺序执行每个步骤（串行）
        for step in &pipeline.steps {
            // 发送开始执行步骤的日志
            if let Some(callback) = &log_callback {
                let event = OutputEvent {
                    pipeline_name: pipeline.name.clone(),
                    server_name: "system".to_string(),
                    step_name: step.name.clone(),
                    output_type: crate::models::OutputType::Log,
                    content: format!("开始执行步骤: {} ({} 个服务器)", step.name, step.servers.len()),
                    timestamp: std::time::Instant::now(),
                };
                callback(event);
            }

            info!("Starting step: {} on {} servers", step.name, step.servers.len());
            
            // 同一步骤内的所有服务器并发执行
            let step_results = self.execute_step_with_realtime_output(step, pipeline_name, output_callback.as_ref()).await?;
            
            // 检查步骤是否成功（所有服务器都成功才算成功）
            let step_success = step_results.iter().all(|r| r.execution_result.success);
            
            // 添加步骤结果
            all_step_results.extend(step_results);

            // 发送步骤完成日志
            if let Some(callback) = &log_callback {
                let status = if step_success { "成功" } else { "失败" };
                let event = OutputEvent {
                    pipeline_name: pipeline.name.clone(),
                    server_name: "system".to_string(),
                    step_name: step.name.clone(),
                    output_type: crate::models::OutputType::Log,
                    content: format!("步骤完成: {} ({})", step.name, status),
                    timestamp: std::time::Instant::now(),
                };
                callback(event);
            }

            // 如果步骤失败，可以选择是否继续执行后续步骤
            if !step_success {
                info!("Step '{}' failed, stopping pipeline", step.name);
                break;
            }
            
            info!("Step '{}' completed successfully", step.name);
        }

        let total_time = start_time.elapsed().as_millis() as u64;
        let overall_success = all_step_results.iter().all(|r| r.execution_result.success);

        // 发送流水线完成日志
        if let Some(callback) = &log_callback {
            let status = if overall_success { "成功" } else { "失败" };
            let event = OutputEvent {
                pipeline_name: pipeline.name.clone(),
                server_name: "system".to_string(),
                step_name: "pipeline_complete".to_string(),
                output_type: crate::models::OutputType::Log,
                content: format!("流水线完成: {} ({}) - 总耗时: {}ms", pipeline.name, status, total_time),
                timestamp: std::time::Instant::now(),
            };
            callback(event);
        }

        Ok(PipelineExecutionResult {
            pipeline_name: pipeline.name.clone(),
            step_results: all_step_results,
            overall_success,
            total_execution_time_ms: total_time,
        })
    }

    /// 执行所有流水线（支持实时输出）
    pub async fn execute_all_pipelines_with_realtime_output(
        &self,
        output_callback: Option<OutputCallback>,
        log_callback: Option<OutputCallback>
    ) -> Result<Vec<PipelineExecutionResult>> {
        let mut results = Vec::new();
        
        // 发送开始执行所有流水线的日志
        if let Some(callback) = &log_callback {
            let event = OutputEvent {
                pipeline_name: "system".to_string(),
                server_name: "system".to_string(),
                step_name: "execution_start".to_string(),
                output_type: crate::models::OutputType::Log,
                content: format!("=== 远程脚本执行器 ==="),
                timestamp: std::time::Instant::now(),
            };
            callback(event);
            
            let event = OutputEvent {
                pipeline_name: "system".to_string(),
                server_name: "system".to_string(),
                step_name: "config_load_success".to_string(),
                output_type: crate::models::OutputType::Log,
                content: format!("配置加载成功，发现 {} 个流水线", self.config.pipelines.len()),
                timestamp: std::time::Instant::now(),
            };
            callback(event);
            
            let event = OutputEvent {
                pipeline_name: "system".to_string(),
                server_name: "system".to_string(),
                step_name: "execution_mode".to_string(),
                output_type: crate::models::OutputType::Log,
                content: format!("执行模式: 步骤串行执行，同一步骤内服务器并发执行"),
                timestamp: std::time::Instant::now(),
            };
            callback(event);
        }
        
        // 按顺序执行每个流水线（串行）
        for pipeline in &self.config.pipelines {
            // 发送开始执行流水线的日志
            if let Some(callback) = &log_callback {
                let event = OutputEvent {
                    pipeline_name: pipeline.name.clone(),
                    server_name: "system".to_string(),
                    step_name: "pipeline_start".to_string(),
                    output_type: crate::models::OutputType::Log,
                    content: format!("开始执行流水线: {}", pipeline.name),
                    timestamp: std::time::Instant::now(),
                };
                callback(event);
            }
            
            info!("Starting pipeline: {}", pipeline.name);
            
            let result = self.execute_pipeline_with_realtime_output(&pipeline.name, output_callback.as_ref().cloned(), log_callback.as_ref().cloned()).await?;
            let success = result.overall_success;
            results.push(result);
            
            // 如果流水线失败，可以选择是否继续执行后续流水线
            if !success {
                info!("Pipeline '{}' failed, stopping execution", pipeline.name);
                break;
            }
            
            info!("Pipeline '{}' completed successfully", pipeline.name);
        }
        
        Ok(results)
    }

    /// 执行指定的流水线（原有方法，保持兼容性）
    pub async fn execute_pipeline(&self, pipeline_name: &str) -> Result<PipelineExecutionResult> {
        self.execute_pipeline_with_realtime_output(pipeline_name, None, None).await
    }

    /// 执行单个步骤（支持实时输出）
    async fn execute_step_with_realtime_output(
        &self, 
        step: &Step,
        pipeline_name: &str,
        output_callback: Option<&OutputCallback>,
    ) -> Result<Vec<StepExecutionResult>> {
        let start_time = std::time::Instant::now();
        info!("Executing step: {} on {} servers", step.name, step.servers.len());

        let mut step_results = Vec::new();
        let mut futures = Vec::new();

        // 为每个服务器创建执行任务
        for server_name in &step.servers {
            if !self.client_exists(server_name) {
                return Err(anyhow::anyhow!("Server '{}' not found in configuration", server_name));
            }

            // 克隆必要的数据以避免生命周期问题
            let config = self.config.clone();
            let server_name = server_name.clone();
            let step_name = step.name.clone();
            let output_callback = output_callback.cloned();
            let clone_step = step.clone();
            let pipeline_name = pipeline_name.to_string();

            let future = tokio::spawn(async move {
                // 创建新的执行器实例
                let executor = RemoteExecutor { config };
                match executor.execute_script_with_realtime_output(&server_name, clone_step, &pipeline_name, output_callback).await {
                    Ok(result) => {
                        info!("Step '{}' on server '{}' completed with exit code: {}", 
                              step_name, server_name, result.exit_code);
                        Ok((server_name, result))
                    }
                    Err(e) => {
                        error!("Step '{}' on server '{}' failed: {}", step_name, server_name, e);
                        Err(e)
                    }
                }
            });

            futures.push(future);
        }

        // 等待所有执行完成
        let results = join_all(futures).await;
        
        for result in results {
            match result {
                Ok(Ok((server_name, execution_result))) => {
                    let success = execution_result.success;
                    step_results.push(StepExecutionResult {
                        step_name: step.name.clone(),
                        server_name,
                        execution_result,
                        overall_success: success,
                        execution_time_ms: start_time.elapsed().as_millis() as u64,
                    });
                }
                Ok(Err(e)) => {
                    return Err(e);
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Task execution failed: {}", e));
                }
            }
        }

        Ok(step_results)
    }

    /// 执行单个步骤（原有方法，保持兼容性）
    async fn execute_step(&self, step: &Step) -> Result<Vec<StepExecutionResult>> {
        self.execute_step_with_realtime_output(step, "unknown", None).await
    }

    /// 在指定客户端执行shell脚本（支持实时输出）
    pub async fn execute_script_with_realtime_output(
        &self, 
        client_name: &str, 
        step: Step,
        pipeline_name: &str,
        output_callback: Option<OutputCallback>
    ) -> Result<ExecutionResult> {
        // 检查脚本文件是否存在
        let script_path = Path::new(step.script.as_str());
        if !script_path.exists() {
            return Err(anyhow::anyhow!("Script '{}' not found", step.script));
        }

        let client_config = self.config
            .clients
            .get(client_name)
            .ok_or_else(|| anyhow::anyhow!("Client '{}' not found in configuration", client_name))?;

        match client_config.execution_method {
            ExecutionMethod::SSH => {
                self.execute_script_via_ssh_with_realtime_output(client_config, step, client_name, pipeline_name, output_callback).await
            }
            ExecutionMethod::WebSocket => {
                Err(anyhow::anyhow!("WebSocket execution not implemented yet"))
            }
        }
    }

    /// 通过SSH执行脚本（支持实时输出）
    async fn execute_script_via_ssh_with_realtime_output(
        &self, 
        client_config: &ClientConfig, 
        step: Step,
        server_name: &str,
        pipeline_name: &str,
        output_callback: Option<OutputCallback>
    ) -> Result<ExecutionResult> {
        let ssh_config = client_config.ssh_config.as_ref()
            .ok_or_else(|| anyhow::anyhow!("SSH configuration not found for client '{}'", client_config.name))?;

        let start_time = std::time::Instant::now();

        // 克隆数据以避免生命周期问题
        let ssh_config = ssh_config.clone();
        let script_content = step.script.to_string();
        let server_name = server_name.to_string();
        let pipeline_name = pipeline_name.to_string();

        // 在tokio的阻塞线程池中执行SSH操作
        let result = tokio::task::spawn_blocking(move || {
            SshExecutor::execute_script_with_realtime_output(
                &server_name, 
                &ssh_config, 
                &step,
                &pipeline_name,
                output_callback
            )
        }).await??;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(ExecutionResult {
            success: result.exit_code == 0,
            stdout: result.stdout,
            stderr: result.stderr,
            script: script_content,
            exit_code: result.exit_code,
            execution_time_ms: execution_time,
            error_message: result.error_message,
        })
    }

    /// 获取所有可用的客户端名称
    pub fn get_available_clients(&self) -> Vec<String> {
        self.config.clients.keys().cloned().collect()
    }

    /// 检查客户端是否存在
    pub fn client_exists(&self, client_name: &str) -> bool {
        self.config.clients.contains_key(client_name)
    }

    /// 获取所有可用的流水线名称
    pub fn get_available_pipelines(&self) -> Vec<String> {
        self.config.pipelines.iter().map(|p| p.name.clone()).collect()
    }

    /// 检查流水线是否存在
    pub fn pipeline_exists(&self, pipeline_name: &str) -> bool {
        self.config.pipelines.iter().any(|p| p.name == pipeline_name)
    }
} 