/// Normalize the expansion key
pub fn strip_with<P>(s: &str, predicate: P) -> &str
where
    P: Fn(char) -> bool
{
    if !s.is_empty() {
        let mut i = 0;
        let mut j = s.len()-1;
        while let Some(c) = s.chars().nth(j) { if !(predicate)(c) { break } else { j -= 1 } }
        while let Some(c) = s.chars().nth(i) { if !(predicate)(c) { break } else { i += 1 } }
        &s[i..=j]
    } else {
        s
    }
}

pub fn escape_all(s: &str, esc: char) -> String {
    s.chars().fold(
        String::with_capacity(s.len()),
            |mut acc, chr| {
                acc.push(esc);
                acc.push(chr);
                acc
            })
}

pub fn check_contents_with<P>(s: &str, predicate: P) -> bool
where
    P: Fn(char) -> bool
{
    for c in s.chars() {
        if !(predicate)(c) { return false }
    }
    true
}
