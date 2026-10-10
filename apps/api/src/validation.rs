const ASCENDING_DIGITS: &str = "01234567890123456789";
const DESCENDING_DIGITS: &str = "09876543210987654321";

pub fn is_valid_email(value: &str) -> bool {
    if value.is_empty() || value.len() > 254 {
        return false;
    }

    let Some((local, domain)) = value.split_once('@') else { return false };
    if local.is_empty() || local.len() > 64 || domain.contains('@') {
        return false;
    }
    if !is_valid_email_local_part(local) || !is_valid_email_domain(domain) {
        return false;
    }

    true
}

fn is_valid_email_local_part(local: &str) -> bool {
    if local.starts_with('.') || local.ends_with('.') || local.contains("..") {
        return false;
    }
    local
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '%' | '+' | '-'))
}

fn is_valid_email_domain(domain: &str) -> bool {
    if domain.starts_with('.') || domain.ends_with('.') || domain.contains("..") {
        return false;
    }
    let labels: Vec<&str> = domain.split('.').collect();
    if labels.len() < 2 {
        return false;
    }
    let Some(tld) = labels.last() else { return false };
    if tld.len() < 2 || !tld.chars().all(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    labels.iter().all(|label| {
        !label.is_empty()
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_emails() {
        assert!(is_valid_email("name@example.com"));
        assert!(is_valid_email("a.b+c@sub.example.co.uk"));
    }

    #[test]
    fn rejects_invalid_emails() {
        assert!(!is_valid_email(""));
        assert!(!is_valid_email("no-at-sign"));
        assert!(!is_valid_email("a@b"));
        assert!(!is_valid_email("a@.com"));
        assert!(!is_valid_email("a@b..com"));
        assert!(!is_valid_email("a@b-.com"));
        assert!(!is_valid_email(&format!("{}@example.com", "a".repeat(300))));
    }
}
