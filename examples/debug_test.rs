use net_shell::template::TemplateEngine;
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = TemplateEngine::new();
    engine.set_variable("items", json!(["apple", "banana", "cherry"]));

    let template = r#"
        {% for item in items %}
        - {{ item }}
        {% endfor %}"#;

    // 测试默认保留换行符的行为
    let result = engine.render_string(template)?;
    println!("默认行为结果:");
    println!("{:?}", result);
    println!("实际输出:");
    println!("{}", result);
    
    // 测试不保留换行符的行为
    let result = engine.set_preserve_loop_newlines(false).render_string(template)?;
    println!("不保留换行符结果:");
    println!("{:?}", result);
    println!("实际输出:");
    println!("{}", result);
    
    Ok(())
}