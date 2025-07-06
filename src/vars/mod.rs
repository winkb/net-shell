use std::collections::HashMap;
use regex::Regex;
use anyhow::{Result, Context};
use crate::models::{ExtractRule, ExecutionResult};

/// 变量管理器
#[derive(Debug, Clone)]
pub struct VariableManager {
    variables: HashMap<String, String>,
}

impl VariableManager {
    /// 创建新的变量管理器
    pub fn new(initial_variables: Option<HashMap<String, String>>) -> Self {
        Self {
            variables: initial_variables.unwrap_or_default(),
        }
    }

    /// 替换字符串中的变量占位符
    pub fn replace_variables(&self, content: &str) -> String {
        let mut result = content.to_string();
        
        // 替换 {{ variable_name }} 格式的变量
        for (key, value) in &self.variables {
            let placeholder = format!("{{{{ {} }}}}", key);
            result = result.replace(&placeholder, value);
        }
        
        result
    }

    /// 从执行结果中提取变量
    pub fn extract_variables(&mut self, extract_rules: &[ExtractRule], execution_result: &ExecutionResult) -> Result<()> {
        for rule in extract_rules {
            let source_content = match rule.source.as_str() {
                "stdout" => &execution_result.stdout,
                "stderr" => &execution_result.stderr,
                "exit_code" => &execution_result.exit_code.to_string(),
                _ => {
                    return Err(anyhow::anyhow!("Unknown extract source: {}", rule.source));
                }
            };

            // 使用正则表达式提取变量
            let regex = Regex::new(&rule.pattern)
                .context(format!("Invalid regex pattern: {}", rule.pattern))?;
            
            if let Some(captures) = regex.captures(source_content) {
                if let Some(value) = captures.get(1) {
                    self.variables.insert(rule.name.clone(), value.as_str().to_string());
                }
            }
        }
        
        Ok(())
    }

    /// 获取当前所有变量
    pub fn get_variables(&self) -> &HashMap<String, String> {
        &self.variables
    }

    /// 设置变量
    pub fn set_variable(&mut self, key: String, value: String) {
        self.variables.insert(key, value);
    }

    /// 获取变量值
    pub fn get_variable(&self, key: &str) -> Option<&String> {
        self.variables.get(key)
    }
} 