const ASCENDING_DIGITS: &str = "01234567890123456789";
const DESCENDING_DIGITS: &str = "09876543210987654321";

pub fn is_valid_email(value: &str) -> bool {
    // Deliberately loose — just "looks like an email". Decision 10 scopes
    // real validation hardening to roadmap.md Phase 17.
    match value.split_once('@') {
        Some((local, domain)) => !local.is_empty() && domain.contains('.'),
        None => false,
    }
}

pub fn is_valid_phone(value: &str) -> bool {
    // Must look like a real phone number: an optional leading "+" with a
    // country code, or a leading trunk "0", then only digits/spaces/
    // hyphens/parentheses — "+" may only appear as the very first character.
    let mut chars = value.chars();
    let Some(first) = chars.next() else { return false };
    let rest = chars.as_str();

    let starts_ok = match first {
        '+' => rest.chars().next().is_some_and(|c| c.is_ascii_digit() && c != '0'),
        '0' => true,
        _ => false,
    };
    if !starts_ok {
        return false;
    }
    if !rest.chars().all(|c| c.is_ascii_digit() || matches!(c, '-' | '(' | ')' | ' ')) {
        return false;
    }

    let digits: String = value.chars().filter(char::is_ascii_digit).collect();
    if digits.len() < 7 || digits.len() > 15 {
        return false;
    }

    // Reject obviously fake numbers: all the same digit, or a straight
    // ascending/descending run (e.g. "1234567890", "0123456789").
    if digits.bytes().all(|b| b == digits.as_bytes()[0]) {
        return false;
    }
    if ASCENDING_DIGITS.contains(&digits) || DESCENDING_DIGITS.contains(&digits) {
        return false;
    }

    true
}
