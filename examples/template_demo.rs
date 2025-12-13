use net_shell::template::TemplateEngine;
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 测试模板引擎完整功能 ===");
    
    // 测试基本变量替换
    let mut engine = TemplateEngine::new();
    engine.set_variable("name", "World");
    engine.set_variable("age", 25);
    
    let result = engine.render_string("Hello, {{ name }}! You are {{ age }} years old.")?;
    println!("变量替换测试: {}", result);
    
    // 测试嵌套变量访问
    engine.set_variable("user", json!({
        "name": "Alice",
        "profile": {
            "age": 30,
            "city": "Beijing"
        }
    }));
    
    let result = engine.render_string("Name: {{ user.name }}, City: {{ user.profile.city }}")?;
    println!("嵌套变量测试: {}", result);
    
    // 测试for循环
    engine.set_variable("items", json!(["apple", "banana", "cherry"]));
    
    let template = r#"
{% for item in items %}
- {{ item }}
{% endfor %}"#;
    
    let result = engine.render_string(template)?;
    println!("For循环测试:{}", result);
    
    // 测试自定义定界符
    let mut custom_engine = TemplateEngine::with_delimiters("${", "}");
    custom_engine.set_variable("name", "Custom");
    
    let result = custom_engine.render_string("Hello, ${ name }!")?;
    println!("自定义定界符测试: {}", result);
    
    // 测试复杂模板
    engine.set_variable("title", "User List");
    engine.set_variable("users", json!([
        {"name": "Alice", "age": 25},
        {"name": "Bob", "age": 30},
        {"name": "Charlie", "age": 35}
    ]));
    
    let template = r#"
<h1>{{ title }}</h1>
<ul>
{% for user in users %}
    <li>{{ user.name }} ({{ user.age }} years old)</li>
{% endfor %}
</ul>"#;
    
    let result = engine.render_string(template)?;
    println!("复杂模板测试:{}", result);
    
    println!("=== 所有测试完成 ===");
    Ok(())
}