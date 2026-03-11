fn main() {
    if let Err(error) = c_go::cli::run() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}
