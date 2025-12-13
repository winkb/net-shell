use net_shell::template::TemplateEngine;
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = TemplateEngine::new();
    engine.set_variable("items", json!(["apple", "banana", "cherry"]));

    // 测试带缩进的模板
    let template = r#"
        {% for item in items %}
        - {{ item }}
        {% endfor %}"#;
    
    println!("带缩进模板: {}", template);
    
    // 测试默认行为
    let result = engine.render_string(template)?;
    println!("默认结果: {:?}", result);
    
    // 测试不保留换行符
    engine.set_preserve_loop_newlines(false);
    let result = engine.render_string(template)?;
    println!("不保留换行符结果: {:?}", result);
    
    Ok(())
}