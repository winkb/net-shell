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

            // 检查是否启用级联模式
            if rule.cascade {
                // 级联模式：前一个正则的匹配结果作为下一个正则的输入
                self.extract_with_cascade(rule, source_content)?;
            } else {
                // 普通模式：尝试多个正则表达式，按顺序匹配直到成功
                self.extract_with_fallback(rule, source_content)?;
            }
        }
        
        Ok(())
    }

    /// 级联模式提取：前一个正则的匹配结果作为下一个正则的输入
    /// 约定：始终获取第一个捕获组（第一个括号）的内容
    fn extract_with_cascade(&mut self, rule: &ExtractRule, source_content: &str) -> Result<()> {
        let mut current_content = source_content.to_string();
        let mut extracted_value = None;

        for (pattern_index, pattern) in rule.patterns.iter().enumerate() {
            let regex = Regex::new(pattern)
                .context(format!("Invalid regex pattern {} for rule '{}': {}", pattern_index + 1, rule.name, pattern))?;
            
            if let Some(captures) = regex.captures(&current_content) {
                // 约定：始终获取第一个捕获组（第一个括号）的内容
                let matched_value = if let Some(value) = captures.get(1) {
                    value.as_str().to_string()
                } else {
                    // 如果没有捕获组，记录警告并使用完整匹配
                    tracing::warn!("Pattern {} for rule '{}' has no capture groups, using full match: {}", 
                                  pattern_index + 1, rule.name, pattern);
                    if let Some(full_match) = captures.get(0) {
                        full_match.as_str().to_string()
                    } else {
                        continue;
                    }
                };
                
                if pattern_index == rule.patterns.len() - 1 {
                    // 最后一个正则，保存最终结果
                    extracted_value = Some(matched_value);
                    break;
                } else {
                    // 不是最后一个正则，将匹配结果作为下一个正则的输入
                    current_content = matched_value;
                }
            } else {
                // 当前正则没有匹配，级联失败
                tracing::debug!("Cascade failed at pattern {} for rule '{}': no match", pattern_index + 1, rule.name);
                break;
            }
        }

        if let Some(value) = extracted_value {
            self.variables.insert(rule.name.clone(), value.clone());
            tracing::debug!("Cascade extraction successful for rule '{}': {}", rule.name, value);
        } else {
            tracing::debug!("Cascade extraction failed for rule '{}'", rule.name);
        }

        Ok(())
    }

    /// 普通模式提取：尝试多个正则表达式，按顺序匹配直到成功
    /// 约定：始终获取第一个捕获组（第一个括号）的内容
    fn extract_with_fallback(&mut self, rule: &ExtractRule, source_content: &str) -> Result<()> {
        let mut extracted = false;
        
        for (pattern_index, pattern) in rule.patterns.iter().enumerate() {
            let regex = Regex::new(pattern)
                .context(format!("Invalid regex pattern {} for rule '{}': {}", pattern_index + 1, rule.name, pattern))?;
            
            if let Some(captures) = regex.captures(source_content) {
                // 约定：始终获取第一个捕获组（第一个括号）的内容
                if let Some(value) = captures.get(1) {
                    self.variables.insert(rule.name.clone(), value.as_str().to_string());
                    extracted = true;
                    tracing::debug!("Fallback extraction successful for rule '{}' with pattern {}: {}", rule.name, pattern_index + 1, value.as_str());
                    break; // 找到匹配就停止尝试其他模式
                } else {
                    // 如果没有捕获组，记录警告
                    tracing::warn!("Pattern {} for rule '{}' has no capture groups: {}", 
                                  pattern_index + 1, rule.name, pattern);
                }
            }
        }
        
        // 可选：记录未匹配的规则（用于调试）
        if !extracted {
            tracing::debug!("No pattern matched for rule '{}' in source '{}'", rule.name, rule.source);
        }

        Ok(())
    }

    /// 获取当前所有变量
    pub fn get_variables(&self) -> &HashMap<String, String> {
        &self.variables
    }

    /// 移除变量
    pub fn remove_variable(&mut self, key: &str) {
        self.variables.remove(key);
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