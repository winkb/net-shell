use net_shell::template::TemplateEngine;
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = TemplateEngine::new();
    engine.set_variable("items", json!(["apple", "banana", "cherry"]));

    let template = r#"
        {% for item in items %}
        - {{ item }}
        {% endfor }"#;

    println!("原始模板:");
    for (i, line) in template.lines().enumerate() {
        println!("{}: {:?}", i, line);
    }
    
    // 测试不保留换行符的行为
    engine.set_preserve_loop_newlines(false);
    let result = engine.render_string(template)?;
    println!("处理结果:");
    for (i, line) in result.lines().enumerate() {
        println!("{}: {:?}", i, line);
    }
    
    Ok(())
}