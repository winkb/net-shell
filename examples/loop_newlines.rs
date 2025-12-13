use net_shell::template::TemplateEngine;
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 测试循环换行符控制 ===");
    
    // 测试默认保留换行符
    let mut engine = TemplateEngine::new();
    engine.set_variable("items", json!(["apple", "banana", "cherry"]));
    
    let template = r#"
{% for item in items %}
- {{ item }}
{% endfor %}"#;
    
    let result = engine.render_string(template)?;
    println!("保留换行符的结果:{}", result);
    
    // 测试去除换行符
    engine.set_preserve_loop_newlines(false);
    let result = engine.render_string(template)?;
    println!("去除换行符的结果:{}", result);
    
    // 测试更复杂的模板
    let template2 = r#"
<ul>
{% for item in items %}
    <li>{{ item }}</li>
{% endfor %}
</ul>"#;
    
    let mut engine2 = TemplateEngine::new();
    engine2.set_variable("items", json!(["red", "green", "blue"]));
    
    let result = engine2.render_string(template2)?;
    println!("保留换行符的复杂模板:{}", result);
    
    engine2.set_preserve_loop_newlines(false);
    let result = engine2.render_string(template2)?;
    println!("去除换行符的复杂模板:{}", result);
    
    println!("=== 测试完成 ===");
    Ok(())
}