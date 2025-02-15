use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

static DOWNLOADER: Mutex<()> = Mutex::new(());

/// Request data from s3 and cache it locally
pub fn request<S: AsRef<str>>(bucket_name: &str, input: S) -> PathBuf {
    let reffed = input.as_ref();
    let cache = Path::new("assets").join("saves").join(reffed);
    if !cache.exists() {
        let _guard = DOWNLOADER.lock().unwrap();
        if cache.exists() {
            return cache;
        }

        let url = format!(
            "https://{}.s3.us-west-002.backblazeb2.com/{}",
            bucket_name, reffed
        );
        let resp = attohttpc::get(&url).send().unwrap();

        if !resp.is_success() {
            panic!("expected a 200 code from s3");
        } else {
            let data = resp.bytes().unwrap();
            std::fs::create_dir_all(cache.parent().unwrap()).unwrap();
            std::fs::write(&cache, &data).unwrap();
        }
    }

    cache
}
