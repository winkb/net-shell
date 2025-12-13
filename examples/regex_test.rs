use regex::Regex;

fn main() {
    let left = "{%";
    let right = "%}";
    let left_escaped = regex::escape(left);
    let right_escaped = regex::escape(right);
    
    // for循环匹配正则：{% for item in items %}   ... {% endfor %}
    let for_pattern = format!(r"(?s){}\s*for\s+(\w+)\s+in\s+(\w+)\s*{}(.*?){}\s*endfor\s*{}", left_escaped, right_escaped, left_escaped, right_escaped);
    println!("For pattern: {}", for_pattern);
    
    let for_regex = Regex::new(&for_pattern).unwrap();
    
    let template = r#"
{% for item in items %}
- {{ item }}
{% endfor %}"#;
    
    println!("Template: {}", template);
    
    if let Some(captures) = for_regex.captures(template) {
        println!("Match found!");
        for (i, cap) in captures.iter().enumerate() {
            if let Some(group) = cap {
                println!("Group {}: '{}'", i, group.as_str());
            }
        }
    } else {
        println!("No match found!");
    }
}