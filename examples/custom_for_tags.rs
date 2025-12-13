use net_shell::template::TemplateEngine;
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 测试自定义For循环标签 ===");
    
    // 测试默认的{%和%}标签
    let mut engine = TemplateEngine::new();
    engine.set_variable("items", json!(["apple", "banana", "cherry"]));
    
    let template = r#"
{% for item in items %}
- {{ item }}
{% endfor %}"#;
    
    let result = engine.render_string(template)?;
    println!("默认标签测试:{}", result);
    
    // 测试自定义的<%和%>标签
    let mut custom_engine = TemplateEngine::with_all_delimiters("{{", "}}", "<%", "%>");
    custom_engine.set_variable("fruits", json!(["orange", "grape", "lemon"]));
    
    let template = r#"
<% for fruit in fruits %>
* {{ fruit }}
<% endfor %>"#;
    
    let result = custom_engine.render_string(template)?;
    println!("自定义标签测试:{}", result);
    
    // 测试自定义的[[和]]标签
    let mut custom_engine2 = TemplateEngine::with_all_delimiters("{{", "}}", "[[", "]]");
    custom_engine2.set_variable("numbers", json!(["one", "two", "three"]));
    
    let template = r#"
[[ for num in numbers ]]
+ {{ num }}
[[ endfor ]]"#;
    
    let result = custom_engine2.render_string(template)?;
    println!("自定义标签测试2:{}", result);
    
    println!("=== 测试完成 ===");
    Ok(())
}