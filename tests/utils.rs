use s3::{creds::Credentials, Bucket, Region};
use std::path::{Path, PathBuf};

/// Request data from s3 and cache it locally
pub fn request<S: AsRef<str>>(bucket_name: &str, input: S) -> PathBuf {
    let reffed = input.as_ref();
    let cache = Path::new("assets").join("saves").join(reffed);
    if cache.exists() {
        cache
    } else {
        let region_name = "us-west-002".to_string();
        let endpoint = "s3.us-west-002.backblazeb2.com".to_string();
        let region = Region::Custom {
            region: region_name,
            endpoint,
        };
        let credentials = Credentials::anonymous().unwrap();
        let bucket = Bucket::new(bucket_name, region, credentials).unwrap();
        let (data, code) = bucket.get_object_blocking(reffed).unwrap();

        if code != 200 {
            panic!("expected a 200 code from s3");
        } else {
            std::fs::create_dir_all(cache.parent().unwrap()).unwrap();
            std::fs::write(&cache, &data).unwrap();
            cache
        }
    }
}
