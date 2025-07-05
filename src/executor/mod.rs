use anyhow::{Context, Result};
use futures::future::join_all;
use std::path::Path;
use tracing::{error, info};

use crate::config::ConfigManager;
use crate::models::{
    ClientConfig, ExecutionMethod, ExecutionResult, PipelineExecutionResult, 
    RemoteExecutionConfig, Step, StepExecutionResult
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

    /// 执行指定的流水线
    pub async fn execute_pipeline(&self, pipeline_name: &str) -> Result<PipelineExecutionResult> {
        let pipeline = self.config.pipelines.iter()
            .find(|p| p.name == pipeline_name)
            .ok_or_else(|| anyhow::anyhow!("Pipeline '{}' not found", pipeline_name))?;

        let start_time = std::time::Instant::now();
        let mut step_results = Vec::new();

        info!("Starting pipeline: {}", pipeline.name);

        // 按顺序执行每个步骤
        for step in &pipeline.steps {
            let step_result = self.execute_step(step).await?;
            let step_success = step_result.overall_success;
            step_results.push(step_result);

            // 如果步骤失败，可以选择是否继续执行后续步骤
            if !step_success {
                info!("Step '{}' failed, stopping pipeline", step.name);
                break;
            }
        }

        let total_time = start_time.elapsed().as_millis() as u64;
        let overall_success = step_results.iter().all(|r| r.overall_success);

        Ok(PipelineExecutionResult {
            pipeline_name: pipeline.name.clone(),
            step_results,
            overall_success,
            total_execution_time_ms: total_time,
        })
    }

    /// 执行单个步骤
    async fn execute_step(&self, step: &Step) -> Result<StepExecutionResult> {
        let start_time = std::time::Instant::now();
        info!("Executing step: {} on {} servers", step.name, step.servers.len());

        let mut server_results = std::collections::HashMap::new();
        let mut futures = Vec::new();

        // 为每个服务器创建执行任务
        for server_name in &step.servers {
            if !self.client_exists(server_name) {
                return Err(anyhow::anyhow!("Server '{}' not found in configuration", server_name));
            }

            // 克隆必要的数据以避免生命周期问题
            let config = self.config.clone();
            let server_name = server_name.clone();
            let script = step.script.clone();
            let step_name = step.name.clone();

            let future = tokio::spawn(async move {
                // 创建新的执行器实例
                let executor = RemoteExecutor { config };
                match executor.execute_script(&server_name, &script).await {
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
                    server_results.insert(server_name, execution_result);
                }
                Ok(Err(e)) => {
                    return Err(e);
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Task execution failed: {}", e));
                }
            }
        }

        let execution_time = start_time.elapsed().as_millis() as u64;
        let overall_success = server_results.values().all(|r| r.success);

        Ok(StepExecutionResult {
            step_name: step.name.clone(),
            server_results,
            overall_success,
            execution_time_ms: execution_time,
        })
    }

    /// 在指定客户端执行shell脚本
    pub async fn execute_script(&self, client_name: &str, script: &str) -> Result<ExecutionResult> {
        // 检查脚本文件是否存在
        let script_path = Path::new(script);
        if !script_path.exists() {
            return Err(anyhow::anyhow!("Script '{}' not found", script));
        }

        let client_config = self.config
            .clients
            .get(client_name)
            .ok_or_else(|| anyhow::anyhow!("Client '{}' not found in configuration", client_name))?;

        match client_config.execution_method {
            ExecutionMethod::SSH => {
                self.execute_script_via_ssh(client_config, script).await
            }
            ExecutionMethod::WebSocket => {
                Err(anyhow::anyhow!("WebSocket execution not implemented yet"))
            }
        }
    }

    /// 通过SSH执行脚本
    async fn execute_script_via_ssh(&self, client_config: &ClientConfig, script: &str) -> Result<ExecutionResult> {
        let ssh_config = client_config.ssh_config.as_ref()
            .ok_or_else(|| anyhow::anyhow!("SSH configuration not found for client '{}'", client_config.name))?;

        let start_time = std::time::Instant::now();

        // 克隆数据以避免生命周期问题
        let ssh_config = ssh_config.clone();
        let script_content = script.to_string();

        // 在tokio的阻塞线程池中执行SSH操作
        let result = tokio::task::spawn_blocking(move || {
            SshExecutor::execute_script(&ssh_config, &script_content)
        }).await??;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(ExecutionResult {
            success: result.exit_code == 0,
            stdout: result.stdout,
            stderr: result.stderr,
            script: script.to_string(),
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