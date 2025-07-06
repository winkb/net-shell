use anyhow::{Context, Result};
use serde_yaml;
use std::path::Path;
use std::collections::HashMap;

use crate::models::RemoteExecutionConfig;
use crate::vars::VariableManager;

/// 配置管理器
pub struct ConfigManager;

impl ConfigManager {
    /// 从YAML文件加载配置（不处理变量替换）
    pub fn from_yaml_file_raw<P: AsRef<Path>>(path: P) -> Result<RemoteExecutionConfig> {
        let content = std::fs::read_to_string(path)
            .context("Failed to read YAML configuration file")?;
        
        Self::from_yaml_str_raw(&content)
    }

    /// 从YAML字符串加载配置（不处理变量替换）
    pub fn from_yaml_str_raw(yaml_content: &str) -> Result<RemoteExecutionConfig> {
        let config: RemoteExecutionConfig = serde_yaml::from_str(yaml_content)
            .context("Failed to parse YAML configuration")?;
        
        Ok(config)
    }

    /// 从YAML文件加载配置并应用变量替换
    pub fn from_yaml_file_with_variables<P: AsRef<Path>>(path: P, variable_manager: &VariableManager) -> Result<RemoteExecutionConfig> {
        let content = std::fs::read_to_string(path)
            .context("Failed to read YAML configuration file")?;
        
        Self::from_yaml_str_with_variables(&content, variable_manager)
    }

    /// 从YAML字符串加载配置并应用变量替换
    pub fn from_yaml_str_with_variables(yaml_content: &str, variable_manager: &VariableManager) -> Result<RemoteExecutionConfig> {
        // 对整个YAML内容进行变量替换（当作字符串处理）
        let replaced_content = variable_manager.replace_variables(yaml_content);
        
        // 解析替换后的内容为最终配置
        let config: RemoteExecutionConfig = serde_yaml::from_str(&replaced_content)
            .context("Failed to parse YAML configuration after variable replacement")?;
        
        Ok(config)
    }

    /// 提取YAML中的初始变量
    pub fn extract_initial_variables(yaml_content: &str) -> Result<Option<HashMap<String, String>>> {
        let yaml_value: serde_yaml::Value = serde_yaml::from_str(yaml_content)
            .context("Failed to parse YAML for variable extraction")?;
        
        let initial_variables = if let Some(vars) = yaml_value.get("variables") {
            if let Ok(vars_map) = serde_yaml::from_value::<HashMap<String, String>>(vars.clone()) {
                Some(vars_map)
            } else {
                None
            }
        } else {
            None
        };
        
        Ok(initial_variables)
    }

    /// 从YAML文件加载配置（保持向后兼容）
    pub fn from_yaml_file<P: AsRef<Path>>(path: P) -> Result<RemoteExecutionConfig> {
        let content = std::fs::read_to_string(path)
            .context("Failed to read YAML configuration file")?;
        
        Self::from_yaml_str(&content)
    }

    /// 从YAML字符串加载配置（保持向后兼容）
    pub fn from_yaml_str(yaml_content: &str) -> Result<RemoteExecutionConfig> {
        // 提取初始变量
        let initial_variables = Self::extract_initial_variables(yaml_content)?;
        
        // 创建变量管理器
        let variable_manager = VariableManager::new(initial_variables);
        
        // 应用变量替换
        Self::from_yaml_str_with_variables(yaml_content, &variable_manager)
    }

    /// 验证配置的有效性
    pub fn validate_config(config: &RemoteExecutionConfig) -> Result<()> {
        // 检查是否有客户端配置
        if config.clients.is_empty() {
            return Err(anyhow::anyhow!("No clients configured"));
        }

        // 检查是否有流水线配置
        if config.pipelines.is_empty() {
            return Err(anyhow::anyhow!("No pipelines configured"));
        }

        // 检查每个流水线的步骤
        for pipeline in &config.pipelines {
            if pipeline.steps.is_empty() {
                return Err(anyhow::anyhow!("Pipeline '{}' has no steps", pipeline.name));
            }

            for step in &pipeline.steps {
                if step.servers.is_empty() {
                    return Err(anyhow::anyhow!("Step '{}' in pipeline '{}' has no servers", 
                                              step.name, pipeline.name));
                }

                // 检查步骤中引用的服务器是否存在
                for server in &step.servers {
                    if !config.clients.contains_key(server) {
                        return Err(anyhow::anyhow!("Server '{}' referenced in step '{}' not found in clients", 
                                                  server, step.name));
                    }
                }
            }
        }

        Ok(())
    }
} 