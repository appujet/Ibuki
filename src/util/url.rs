use std::sync::LazyLock;

use regex::Regex;

static URL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"https?://(www\.)?[-a-zA-Z0-9@:%._\+~#=]{2,256}(\.[a-z]{2,4})?\b([-a-zA-Z0-9@:%_\+.~#?&//=]*)").unwrap()
});

pub fn is_url(url: &str) -> bool {
    URL_REGEX.is_match(url)
}
