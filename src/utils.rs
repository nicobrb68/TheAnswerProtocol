pub fn fatal(msg: &str) -> ! {
    eprintln!("ERROR: {}", msg);
    std::process::exit(1);
}

pub fn is_valid_username(name: &str) -> bool {
    !name.is_empty() && name.len() <= 20 && name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

pub fn is_valid_group_name(group_name: &str) -> bool {
    !group_name.is_empty() && group_name.len() <= 25 && group_name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')

}

pub fn get_args(line: &str) -> &str {
    match line.find(' ') {
        Some(pos) => line[pos + 1..].trim(),
        None => "",
    }
}