pub fn camel_case_to_snake_case(name: &str) -> String {
    let mut result = String::new();
    let mut chars = name.chars();
    if let Some(first) = chars.next() {
        result.push(first.to_ascii_lowercase());
        for c in chars {
            if c.is_uppercase() {
                result.push('_');
                result.push(c.to_ascii_lowercase());
            } else {
                result.push(c);
            }
        }
    }
    result
}
