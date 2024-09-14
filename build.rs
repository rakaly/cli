fn main() {
    std::fs::create_dir_all("assets/tokens").expect("to create tokens directory");
    for game in ["ck3", "hoi4", "eu4", "imperator", "vic3"] {
        let fp = format!("assets/tokens/{game}.txt");
        let p = std::path::Path::new(&fp);
        if !p.exists() {
            std::fs::write(p, "").expect("to write file");
        }
    }
}
