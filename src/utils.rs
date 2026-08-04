pub fn fatal(msg: &str) -> ! {
    eprintln!("ERROR: {}", msg);
    std::process::exit(1);
}
