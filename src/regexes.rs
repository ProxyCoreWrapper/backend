use once_cell::sync::Lazy;
use regex::Regex;

pub static CONFIG_ID_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[\w-]+$").expect("config ID regex must be valid"));
