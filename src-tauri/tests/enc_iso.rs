use shiori::services::renderer::BookReaderAdapter;
#[test]
fn enc_iso() {
    // 1. what the ADAPTER yields
    let mut a = shiori::services::mobi_adapter::MobiAdapter::new();
    let path = "/home/zura/Personal/coding_cuff/Shiori/broken-files/1752426479_the_briar_club_-_kate_quinn.mobi";
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async { a.load(path).await }).unwrap();
    let ch = a.get_chapter(8).unwrap();
    let i = ch.content.find("lair").unwrap_or(0);
    println!("ADAPTER: ...{}...", &ch.content[i.saturating_sub(20)..i+30]);

    // 2. what the conversion parse yields
    let book = shiori::conversion::formats::mobi::parse(std::path::Path::new(path)).unwrap();
    let ch9 = &book.chapters[8].html;
    let j = ch9.find("lair").unwrap_or(0);
    println!("PARSE:   ...{}...", &ch9[j.saturating_sub(20)..j+30]);
    println!("PARSE has U+FFFD: {}", ch9.contains('\u{FFFD}'));
    println!("PARSE has mojibake 'â': {}", ch9.contains('â'));
}
