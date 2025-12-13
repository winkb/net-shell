# 模板引擎使用指南

这个模板引擎支持以下功能：

## 1. 基本变量替换

```rust
use net_shell::template::TemplateEngine;

let mut engine = TemplateEngine::new();
engine.set_variable("name", "World");

let result = engine.render_string("Hello, {{ name }}!")?;
// 结果: "Hello, World!"
```

## 2. 嵌套变量访问

```rust
use serde_json::json;
use net_shell::template::TemplateEngine;

let mut engine = TemplateEngine::new();
engine.set_variable("user", json!({
    "name": "Alice",
    "profile": {
        "age": 30,
        "city": "Beijing"
    }
}));

let result = engine.render_string("Name: {{ user.name }}, City: {{ user.profile.city }}")?;
// 结果: "Name: Alice, City: Beijing"
```

## 3. For循环

```rust
use serde_json::json;
use net_shell::template::TemplateEngine;

let mut engine = TemplateEngine::new();
engine.set_variable("items", json!(["apple", "banana", "cherry"]));

let template = r#"
{% for item in items %}
- {{ item }}
{% endfor %}"#;

let result = engine.render_string(template)?;
// 结果: 
// - apple
// - banana
// - cherry
```

## 4. 自定义定界符

```rust
use net_shell::template::TemplateEngine;

let mut engine = TemplateEngine::with_delimiters("${", "}");
engine.set_variable("name", "Custom");

let result = engine.render_string("Hello, ${ name }!")?;
// 结果: "Hello, Custom!"
```

## 5. 复杂模板

```rust
use serde_json::json;
use net_shell::template::TemplateEngine;

let mut engine = TemplateEngine::new();
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
```

## 6. 从文件渲染模板

```rust
use net_shell::template::TemplateEngine;

let mut engine = TemplateEngine::new();
engine.set_template_dir("./templates")?;
engine.set_variable("name", "World");

let result = engine.render_file("template.html")?;
```

## 语法说明

- 变量替换: `{{ variable_name }}` 或 `{{ object.property }}`
- For循环: `{% for item in items %} ... {% endfor %}`
- Include指令: `{% include "template.html" %}`

## 注意事项

- For循环使用 `{%` 和 `%}` 作为定界符
- 变量替换使用 `{{` 和 `}}` 作为定界符
- 支持嵌套对象访问，使用点号分隔
- 所有变量都存储为 `serde_json::Value` 类型，支持字符串、数字、数组和对象