use anyhow::{anyhow, Result};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// 模板引擎结构体
pub struct TemplateEngine {
    /// 变量映射
    variables: HashMap<String, serde_json::Value>,
    /// 模板目录路径
    template_dir: Option<String>,
    /// 左定界符
    left_delimiter: String,
    /// 右定界符
    right_delimiter: String,
    /// for循环左定界符
    for_left_delimiter: String,
    /// for循环右定界符
    for_right_delimiter: String,
    /// 是否保留循环中的换行符
    preserve_loop_newlines: bool,
    /// 变量正则表达式
    var_regex: Regex,
    /// for循环正则表达式
    for_regex: Regex,
    /// include正则表达式
    include_regex: Regex,
}

impl TemplateEngine {
    /// 创建新的模板引擎实例
    pub fn new() -> Self {
        Self::with_delimiters("{{", "}}")
    }

    /// 使用自定义定界符创建模板引擎实例
    pub fn with_delimiters(left: &str, right: &str) -> Self {
        Self::with_all_delimiters(left, right, "{%", "%}")
    }

    /// 使用自定义定界符创建模板引擎实例，包括for循环定界符
    pub fn with_all_delimiters(
        var_left: &str,
        var_right: &str,
        for_left: &str,
        for_right: &str,
    ) -> Self {
        let var_left_escaped = regex::escape(var_left);
        let var_right_escaped = regex::escape(var_right);
        let for_left_escaped = regex::escape(for_left);
        let for_right_escaped = regex::escape(for_right);

        // 变量匹配正则：{{ variable }}
        let var_pattern = format!(
            r"{}\s*([a-zA-Z_][a-zA-Z0-9_]*(?:\.[a-zA-Z_][a-zA-Z0-9_]*)*)\s*{}",
            var_left_escaped, var_right_escaped
        );
        let var_regex = Regex::new(&var_pattern).unwrap();

        // for循环匹配正则：{% for item in items %}   ... {% endfor %}
        // 支持split语法：{% for item in items split "," %}   ... {% endfor %}
        let for_pattern = format!(
            "(?s){}\\s*for\\s+(\\w+)\\s+in\\s+(\\w+)(?:\\s+split\\s+\"([^\"]+)\")?\\s*{}(.*?){}\\s*endfor\\s*{}",
            for_left_escaped, for_right_escaped, for_left_escaped, for_right_escaped
        );
        let for_regex = Regex::new(&for_pattern).unwrap();

        // include匹配正则：{% include "template.html" %}
        let include_pattern = format!(
            "{}\\s*include\\s+\"([^\"]+)\"\\s*{}",
            for_left_escaped, for_right_escaped
        );
        let include_regex = Regex::new(&include_pattern).unwrap();

        Self {
            variables: HashMap::new(),
            template_dir: None,
            left_delimiter: var_left.to_string(),
            right_delimiter: var_right.to_string(),
            for_left_delimiter: for_left.to_string(),
            for_right_delimiter: for_right.to_string(),
            preserve_loop_newlines: true, // 默认保留换行符，保持向后兼容
            var_regex,
            for_regex,
            include_regex,
        }
    }

    /// 设置模板目录
    pub fn set_template_dir<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.template_dir = Some(path.as_ref().to_string_lossy().to_string());
        self
    }

    /// 设置变量
    pub fn set_variable<K: Into<String>, V: Into<serde_json::Value>>(
        &mut self,
        key: K,
        value: V,
    ) -> &mut Self {
        self.variables.insert(key.into(), value.into());
        self
    }

    /// 批量设置变量
    pub fn set_variables(&mut self, vars: HashMap<String, serde_json::Value>) -> &mut Self {
        for (key, value) in vars {
            self.variables.insert(key, value);
        }
        self
    }

    /// 设置是否保留循环中的换行符
    pub fn set_preserve_loop_newlines(&mut self, preserve: bool) -> &mut Self {
        self.preserve_loop_newlines = preserve;
        self
    }

    /// 渲染模板字符串
    pub fn render_string(&self, template: &str) -> Result<String> {
        let mut result = template.to_string();

        // 1. 处理include指令
        result = self.process_includes(&result)?;

        // 2. 处理for循环
        result = self.process_for_loops(&result)?;

        // 3. 处理变量替换
        result = self.process_variables(&result)?;

        Ok(result)
    }

    /// 渲染模板文件
    pub fn render_file<P: AsRef<Path>>(&self, template_path: P) -> Result<String> {
        let template_content = fs::read_to_string(template_path)?;
        self.render_string(&template_content)
    }

    /// 处理include指令
    fn process_includes(&self, template: &str) -> Result<String> {
        let mut result = template.to_string();

        while let Some(captures) = self.include_regex.captures(&result) {
            let full_match = captures.get(0).unwrap().as_str();
            let template_name = captures.get(1).unwrap().as_str();

            let included_content = if let Some(ref dir) = self.template_dir {
                let full_path = Path::new(dir).join(template_name);
                fs::read_to_string(full_path)
                    .map_err(|e| anyhow!("Failed to include template '{}': {}", template_name, e))?
            } else {
                return Err(anyhow!(
                    "Template directory not set for include: {}",
                    template_name
                ));
            };

            result = result.replace(full_match, &included_content);
        }

        Ok(result)
    }

    /// 处理for循环
    fn process_for_loops(&self, template: &str) -> Result<String> {
        let mut result = template.to_string();

        while let Some(captures) = self.for_regex.captures(&result) {
            let full_match = captures.get(0).unwrap().as_str();
            let item_name = captures.get(1).unwrap().as_str();
            let array_name = captures.get(2).unwrap().as_str();
            let split_delimiter = captures.get(3).map(|m| m.as_str());
            let loop_content = captures.get(4).unwrap().as_str();

            let array_value = self
                .variables
                .get(array_name)
                .ok_or_else(|| anyhow!("Array '{}' not found in variables", array_name))?;

            // 根据是否有split参数处理不同的数据类型
            let items: Vec<serde_json::Value> = if let Some(delimiter) = split_delimiter {
                // 处理split操作
                match array_value {
                    serde_json::Value::String(s) => {
                        s.split(delimiter)
                            .map(|part| serde_json::Value::String(part.to_string()))
                            .collect()
                    }
                    _ => {
                        return Err(anyhow!(
                            "Cannot split non-string variable '{}'",
                            array_name
                        ))
                    }
                }
            } else {
                // 处理数组
                if let serde_json::Value::Array(items) = array_value {
                    items.clone()
                } else {
                    return Err(anyhow!("'{}' is not an array", array_name));
                }
            };

            let mut loop_result = String::new();

            for item in items {
                let mut temp_vars = self.variables.clone();
                temp_vars.insert(item_name.to_string(), item.clone());

                let temp_engine = Self {
                    variables: temp_vars,
                    template_dir: self.template_dir.clone(),
                    left_delimiter: self.left_delimiter.clone(),
                    right_delimiter: self.right_delimiter.clone(),
                    for_left_delimiter: self.for_left_delimiter.clone(),
                    for_right_delimiter: self.for_right_delimiter.clone(),
                    preserve_loop_newlines: self.preserve_loop_newlines,
                    var_regex: self.var_regex.clone(),
                    for_regex: self.for_regex.clone(),
                    include_regex: self.include_regex.clone(),
                };

                let mut rendered = temp_engine.process_variables(loop_content)?;

                // 如果不保留换行符，则去除循环产生的空行，但保留内容内的换行符和缩进
                if !self.preserve_loop_newlines {
                    // 按行分割，过滤掉只包含空白字符的行
                    let lines: Vec<&str> = rendered
                        .lines()
                        .filter(|line| !line.trim().is_empty())
                        .collect();

                    // 重新组合，保留原有的缩进和格式
                    if !lines.is_empty() {
                        rendered = lines.join("\n");

                        // 如果不是第一个循环项，在前面添加换行符
                        if !loop_result.is_empty() {
                            loop_result.push_str("\n");
                        }
                    } else {
                        rendered = String::new();
                    }
                }

                loop_result.push_str(&rendered);
            }

            result = result.replace(full_match, &loop_result);
        }

        Ok(result)
    }

    /// 处理变量替换
    fn process_variables(&self, template: &str) -> Result<String> {
        let mut result = template.to_string();

        while let Some(captures) = self.var_regex.captures(&result) {
            let full_match = captures.get(0).unwrap().as_str();
            let variable_path = captures.get(1).unwrap().as_str();

            let value = self.get_variable_value(variable_path)?;
            let value_str = match value {
                serde_json::Value::String(s) => s.clone(),
                v => v.to_string(),
            };

            result = result.replace(full_match, &value_str);
        }

        Ok(result)
    }

    /// 获取变量值，支持点号路径访问嵌套对象
    fn get_variable_value(&self, path: &str) -> Result<serde_json::Value> {
        let parts: Vec<&str> = path.split('.').collect();

        if parts.is_empty() {
            return Err(anyhow!("Empty variable path"));
        }

        let current = self
            .variables
            .get(parts[0])
            .ok_or_else(|| anyhow!("Variable '{}' not found", parts[0]))?;

        if parts.len() == 1 {
            return Ok(current.clone());
        }

        let mut result = current;
        for part in &parts[1..] {
            match result {
                serde_json::Value::Object(map) => {
                    result = map
                        .get(*part)
                        .ok_or_else(|| anyhow!("Property '{}' not found in variable", part))?;
                }
                _ => {
                    return Err(anyhow!(
                        "Cannot access property '{}' on non-object value",
                        part
                    ))
                }
            }
        }

        Ok(result.clone())
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tracing::instrument::WithSubscriber;

    #[test]
    fn test_variable_replacement() {
        let mut engine = TemplateEngine::new();
        engine.set_variable("name", "World");
        engine.set_variable("age", 25);

        let result = engine
            .render_string("Hello, {{ name }}! You are {{ age }} years old.")
            .unwrap();
        assert_eq!(result, "Hello, World! You are 25 years old.");
    }

    #[test]
    fn test_nested_variable_access() {
        let mut engine = TemplateEngine::new();
        engine.set_variable(
            "user",
            json!({
                "name": "Alice",
                "profile": {
                    "age": 30,
                    "city": "Beijing"
                }
            }),
        );

        let result = engine
            .render_string("Name: {{ user.name }}, City: {{ user.profile.city }}")
            .unwrap();
        assert_eq!(result, "Name: Alice, City: Beijing");
    }

    #[test]
    fn test_for_loop() {
        let mut engine = TemplateEngine::with_all_delimiters("{{", "}}", "#{%", "%}");
        engine.set_variable("items", json!(["apple", "banana", "cherry"]));

        let template = r#"
#{% for item in items %}
        - {{ item }}
#{% endfor %}"#;

        let result = engine
            .set_preserve_loop_newlines(false)
            .render_string(template)
            .unwrap();
        let expected = r#"
        - apple
        - banana
        - cherry"#;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_custom_delimiters() {
        let mut engine = TemplateEngine::with_delimiters("${", "}");
        engine.set_variable("name", "Custom");

        let result = engine.render_string("Hello, ${ name }!").unwrap();
        assert_eq!(result, "Hello, Custom!");
    }

    #[test]
    fn test_complex_template() {
        let mut engine = TemplateEngine::new();
        engine.set_variable("title", "User List");
        engine.set_variable(
            "users",
            json!( [
            {"name": "Alice", "age": 25},
            {"name": "Bob", "age": 30},
            {"name": "Charlie", "age": 35}
        ] ),
        );

        let template = r#"
        <h1>{{ title }}</h1>
        <ul>
        {% for user in users %}
            <li>{{ user.name }} ({{ user.age }} years old)</li>
        {% endfor %}
        </ul>"#;

        let result = engine.render_string(template).unwrap();

        assert!(result.contains("<h1>User List</h1>"));
        assert!(result.contains("<li>Alice (25 years old)</li>"));
        assert!(result.contains("<li>Bob (30 years old)</li>"));
        assert!(result.contains("<li>Charlie (35 years old)</li>"));
    }

    #[test]
    fn test_custom_for_tags() {
        let mut engine = TemplateEngine::with_all_delimiters("{{", "}}", "<%", "%>");
        engine.set_variable("items", json!(["red", "green", "blue"]));

        let template = r#"
<% for color in items %>
* {{ color }}
<% endfor %>"#;

        let result = engine.render_string(template).unwrap();

        assert!(result.contains("* red"));
        assert!(result.contains("* green"));
        assert!(result.contains("* blue"));
    }

    #[test]
    fn test_preserve_loop_newlines() {
        // 测试默认保留换行符
        let mut engine = TemplateEngine::new();
        engine.set_variable("items", json!(["a", "b", "c"]));

        let template = r#"
{% for item in items %}
- {{ item }}
{% endfor %}"#;

        let result = engine.render_string(template).unwrap();

        // 默认应该保留换行符
        assert!(result.contains("\n- a\n"));
        assert!(result.contains("\n- b\n"));
        assert!(result.contains("\n- c\n"));

        // 测试不保留换行符
        engine.set_preserve_loop_newlines(false);
        let result = engine.render_string(template).unwrap();

        // 不应该有多余的空行
        assert!(result.contains("- a\n- b\n- c"));
    }

    #[test]
    fn test_split_functionality() {
        let mut engine = TemplateEngine::new();
        engine.set_variable("csv_string", "apple,banana,cherry");

        let template = r#"
{% for fruit in csv_string split "," %}
- {{ fruit }}
{% endfor %}"#;

        let result = engine
            .set_preserve_loop_newlines(false)
            .render_string(template)
            .unwrap();
        
        let expected = r#"
- apple
- banana
- cherry"#;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_split_with_space_delimiter() {
        let mut engine = TemplateEngine::new();
        engine.set_variable("space_separated", "red green blue");

        let template = r#"
{% for color in space_separated split " " %}
* {{ color }}
{% endfor %}"#;

        let result = engine
            .set_preserve_loop_newlines(false)
            .render_string(template)
            .unwrap();
        
        assert!(result.contains("* red"));
        assert!(result.contains("* green"));
        assert!(result.contains("* blue"));
    }

    #[test]
    fn test_split_with_complex_delimiter() {
        let mut engine = TemplateEngine::new();
        engine.set_variable("complex_string", "item1||item2||item3");

        let template = r#"
{% for item in complex_string split "||" %}
{{ item }}
{% endfor %}"#;

        let result = engine
            .set_preserve_loop_newlines(false)
            .render_string(template)
            .unwrap();
        
        assert!(result.contains("item1"));
        assert!(result.contains("item2"));
        assert!(result.contains("item3"));
    }
}
