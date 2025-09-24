use anyhow::{Context, Result};
use futures::future::join_all;
use std::collections::HashMap;
use std::path::Path;
use tracing::{error, info};

use crate::config::ConfigManager;
use crate::models::{
    ClientConfig, ExecutionMethod, ExecutionResult, PipelineExecutionResult, 
    RemoteExecutionConfig, Step, StepExecutionResult, OutputCallback, OutputEvent
};
use crate::ssh::SshExecutor;
use crate::ssh::local::LocalExecutor;
use crate::vars::VariableManager;
use crate::ShellExecutionResult;

/// 远程执行器
pub struct RemoteExecutor {
    config: RemoteExecutionConfig,
    variable_manager: VariableManager,
}

impl RemoteExecutor {


    /// 从YAML文件创建执行器
    pub fn from_yaml_file<P: AsRef<Path>>(path: P, variables: Option<HashMap<String, String>>) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .context("Failed to read YAML configuration file")?;
        
        Self::from_yaml_str(&content, variables)
    }

    /// 从YAML字符串创建执行器
    pub fn from_yaml_str(yaml_content: &str, variables: Option<HashMap<String, String>>) -> Result<Self> {
        // 提取初始变量
        let initial_variables = ConfigManager::extract_initial_variables(yaml_content)?;

        // 合并变量
        let mut all_variables = HashMap::new();

        if let Some(v) = initial_variables {
            all_variables.extend(v);
        }

        if let Some(v) = variables {
            all_variables.extend(v);
        }
        
        // 创建变量管理器
        let variable_manager = VariableManager::new(Some(all_variables));
        
        // 应用变量替换解析配置
        let config = ConfigManager::from_yaml_str_with_variables(yaml_content, &variable_manager)?;
        ConfigManager::validate_config(&config)?;
        
        Ok(Self { config, variable_manager})
    }

    /// 执行指定的流水线（支持实时输出）
    pub async fn execute_pipeline_with_realtime_output(
        &mut self, // 需要可变引用
        pipeline_name: &str,
        output_callback: Option<OutputCallback>,
        log_callback: Option<OutputCallback>
    ) -> Result<PipelineExecutionResult> {
        let pipeline = self.config.pipelines.iter()
            .find(|p| p.name == pipeline_name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Pipeline '{}' not found", pipeline_name))?;

        let pipeline_name = pipeline.name.clone();
        let steps: Vec<Step> = pipeline.steps.clone();
        let start_time = std::time::Instant::now();
        let mut all_step_results = Vec::new();

        // 发送开始执行流水线的日志
        if let Some(callback) = &log_callback {
            let event = OutputEvent {
                pipeline_name: pipeline_name.clone(),
                server_name: "system".to_string(),
                step: Step::default(), // 流水线开始事件没有具体的步骤
                output_type: crate::models::OutputType::Log,
                content: format!("开始执行流水线: {}", pipeline_name),
                timestamp: std::time::Instant::now(),
                variables: self.variable_manager.get_variables().clone(),
            };
            callback(event);
        }

        info!("Starting pipeline: {}", pipeline_name);

        // 按顺序执行每个步骤（串行）
        for step in steps {
            // 合并 step 级变量到全局变量（优先级高）
            let mut step_var_keys = Vec::new();
            if let Some(vars) = &step.variables {
                for (k, v) in vars {
                    self.variable_manager.set_variable(k.clone(), v.clone());
                    step_var_keys.push(k.clone());
                }
            }
            // 对当前步骤应用变量替换
            let mut step_with_variables = step.clone();
            step_with_variables.script = self.variable_manager.replace_variables(&step.script);
            
            // 发送步骤开始事件
            if let Some(callback) = &output_callback {
                let event = OutputEvent {
                    pipeline_name: pipeline_name.clone(),
                    server_name: "system".to_string(),
                    step: step.clone(), // 传递完整的Step对象
                    output_type: crate::models::OutputType::StepStarted,
                    content: format!("开始执行步骤: {} ({} 个服务器)", step.name, step.servers.len()),
                    timestamp: std::time::Instant::now(),
                    variables: self.variable_manager.get_variables().clone(),
                };
                callback(event);
            }
            
            // 发送开始执行步骤的日志
            if let Some(callback) = &log_callback {
                let event = OutputEvent {
                    pipeline_name: pipeline_name.clone(),
                    server_name: "system".to_string(),
                    step: step.clone(), // 传递完整的Step对象
                    output_type: crate::models::OutputType::Log,
                    content: format!("开始执行步骤: {} ({} 个服务器)", step.name, step.servers.len()),
                    timestamp: std::time::Instant::now(),
                    variables: self.variable_manager.get_variables().clone(),
                };
                callback(event);
            }

            info!("Starting step: {} on {} servers", step.name, step.servers.len());
            
            // 同一步骤内的所有服务器并发执行
            let step_results = self.execute_step_with_realtime_output(&step_with_variables, pipeline_name.as_str(), output_callback.as_ref()).await?;
            
            // 检查步骤是否成功（所有服务器都成功才算成功）
            let step_success = step_results.iter().all(|r| r.execution_result.success);
            
            // 添加步骤结果
            all_step_results.extend(step_results);

            // 发送步骤完成事件
            if let Some(callback) = &output_callback {
                let status = if step_success { "成功" } else { "失败" };
                let event = OutputEvent {
                    pipeline_name: pipeline_name.clone(),
                    server_name: "system".to_string(),
                    step: step.clone(), // 传递完整的Step对象
                    output_type: crate::models::OutputType::StepCompleted,
                    content: format!("步骤完成: {} ({})", step.name, status),
                    timestamp: std::time::Instant::now(),
                    variables: self.variable_manager.get_variables().clone(),
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
                pipeline_name: pipeline_name.clone(),
                server_name: "system".to_string(),
                step: Step::default(), // 流水线完成事件没有具体的步骤
                output_type: crate::models::OutputType::Log,
                content: format!("流水线完成: {} ({}) - 总耗时: {}ms", pipeline_name, status, total_time),
                timestamp: std::time::Instant::now(),
                variables: self.variable_manager.get_variables().clone(),
            };
            callback(event);
        }

        Ok(PipelineExecutionResult {
            title: pipeline.title.clone().unwrap_or(pipeline_name.clone()),
            pipeline_name: pipeline_name.clone(),
            step_results: all_step_results,
            overall_success,
            total_execution_time_ms: total_time,
        })
    }

    /// 执行所有流水线（支持实时输出）
    pub async fn execute_all_pipelines_with_realtime_output(
        &mut self, // 需要可变引用
        output_callback: Option<OutputCallback>,
        log_callback: Option<OutputCallback>
    ) -> Result<ShellExecutionResult> {
        let mut results = Vec::new();
        
        // 发送开始执行所有流水线的日志
        if let Some(callback) = &log_callback {
            let event = OutputEvent {
                pipeline_name: "system".to_string(),
                server_name: "system".to_string(),
                step: Step::default(), // 系统级别事件没有具体步骤
                output_type: crate::models::OutputType::Log,
                content: format!("=== 远程脚本执行器 ==="),
                timestamp: std::time::Instant::now(),
                variables: self.variable_manager.get_variables().clone(),
            };
            callback(event);
            
            let event = OutputEvent {
                pipeline_name: "system".to_string(),
                server_name: "system".to_string(),
                step: Step::default(), // 系统级别事件没有具体步骤
                output_type: crate::models::OutputType::Log,
                content: format!("配置加载成功，发现 {} 个流水线", self.config.pipelines.len()),
                timestamp: std::time::Instant::now(),
                variables: self.variable_manager.get_variables().clone(),
            };
            callback(event);
            
            let event = OutputEvent {
                pipeline_name: "system".to_string(),
                server_name: "system".to_string(),
                step: Step::default(), // 系统级别事件没有具体步骤
                output_type: crate::models::OutputType::Log,
                content: format!("执行模式: 步骤串行执行，同一步骤内服务器并发执行"),
                timestamp: std::time::Instant::now(),
                variables: self.variable_manager.get_variables().clone(),
            };
            callback(event);
        }
        
        // 按顺序执行每个流水线（串行）
        let pipeline_names: Vec<String> = self.config.pipelines.iter().map(|p| p.name.clone()).collect();
        for pipeline_name in pipeline_names {
            // 发送开始执行流水线的日志
            if let Some(callback) = &log_callback {
                let event = OutputEvent {
                    pipeline_name: pipeline_name.clone(),
                    server_name: "system".to_string(),
                    step: Step::default(), // 流水线开始事件没有具体的步骤
                    output_type: crate::models::OutputType::Log,
                    content: format!("开始执行流水线: {}", pipeline_name),
                    timestamp: std::time::Instant::now(),
                    variables: self.variable_manager.get_variables().clone(),
                };
                callback(event);
            }
            info!("Starting pipeline: {}", pipeline_name);

            let result = self.execute_pipeline_with_realtime_output(&pipeline_name, output_callback.as_ref().cloned(), log_callback.as_ref().cloned()).await?;
            let success = result.overall_success;
            results.push(result);
            if !success {
                info!("Pipeline '{}' failed, stopping execution", pipeline_name);
                break;
            }
            info!("Pipeline '{}' completed successfully", pipeline_name);
        }
        
        Ok(ShellExecutionResult{
            success: true,
            reason: "ok".to_string(),
            pipeline_results: results,
        })
    }

    /// 执行指定的流水线（原有方法，保持兼容性）
    pub async fn execute_pipeline(&mut self, pipeline_name: &str) -> Result<PipelineExecutionResult> {
        self.execute_pipeline_with_realtime_output(pipeline_name, None, None).await
    }

    /// 执行单个步骤（支持实时输出）
    async fn execute_step_with_realtime_output(
        &mut self,
        step: &Step,
        pipeline_name: &str,
        output_callback: Option<&OutputCallback>
    ) -> Result<Vec<StepExecutionResult>> {
        let start_time = std::time::Instant::now();
        // Clone config at the start to avoid &self borrow conflicts
        let config = self.config.clone();
        let variable_manager = &mut self.variable_manager;
        
        // 检查是否有服务器配置
        if step.servers.is_empty() {
            // 本地执行
            info!("Executing step: {} locally (no servers specified)", step.name);
            let output_callback = output_callback.cloned();
            let step_clone = step.clone();
            let pipeline_name = pipeline_name.to_string();
            let step_name = step.name.clone();
            let mut variables = variable_manager.get_variables().clone();
            variables.insert("pipeline_name".to_string(), pipeline_name.clone());
            variables.insert("step_name".to_string(), step_name.clone());
            let execution_result = LocalExecutor::execute_script_with_realtime_output(
                &step_clone,
                &pipeline_name,
                &step_name,
                output_callback,
                variables,
            ).await?;
            let success = execution_result.success;
            // 提取变量（如果有extract规则）
            if let Some(extract_rules) = step.extract.clone() {
                if let Err(e) = variable_manager.extract_variables(&extract_rules, &execution_result) {
                    info!("Failed to extract variables from step '{}': {}", step.name, e);
                }
            }
            let step_result = StepExecutionResult {
                title: step.title.clone().unwrap_or(step.name.clone()),
                step_name: step.name.clone(),
                server_name: "localhost".to_string(),
                execution_result,
                overall_success: success,
                execution_time_ms: start_time.elapsed().as_millis() as u64,
            };
            return Ok(vec![step_result]);
        }
        
        // 远程执行（原有逻辑）
        info!("Executing step: {} on {} servers", step.name, step.servers.len());

        let mut step_results = Vec::new();
        let mut futures = Vec::new();
        // 用于收集所有服务器提取到的变量 (变量名, 变量值)
        let mut extracted_vars: Vec<(String, String)> = Vec::new();
        let clone_variable_manager = variable_manager.clone();

        // 为每个服务器创建执行任务
        let server_names: Vec<String> = step.servers.clone();
        for server_name in server_names {
            if !config.clients.contains_key(&server_name) {
                return Err(anyhow::anyhow!("Server '{}' not found in configuration", server_name));
            }

            // 克隆必要的数据以避免生命周期问题
            let config = config.clone();
            let step_name = step.name.clone();
            let output_callback = output_callback.cloned();
            let clone_step = step.clone();
            let pipeline_name = pipeline_name.to_string();
            let mut clone_variable_manager = clone_variable_manager.clone();
            clone_variable_manager.set_variable("pipeline_name".to_string(), pipeline_name.clone());
            clone_variable_manager.set_variable("step_name".to_string(), step_name.clone());

            let future = tokio::spawn(async move {
                // 创建新的执行器实例
                let executor = RemoteExecutor { 
                    config,
                    variable_manager:clone_variable_manager,
                };

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
                    // 提取变量（如果有extract规则）
                    if let Some(extract_rules) = step.extract.clone() {
                        // 提取变量到本地 map
                        let mut temp_vm = VariableManager::new(None);
                        if let Err(e) = temp_vm.extract_variables(&extract_rules, &execution_result) {
                            info!("Failed to extract variables from step '{}': {}", step.name, e);
                        } else {
                            for (k, v) in temp_vm.get_variables() {
                                extracted_vars.push((k.clone(), v.clone()));
                            }
                        }
                    }
                    
                    step_results.push(StepExecutionResult {
                        title: step.title.clone().unwrap_or(step.name.clone()),
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
        // 合并所有服务器提取到的变量到全局 variable_manager
        for (k, v) in extracted_vars {
            variable_manager.set_variable(k, v);
        }

        Ok(step_results)
    }

    /// 执行单个步骤（原有方法，保持兼容性）
    async fn execute_step(&mut self, step: &Step) -> Result<Vec<StepExecutionResult>> {
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
        let step_name = step.name.clone();
        let extract_rules = step.extract.clone();
        let variable_manager = self.variable_manager.clone();
        let clone_ssh_config = ssh_config.clone();

        // 在tokio的阻塞线程池中执行SSH操作
        let result = match tokio::task::spawn_blocking(move || {
            SshExecutor::execute_script_with_realtime_output(
                &server_name, 
                &ssh_config, 
                &step,
                &pipeline_name,
                &step_name,
                output_callback,
                variable_manager,
                extract_rules
            )
        }).await?.context(format!("join faield")) {
            Ok(v) => v,
            Err(e) => {

                let execution_time = start_time.elapsed().as_millis() as u64;
                return Ok(ExecutionResult{
                    success: false,
                    stdout: "".to_string(),
                    stderr: "".to_string(),
                    script: script_content,
                    exit_code: 0,
                    execution_time_ms: execution_time,
                    error_message: Some(format!("{:?}", e)),
                });
            }
        };

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