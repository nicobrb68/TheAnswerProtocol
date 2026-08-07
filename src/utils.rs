pub fn fatal(msg: &str) -> ! {
    eprintln!("ERROR: {}", msg);
    std::process::exit(1);
}

pub fn is_valid_username(name: &str) -> bool {
    !name.is_empty() && name.len() <= 20 && name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}