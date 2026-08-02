fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let path = std::path::Path::new(
        "/home/zura/Personal/coding_cuff/Shiori/broken-files/samples/book.pdf",
    );
    match shiori::conversion::formats::pdf::parse(path) {
        Ok(book) => {
            println!("TITLE: {:?}", book.title);
            for (i, ch) in book.chapters.iter().enumerate() {
                println!("CH{}: {:?} ({} bytes)", i, ch.title, ch.html.len());
            }
        }
        Err(e) => println!("ERROR: {:?}", e),
    }
}
