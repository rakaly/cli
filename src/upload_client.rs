use anyhow::{anyhow, bail, Context};
use attohttpc::header::{AUTHORIZATION, CONTENT_ENCODING, CONTENT_TYPE};
use serde::Deserialize;
use std::{
    fs::File,
    io::{BufReader, Cursor, Read, Seek, Write},
    path::Path,
    time::Instant,
};

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct NewSave {
    pub save_id: String,
    pub remaining_save_slots: i32,
    pub used_save_slot: bool,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct RakalyError {
    pub name: String,
    pub msg: String,
}

#[derive(Debug)]
pub struct UploadClient<'a> {
    pub user: &'a str,
    pub api_key: &'a str,
    pub base_url: &'a str,
}

impl<'a> UploadClient<'a> {
    fn format_basic_auth(&self) -> String {
        let auth = format!("{}:{}", self.user, self.api_key);
        format!("Basic {}", base64::encode(auth))
    }

    fn save_url(&self) -> String {
        let result = format!("{}/{}", self.base_url, "api/saves");
        log::debug!("save url: {}", &result);
        result
    }

    fn upload_file_name(&self, path: &Path) -> anyhow::Result<String> {
        let file_name = path
            .file_name()
            .map(|x| x.to_string_lossy())
            .ok_or_else(|| anyhow!("unable to retrieve filename from: {}", path.display()))?;
        log::info!("uploading file: {}", &file_name);
        Ok(file_name.to_string())
    }

    fn upload_zip(&self, path: &Path) -> anyhow::Result<NewSave> {
        let file = File::open(path).context("unable to open")?;
        let meta = file.metadata().context("unable to get metadata")?;

        let reader = BufReader::new(file);
        let now = Instant::now();
        let buffer = recompress(reader, meta.len() as usize)?;
        log::info!(
            "compressed {} bytes to {} in {}ms",
            meta.len(),
            buffer.len(),
            now.elapsed().as_millis()
        );

        let now = Instant::now();
        let resp = attohttpc::post(self.save_url())
            .header(AUTHORIZATION, self.format_basic_auth())
            .header(CONTENT_ENCODING, "br")
            .header(CONTENT_TYPE, "application/x-tar")
            .header("rakaly-filename", self.upload_file_name(path)?)
            .bytes(buffer)
            .send()?;
        log::info!("uploaded in {}ms", now.elapsed().as_millis());

        if resp.is_success() {
            let save_id = resp.json()?;
            Ok(save_id)
        } else {
            let error: RakalyError = resp.json()?;
            bail!("server returned an error: {} : {}", error.name, error.msg)
        }
    }

    fn upload_txt(&self, path: &Path) -> anyhow::Result<NewSave> {
        let file = File::open(path).context("unable to open")?;
        let meta = file.metadata().context("unable to get metadata")?;

        let mut reader = BufReader::new(file);
        let buffer = Vec::with_capacity(meta.len() as usize / 10);
        let mut compressor = new_brotli(Cursor::new(buffer));

        let now = Instant::now();
        std::io::copy(&mut reader, &mut compressor).context("unable to compress")?;
        let buffer = compressor.into_inner().into_inner();
        log::info!(
            "compressed {} bytes to {} in {}ms",
            meta.len(),
            buffer.len(),
            now.elapsed().as_millis()
        );

        let now = Instant::now();
        let resp = attohttpc::post(self.save_url())
            .header(AUTHORIZATION, self.format_basic_auth())
            .header(CONTENT_ENCODING, "br")
            .header(CONTENT_TYPE, "text/plain")
            .header("rakaly-filename", self.upload_file_name(path)?)
            .bytes(buffer.as_slice())
            .send()?;
        log::info!("uploaded in {}ms", now.elapsed().as_millis());

        if resp.is_success() {
            let save_id = resp.json()?;
            Ok(save_id)
        } else {
            let error: RakalyError = resp.json()?;
            bail!("server returned an error: {} : {}", error.name, error.msg)
        }
    }

    pub fn upload(&self, path: &Path) -> anyhow::Result<NewSave> {
        let path_display = path.display();
        let magic = {
            let mut buffer = [0; 4];
            let mut file =
                File::open(path).with_context(|| format!("unable to open: {}", path_display))?;
            file.read_exact(&mut buffer)
                .with_context(|| format!("unable to read: {}", path_display))?;
            buffer
        };

        match magic {
            [0x50, 0x4b, 0x03, 0x04] => self
                .upload_zip(path)
                .with_context(|| format!("unable to upload zip: {}", path_display)),
            [b'E', b'U', b'4', b't'] => self
                .upload_txt(path)
                .with_context(|| format!("unable to upload txt: {}", path_display)),
            x => Err(anyhow!(
                "unexpected file signature: {:?} - {}",
                x,
                path_display
            )),
        }
    }
}

fn new_brotli<W: Write>(writer: W) -> brotli::CompressorWriter<W> {
    brotli::CompressorWriter::new(writer, 4096, 9, 22)
}

pub fn recompress<R>(reader: R, size: usize) -> anyhow::Result<Vec<u8>>
where
    R: Read + Seek,
{
    let mut zip = zip::ZipArchive::new(reader).unwrap();
    let out = Vec::with_capacity(size / 2);

    let compressor = new_brotli(out);
    let mut archive = tar::Builder::new(compressor);

    for index in 0..zip.len() {
        let file = zip.by_index(index)?;
        let mut header = tar::Header::new_gnu();
        header.set_path(file.name())?;
        header.set_size(file.size());
        header.set_mtime(0);
        header.set_cksum();
        archive.append(&header, file)?;
    }

    archive.finish()?;
    let data = archive.into_inner()?.into_inner();
    Ok(data)
}
