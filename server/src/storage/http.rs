//! storage 的下载流辅助（I-005）。

use sfo_http::errors::HttpResult;
use sfo_http::http_server::Response;

use crate::contract::{set_download_headers, ApiError};
use crate::model::FileId;

use super::FileStore;

/// 构造 .tar.gz 下载响应（文件名与版本关联，由 versions/http 传入 project/version）。
pub async fn download_response<Resp: Response>(
    files: &dyn FileStore,
    file_id: &FileId,
    filename: &str,
) -> HttpResult<Resp> {
    match files.open_read(file_id).await {
        Ok(reader) => {
            let mut resp = Resp::new(http::StatusCode::OK);
            if let Err(e) = set_download_headers(&mut resp, filename) {
                return api_error_to_http(&e);
            }
            resp.set_body_read(reader);
            Ok(resp)
        }
        Err(e) => api_error_to_http(&ApiError::from(e)),
    }
}

pub fn api_error_to_http<Resp: Response>(err: &ApiError) -> HttpResult<Resp> {
    super::crate_error_to_http(err)
}

