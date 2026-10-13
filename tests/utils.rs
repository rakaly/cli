use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

static DOWNLOADER: Mutex<()> = Mutex::new(());

/// Download a game fixture from a CDN and cache it locally.
pub fn request<S: AsRef<str>>(game: &str, input: S) -> PathBuf {
    let reffed = input.as_ref();
    let cache = Path::new("assets")
        .join("saves")
        .join(game)
        .join(reffed);
    if !cache.exists() {
        let _guard = DOWNLOADER.lock().unwrap();
        if cache.exists() {
            return cache;
        }

        let url = format!("https://cdn-dev.pdx.tools/{game}-saves/{reffed}");
        let mut resp = attohttpc::get(&url).send().unwrap();

        if !resp.is_success() {
            panic!("expected HTTP 200 from CDN");
        }

        std::fs::create_dir_all(cache.parent().unwrap()).unwrap();
        let mut file = std::fs::File::create(&cache).unwrap();
        std::io::copy(&mut resp, &mut file).unwrap();
    }

    cache
}
