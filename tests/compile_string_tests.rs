use dotr::package::compile_string;
use toml::Table;

#[test]
fn test_compile_string_preserves_forward_slash() {
    let mut context = Table::new();
    context.insert(
        "path".to_string(),
        toml::Value::String("/home/user/config".to_string()),
    );

    let template = "PATH={{ path }}";
    let result = compile_string(template, &context).expect("Failed to compile");

    assert_eq!(result, "PATH=/home/user/config");
    assert!(
        result.contains('/'),
        "Forward slashes should not be escaped"
    );
}

#[test]
fn test_compile_string_preserves_backslash() {
    let mut context = Table::new();
    context.insert(
        "path".to_string(),
        toml::Value::String(r"C:\Users\config".to_string()),
    );

    let template = "PATH={{ path }}";
    let result = compile_string(template, &context).expect("Failed to compile");

    assert_eq!(result, r"PATH=C:\Users\config");
    assert!(result.contains('\\'), "Backslashes should not be escaped");
}

#[test]
fn test_compile_string_preserves_quotes() {
    let mut context = Table::new();
    context.insert(
        "value".to_string(),
        toml::Value::String(r#"hello "world""#.to_string()),
    );

    let template = "VALUE={{ value }}";
    let result = compile_string(template, &context).expect("Failed to compile");

    assert_eq!(result, r#"VALUE=hello "world""#);
    assert!(result.contains('"'), "Quotes should not be escaped");
}

#[test]
fn test_compile_string_preserves_single_quotes() {
    let mut context = Table::new();
    context.insert(
        "value".to_string(),
        toml::Value::String("hello 'world'".to_string()),
    );

    let template = "VALUE={{ value }}";
    let result = compile_string(template, &context).expect("Failed to compile");

    assert_eq!(result, "VALUE=hello 'world'");
    assert!(result.contains('\''), "Single quotes should not be escaped");
}

#[test]
fn test_compile_string_preserves_ampersand() {
    let mut context = Table::new();
    context.insert(
        "value".to_string(),
        toml::Value::String("foo & bar".to_string()),
    );

    let template = "VALUE={{ value }}";
    let result = compile_string(template, &context).expect("Failed to compile");

    assert_eq!(result, "VALUE=foo & bar");
    assert!(result.contains('&'), "Ampersand should not be escaped");
}

#[test]
fn test_compile_string_preserves_less_than() {
    let mut context = Table::new();
    context.insert(
        "value".to_string(),
        toml::Value::String("a < b".to_string()),
    );

    let template = "VALUE={{ value }}";
    let result = compile_string(template, &context).expect("Failed to compile");

    assert_eq!(result, "VALUE=a < b");
    assert!(result.contains('<'), "Less than should not be escaped");
}

#[test]
fn test_compile_string_preserves_greater_than() {
    let mut context = Table::new();
    context.insert(
        "value".to_string(),
        toml::Value::String("a > b".to_string()),
    );

    let template = "VALUE={{ value }}";
    let result = compile_string(template, &context).expect("Failed to compile");

    assert_eq!(result, "VALUE=a > b");
    assert!(result.contains('>'), "Greater than should not be escaped");
}

#[test]
fn test_compile_string_preserves_newlines() {
    let mut context = Table::new();
    context.insert(
        "value".to_string(),
        toml::Value::String("line1\nline2".to_string()),
    );

    let template = "VALUE={{ value }}";
    let result = compile_string(template, &context).expect("Failed to compile");

    assert_eq!(result, "VALUE=line1\nline2");
    assert!(result.contains('\n'), "Newlines should not be escaped");
}

#[test]
fn test_compile_string_preserves_tabs() {
    let mut context = Table::new();
    context.insert(
        "value".to_string(),
        toml::Value::String("col1\tcol2".to_string()),
    );

    let template = "VALUE={{ value }}";
    let result = compile_string(template, &context).expect("Failed to compile");

    assert_eq!(result, "VALUE=col1\tcol2");
    assert!(result.contains('\t'), "Tabs should not be escaped");
}

#[test]
fn test_compile_string_preserves_special_characters() {
    let mut context = Table::new();
    context.insert(
        "value".to_string(),
        toml::Value::String("!@#$%^&*()".to_string()),
    );

    let template = "VALUE={{ value }}";
    let result = compile_string(template, &context).expect("Failed to compile");

    assert_eq!(result, "VALUE=!@#$%^&*()");
}

#[test]
fn test_compile_string_preserves_unicode() {
    let mut context = Table::new();
    context.insert(
        "value".to_string(),
        toml::Value::String("Hello 世界 🌍".to_string()),
    );

    let template = "VALUE={{ value }}";
    let result = compile_string(template, &context).expect("Failed to compile");

    assert_eq!(result, "VALUE=Hello 世界 🌍");
}

#[test]
fn test_compile_string_preserves_file_path_unix() {
    let mut context = Table::new();
    context.insert(
        "home".to_string(),
        toml::Value::String("/home/user".to_string()),
    );
    context.insert(
        "file".to_string(),
        toml::Value::String(".bashrc".to_string()),
    );

    let template = "path={{ home }}/{{ file }}";
    let result = compile_string(template, &context).expect("Failed to compile");

    assert_eq!(result, "path=/home/user/.bashrc");
}

#[test]
fn test_compile_string_preserves_file_path_windows() {
    let mut context = Table::new();
    context.insert(
        "drive".to_string(),
        toml::Value::String(r"C:\Users".to_string()),
    );
    context.insert(
        "file".to_string(),
        toml::Value::String("config.txt".to_string()),
    );

    let template = r"path={{ drive }}\{{ file }}";
    let result = compile_string(template, &context).expect("Failed to compile");

    assert_eq!(result, r"path=C:\Users\config.txt");
}

#[test]
fn test_compile_string_preserves_url() {
    let mut context = Table::new();
    context.insert(
        "url".to_string(),
        toml::Value::String("https://example.com/path?key=value&foo=bar".to_string()),
    );

    let template = "URL={{ url }}";
    let result = compile_string(template, &context).expect("Failed to compile");

    assert_eq!(result, "URL=https://example.com/path?key=value&foo=bar");
    assert!(result.contains("://"));
    assert!(result.contains('?'));
    assert!(result.contains('&'));
    assert!(result.contains('='));
}

#[test]
fn test_compile_string_preserves_json_string() {
    let mut context = Table::new();
    context.insert(
        "json".to_string(),
        toml::Value::String(r#"{"key": "value", "path": "/home/user"}"#.to_string()),
    );

    let template = "JSON={{ json }}";
    let result = compile_string(template, &context).expect("Failed to compile");

    assert_eq!(result, r#"JSON={"key": "value", "path": "/home/user"}"#);
    assert!(result.contains('{'));
    assert!(result.contains('}'));
    assert!(result.contains(':'));
    assert!(result.contains('"'));
}

#[test]
fn test_compile_string_preserves_shell_command() {
    let mut context = Table::new();
    context.insert(
        "cmd".to_string(),
        toml::Value::String(r#"echo "Hello" | grep 'H' && ls -la"#.to_string()),
    );

    let template = "COMMAND={{ cmd }}";
    let result = compile_string(template, &context).expect("Failed to compile");

    assert_eq!(result, r#"COMMAND=echo "Hello" | grep 'H' && ls -la"#);
    assert!(result.contains('|'));
    assert!(result.contains('&'));
    assert!(result.contains('-'));
}

#[test]
fn test_compile_string_preserves_regex_pattern() {
    let mut context = Table::new();
    context.insert(
        "pattern".to_string(),
        toml::Value::String(r"^[\w\-\.]+@[\w\-\.]+$".to_string()),
    );

    let template = "REGEX={{ pattern }}";
    let result = compile_string(template, &context).expect("Failed to compile");

    assert_eq!(result, r"REGEX=^[\w\-\.]+@[\w\-\.]+$");
    assert!(result.contains('['));
    assert!(result.contains(']'));
    assert!(result.contains('\\'));
    assert!(result.contains('^'));
    assert!(result.contains('$'));
}

#[test]
fn test_compile_string_with_nested_variables() {
    let mut context = Table::new();
    let mut git = toml::map::Map::new();
    git.insert(
        "name".to_string(),
        toml::Value::String("John/Doe".to_string()),
    );
    git.insert(
        "email".to_string(),
        toml::Value::String("john@example.com".to_string()),
    );
    context.insert("git".to_string(), toml::Value::Table(git));

    let template = "name={{ git.name }}, email={{ git.email }}";
    let result = compile_string(template, &context).expect("Failed to compile");

    assert_eq!(result, "name=John/Doe, email=john@example.com");
    assert!(
        result.contains('/'),
        "Slashes in nested values should not be escaped"
    );
    assert!(result.contains('@'), "@ symbols should not be escaped");
}

#[test]
fn test_compile_string_preserves_html_entities() {
    let mut context = Table::new();
    context.insert(
        "html".to_string(),
        toml::Value::String("<div>&nbsp;</div>".to_string()),
    );

    let template = "HTML={{ html }}";
    let result = compile_string(template, &context).expect("Failed to compile");

    assert_eq!(result, "HTML=<div>&nbsp;</div>");
    assert!(result.contains('<'));
    assert!(result.contains('>'));
    assert!(result.contains('&'));
}

#[test]
fn test_compile_string_preserves_sql_query() {
    let mut context = Table::new();
    context.insert(
        "query".to_string(),
        toml::Value::String(r#"SELECT * FROM users WHERE name='John' AND age>18"#.to_string()),
    );

    let template = "SQL={{ query }}";
    let result = compile_string(template, &context).expect("Failed to compile");

    assert_eq!(
        result,
        r#"SQL=SELECT * FROM users WHERE name='John' AND age>18"#
    );
    assert!(result.contains('\''));
    assert!(result.contains('>'));
}

#[test]
fn test_compile_string_preserves_markdown() {
    let mut context = Table::new();
    context.insert(
        "md".to_string(),
        toml::Value::String("# Title\n\n[Link](https://example.com)".to_string()),
    );

    let template = "MARKDOWN={{ md }}";
    let result = compile_string(template, &context).expect("Failed to compile");

    assert_eq!(result, "MARKDOWN=# Title\n\n[Link](https://example.com)");
    assert!(result.contains('#'));
    assert!(result.contains('['));
    assert!(result.contains(']'));
    assert!(result.contains("://"));
}

#[test]
fn test_compile_string_multiple_slashes_in_path() {
    let mut context = Table::new();
    context.insert(
        "path".to_string(),
        toml::Value::String("/usr/local/bin/nvim".to_string()),
    );

    let template = "editor={{ path }}";
    let result = compile_string(template, &context).expect("Failed to compile");

    assert_eq!(result, "editor=/usr/local/bin/nvim");
    // Count slashes
    let slash_count = result.matches('/').count();
    assert_eq!(slash_count, 4, "All slashes should be preserved");
}

#[test]
fn test_compile_string_preserves_escape_sequences() {
    let mut context = Table::new();
    context.insert(
        "value".to_string(),
        toml::Value::String(r"Line1\nLine2\tTab".to_string()),
    );

    let template = "VALUE={{ value }}";
    let result = compile_string(template, &context).expect("Failed to compile");

    assert_eq!(result, r"VALUE=Line1\nLine2\tTab");
    // Should preserve the literal \n and \t, not convert them
    assert!(result.contains(r"\n"));
    assert!(result.contains(r"\t"));
}

#[test]
fn test_compile_string_preserves_dollar_sign() {
    let mut context = Table::new();
    context.insert(
        "var".to_string(),
        toml::Value::String("$HOME/config".to_string()),
    );

    let template = "PATH={{ var }}";
    let result = compile_string(template, &context).expect("Failed to compile");

    assert_eq!(result, "PATH=$HOME/config");
    assert!(result.contains('$'), "Dollar signs should not be escaped");
}

#[test]
fn test_compile_string_preserves_percent() {
    let mut context = Table::new();
    context.insert(
        "value".to_string(),
        toml::Value::String("100% complete".to_string()),
    );

    let template = "STATUS={{ value }}";
    let result = compile_string(template, &context).expect("Failed to compile");

    assert_eq!(result, "STATUS=100% complete");
    assert!(result.contains('%'), "Percent signs should not be escaped");
}
