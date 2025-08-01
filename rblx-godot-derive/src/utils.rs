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

pub fn snake_case_to_camel_case(name: &str) -> String {
    let mut result = String::new();
    let mut chars = name.chars().peekable();
    if let Some(first) = chars.next() {
        result.push(first.to_ascii_uppercase());
        loop {
            let c = match chars.next() {
                Some(c) => c,
                None => break,
            };
            if c == '_' {
                if let Some(next) = chars.peek() {
                    if *next == '_' {
                        continue;
                    }
                    result.push(unsafe { chars.next().unwrap_unchecked().to_ascii_uppercase() });
                }
            } else {
                result.push(c);
            }
        }
    }
    result
}
