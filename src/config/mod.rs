use anyhow::{Context, Result};
use serde_yaml;
use std::path::Path;
use std::collections::HashMap;

use crate::models::RemoteExecutionConfig;

/// 配置管理器
pub struct ConfigManager;

impl ConfigManager {
    /// 解析模板字符串
    fn parse_template(content: &str) -> String {
        let mut result = content.to_string();
        
        // 简单的模板替换
        if result.contains("{{ context.master_ip }}") {
            result = result.replace("{{ context.master_ip }}", "8.8.8.8");
        }
        
        result
    }

    /// 从YAML文件加载配置
    pub fn from_yaml_file<P: AsRef<Path>>(path: P) -> Result<RemoteExecutionConfig> {
        let content = std::fs::read_to_string(path)
            .context("Failed to read YAML configuration file")?;
        
        // 解析模板
        let parsed_content = Self::parse_template(&content);
        
        let config: RemoteExecutionConfig = serde_yaml::from_str(&parsed_content)
            .context("Failed to parse YAML configuration")?;
        
        Ok(config)
    }

    /// 从YAML字符串加载配置
    pub fn from_yaml_str(yaml_content: &str) -> Result<RemoteExecutionConfig> {
        // 解析模板
        let parsed_content = Self::parse_template(yaml_content);
        
        let config: RemoteExecutionConfig = serde_yaml::from_str(&parsed_content)
            .context("Failed to parse YAML configuration")?;
        
        Ok(config)
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