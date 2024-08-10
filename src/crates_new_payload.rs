use poem::{error::BadRequest, FromRequest, Request, RequestBody, Result};
use tokio::io::AsyncReadExt;

pub struct CratesPayload {
    pub meta_len: u32,
    pub meta_buf: Vec<u8>,
    pub crate_len: u32,
    pub crate_buf: Vec<u8>,
}

// Implements a token extractor
impl<'a> FromRequest<'a> for CratesPayload {
    async fn from_request(req: &'a Request, body: &mut RequestBody) -> Result<Self> {
        log::info!("[CratesPayload] headers {:?}", req.headers());
        let body = body.take()?;
        let mut reader = body.into_async_read();

        let meta_len = reader.read_u32_le().await.map_err(|e| BadRequest(e))?;
        log::info!("[CratesPayload] meta_len {meta_len}");
        let mut meta_buf = Vec::with_capacity(meta_len as usize);
        unsafe {
            meta_buf.set_len(meta_len as usize);
        }
        reader
            .read_exact(&mut meta_buf)
            .await
            .map_err(|e| BadRequest(e))?;

        let crate_len = reader.read_u32_le().await.map_err(|e| BadRequest(e))?;
        log::info!("[CratesPayload] crate_len {crate_len}");
        let mut crate_buf = Vec::with_capacity(crate_len as usize);
        unsafe {
            crate_buf.set_len(crate_len as usize);
        }
        reader
            .read_exact(&mut crate_buf)
            .await
            .map_err(|e| BadRequest(e))?;

        log::info!(
            "[CratesPayload] meta {meta_len} {}",
            String::from_utf8_lossy(&meta_buf)
        );
        Ok(CratesPayload {
            meta_len,
            meta_buf,
            crate_len,
            crate_buf,
        })
    }
}
