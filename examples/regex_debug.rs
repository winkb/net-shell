use regex::Regex;

fn main() {
    let template = r#"
        {% for item in items %}
        - {{ item }}
        {% endfor %}"#;  // 在这里添加换行符
    
    let for_left_escaped = regex::escape("{%");
    let for_right_escaped = regex::escape("%}");
    
    // for循环匹配正则：{% for item in items %}   ... {% endfor %}
    let for_pattern = format!(r"(?s){}\s*for\s+(\w+)\s+in\s+(\w+)\s*{}(.*?){}\s*endfor\s*{}", for_left_escaped, for_right_escaped, for_left_escaped, for_right_escaped);
    println!("For pattern: {}", for_pattern);
    
    let for_regex = Regex::new(&for_pattern).unwrap();
    
    println!("Template: {}", template);
    println!("Template repr: {:?}", template);
    
    if let Some(captures) = for_regex.captures(template) {
        println!("Match found!");
        for (i, cap) in captures.iter().enumerate() {
            if let Some(group) = cap {
                println!("Group {}: '{}'", i, group.as_str());
                if i == 3 { // 循环内容
                    println!("Loop content repr: {:?}", group.as_str());
                }
            }
        }
    } else {
        println!("No match found!");
    }
}