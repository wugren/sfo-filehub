#[path = "../common/mod.rs"]
mod common;

use filehub_server::versions::upload::{MultipartEvent, MultipartParser, UploadLimits};

fn limits(archive: u64) -> UploadLimits {
    UploadLimits {
        max_archive_bytes: archive,
        max_field_bytes: 512,
        max_header_bytes: 8192,
        max_total_bytes: archive + 1024 * 1024,
    }
}

fn multipart_body(boundary: &str, file: &[u8], sha256: Option<&str>) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"file\"; filename=\"a.tar.gz\"\r\nContent-Type: application/gzip\r\n\r\n").as_bytes(),
    );
    body.extend_from_slice(file);
    if let Some(hash) = sha256 {
        body.extend_from_slice(format!("\r\n--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"sha256\"\r\n\r\n").as_bytes(),
        );
        body.extend_from_slice(hash.as_bytes());
    }
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    body
}

fn parse_all(body: &[u8], chunk: usize) -> Result<Vec<u8>, String> {
    let boundary = "boundary-42";
    let mut parser = MultipartParser::new(boundary, limits(1024 * 1024));
    let mut file = Vec::new();
    let mut sha = None;
    for part in body.chunks(chunk.max(1)) {
        let events = match parser.feed(part) {
            Ok(events) => events,
            Err(e) => return Err(format!("feed err at chunk {part:?}: {e}")),
        };
        for event in events {
            match event {
                MultipartEvent::FileChunk(bytes) => file.extend_from_slice(&bytes),
                MultipartEvent::Field { name, value } if name == "sha256" => sha = Some(value),
                MultipartEvent::Field { .. } => {}
            }
        }
    }
    parser.finish().map_err(|e| format!("finish err: {e}"))?;
    if sha != Some("abc123".to_string()) {
        return Err(format!("unexpected sha: {sha:?}"));
    }
    Ok(file)
}

#[test]
fn parser_handles_chunk_splits_across_boundary() {
    let body = multipart_body("boundary-42", b"file-bytes-1234", Some("abc123"));
    let mut failures = Vec::new();
    for chunk in 1..=body.len() + 5 {
        match parse_all(&body, chunk) {
            Ok(got) => assert_eq!(got, b"file-bytes-1234", "chunk={chunk}"),
            Err(e) => failures.push((chunk, e)),
        }
    }
    if !failures.is_empty() {
        panic!(
            "failures ({}): {:?}",
            failures.len(),
            &failures[..failures.len().min(8)]
        );
    }
}

#[test]
fn parser_handles_single_file_part_like_reqwest() {
    let boundary = "abc123";
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("content-disposition: form-data; name=\"file\"; filename=\"a.tar.gz\"\r\ncontent-type: application/gzip\r\n\r\n").as_bytes(),
    );
    body.extend_from_slice(b"gzip-bytes");
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    for chunk in 1..=body.len() + 3 {
        let mut parser = MultipartParser::new(boundary, limits(1024 * 1024));
        let mut file = Vec::new();
        for part in body.chunks(chunk) {
            for event in parser
                .feed(part)
                .unwrap_or_else(|e| panic!("chunk={chunk}: {e}"))
            {
                if let MultipartEvent::FileChunk(bytes) = event {
                    file.extend_from_slice(&bytes);
                }
            }
        }
        parser
            .finish()
            .unwrap_or_else(|e| panic!("chunk={chunk}: {e}"));
        assert_eq!(file, b"gzip-bytes", "chunk={chunk}");
    }
}

#[test]
fn parser_preserves_binary_archive_exactly() {
    let archive = common::make_targz("a.txt", b"hello filehub");
    let body = multipart_body("boundary-bin", &archive, None);
    for chunk in 1..=body.len() + 3 {
        let mut parser = MultipartParser::new("boundary-bin", limits(1024 * 1024));
        let mut file = Vec::new();
        for part in body.chunks(chunk) {
            for event in parser
                .feed(part)
                .unwrap_or_else(|e| panic!("chunk={chunk}: {e}"))
            {
                if let MultipartEvent::FileChunk(bytes) = event {
                    file.extend_from_slice(&bytes);
                }
            }
        }
        parser
            .finish()
            .unwrap_or_else(|e| panic!("chunk={chunk}: {e}"));
        assert_eq!(file, archive, "binary chunk={chunk}");
    }
}

#[test]
fn parser_accepts_small_part_header_in_chunk_with_large_content() {
    // 回归：单个 body chunk 同时包含合法 part 头与超过 8 KiB 的文件内容时，
    // 不得把后续内容计入 part 头上限（037 修复前误报 headers exceed limit）。
    let file = vec![0xAB; 16 * 1024];
    let body = multipart_body("boundary-42", &file, Some("abc123"));

    let mut parser = MultipartParser::new("boundary-42", limits(1024 * 1024));
    let events = parser
        .feed(&body)
        .expect("single large chunk must not exceed part header limit");
    parser.finish().expect("multipart must finish cleanly");

    let mut got = Vec::new();
    let mut sha = None;
    for event in events {
        match event {
            MultipartEvent::FileChunk(bytes) => got.extend_from_slice(&bytes),
            MultipartEvent::Field { name, value } if name == "sha256" => sha = Some(value),
            MultipartEvent::Field { .. } => {}
        }
    }
    assert_eq!(got, file, "file content must round-trip unchanged");
    assert_eq!(sha.as_deref(), Some("abc123"));
}

#[test]
fn parser_handles_large_content_under_arbitrary_chunk_splits() {
    let file = vec![0x42; 16 * 1024];
    let body = multipart_body("boundary-42", &file, Some("abc123"));
    let mut failures = Vec::new();
    for chunk in 1..=body.len() + 5 {
        match parse_all(&body, chunk) {
            Ok(got) => assert_eq!(got, file, "chunk={chunk}"),
            Err(e) => failures.push((chunk, e)),
        }
    }
    if !failures.is_empty() {
        panic!(
            "failures ({}): {:?}",
            failures.len(),
            &failures[..failures.len().min(8)]
        );
    }
}

#[test]
fn parser_rejects_part_headers_longer_than_limit() {
    // 负向保持：part 头本身超过 8 KiB 仍必须被拒绝（037 不放松真实超限）。
    let boundary = "boundary-42";
    let padded = "a".repeat(8200);
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"file\"; filename=\"a.tar.gz\"\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(format!("X-Pad: {padded}\r\n\r\n").as_bytes());
    body.extend_from_slice(b"file-bytes");
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let mut parser = MultipartParser::new(boundary, limits(1024 * 1024));
    let err = parser
        .feed(&body)
        .expect_err("oversized part header must fail");
    assert_eq!(err, "multipart part headers exceed limit");
}

#[test]
fn parser_rejects_missing_file_part() {
    // 036 回归：只有 sha256、没有 file 的 multipart 必须在 finish 阶段被拒绝。
    let boundary = "boundary-nofile";
    let expected_sha = "ab".repeat(32);
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"sha256\"\r\n\r\n").as_bytes(),
    );
    body.extend_from_slice(expected_sha.as_bytes());
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    for chunk in [1usize, 7, body.len()] {
        let mut parser = MultipartParser::new(boundary, limits(1024 * 1024));
        let mut sha = None;
        for part in body.chunks(chunk) {
            for event in parser
                .feed(part)
                .unwrap_or_else(|e| panic!("chunk={chunk}: {e}"))
            {
                if let MultipartEvent::Field { name, value } = event {
                    assert_eq!(name, "sha256");
                    sha = Some(value);
                }
            }
        }
        assert_eq!(sha.as_deref(), Some(expected_sha.as_str()), "chunk={chunk}");
        let err = parser
            .finish()
            .expect_err("missing file part must fail finish");
        assert!(
            err.contains("missing required file part"),
            "chunk={chunk}: unexpected error: {err}"
        );
    }
}

#[test]
fn parser_rejects_empty_file_part() {
    // 036 回归扩展：显式 0 字节 file part 同样必须被拒绝（不支持发布空文件）。
    let boundary = "boundary-emptyfile";
    let expected_sha = "cd".repeat(32);
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"file\"; filename=\"a.tar.gz\"\r\nContent-Type: application/gzip\r\n\r\n").as_bytes(),
    );
    body.extend_from_slice(format!("\r\n--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"sha256\"\r\n\r\n").as_bytes(),
    );
    body.extend_from_slice(expected_sha.as_bytes());
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    for chunk in [1usize, 5, body.len()] {
        let mut parser = MultipartParser::new(boundary, limits(1024 * 1024));
        let mut file = Vec::new();
        for part in body.chunks(chunk) {
            for event in parser
                .feed(part)
                .unwrap_or_else(|e| panic!("chunk={chunk}: {e}"))
            {
                if let MultipartEvent::FileChunk(bytes) = event {
                    file.extend_from_slice(&bytes);
                }
            }
        }
        assert!(file.is_empty(), "empty file part emits no bytes, chunk={chunk}");
        let err = parser
            .finish()
            .expect_err("empty file part must fail finish");
        assert!(
            err.contains("multipart file part is empty"),
            "chunk={chunk}: unexpected error: {err}"
        );
    }
}
