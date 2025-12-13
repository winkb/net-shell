use net_shell::template::TemplateEngine;
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = TemplateEngine::new();
    engine.set_variable("items", json!(["apple", "banana", "cherry"]));

    // 测试简单的模板
    let simple_template = "{% for item in items %}- {{ item }}{% endfor %}";
    println!("简单模板: {}", simple_template);
    
    let result = engine.render_string(simple_template)?;
    println!("简单结果: {}", result);
    
    // 测试带换行的模板
    let template = "
{% for item in items %}
- {{ item }}
{% endfor %}";
    println!("带换行模板: {}", template);
    
    let result = engine.render_string(template)?;
    println!("带换行结果: {}", result);
    
    Ok(())
}