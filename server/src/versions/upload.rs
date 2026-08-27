//! 增量 multipart/form-data 解析器：流式消费 HTTP body chunk，文件内容按
//! 小块持有式直出，不整包复制进内存，也不泄漏长生命周期内存（026 任务）。

/// 上传路径的三类上限。
#[derive(Debug, Clone, Copy)]
pub struct UploadLimits {
    /// `file` part 累计字节上限（与 `FilesConfig.max_archive_bytes` 一致）。
    pub max_archive_bytes: u64,
    /// 非 file 字段（如 sha256）单字段内容上限。
    pub max_field_bytes: usize,
    /// 单个 part 头部上限。
    pub max_header_bytes: usize,
    /// 总请求体上限（归档 + multipart 开销预算）。
    pub max_total_bytes: u64,
}

/// multipart 解析产出的流式事件。`FileChunk` 为持有式小块（≤输入 chunk 大小）。
#[derive(Debug)]
pub enum MultipartEvent {
    FileChunk(Vec<u8>),
    Field { name: String, value: String },
}

enum ParseStep {
    Event(MultipartEvent),
    Progress,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Preamble,
    Headers,
    Content,
    Boundary,
    Closing,
    Finished,
}

/// 状态机：等待首边界 -> part 头 -> 内容 -> 边界；`feed` 可接受任意切分。
pub struct MultipartParser<'b> {
    boundary: &'b [u8],
    limits: UploadLimits,
    phase: Phase,
    /// 未消费输入（含可能跨 chunk 的边界前缀，至多保留一个分隔长度）。
    pending: Vec<u8>,
    part_name: Option<String>,
    field_buf: String,
    file_bytes: u64,
    total_bytes: u64,
    seen_file: bool,
    seen_sha256: bool,
}

impl<'b> MultipartParser<'b> {
    pub fn new(boundary: &'b str, limits: UploadLimits) -> Self {
        Self {
            boundary: boundary.as_bytes(),
            limits,
            phase: Phase::Preamble,
            pending: Vec::new(),
            part_name: None,
            field_buf: String::new(),
            file_bytes: 0,
            total_bytes: 0,
            seen_file: false,
            seen_sha256: false,
        }
    }

    /// 喂入一个 body chunk，一次返回该 chunk 所能解析出的全部事件。
    /// 未命中完整边界时内部保留尾部字节，等待后续 chunk 补齐。
    pub fn feed(&mut self, chunk: &[u8]) -> Result<Vec<MultipartEvent>, String> {
        self.total_bytes = self
            .total_bytes
            .checked_add(chunk.len() as u64)
            .ok_or_else(|| "total request size overflow".to_string())?;
        if self.total_bytes > self.limits.max_total_bytes {
            return Err(format!(
                "request body exceeds upload limit ({})",
                self.limits.max_total_bytes
            ));
        }
        self.pending.extend_from_slice(chunk);
        let mut events = Vec::new();
        loop {
            match self.parse_one()? {
                ParseStep::Event(event) => events.push(event),
                ParseStep::Progress => continue,
                ParseStep::Blocked => break,
            }
        }
        if self.phase == Phase::Finished && !self.pending.is_empty() {
            return Err("multipart body has trailing data after closing boundary".to_string());
        }
        Ok(events)
    }

    /// body 已读完时调用：要求遇到结束边界，且必须包含非空 `file` part。
    pub fn finish(self) -> Result<(), String> {
        if self.phase != Phase::Finished {
            return Err("multipart body missing closing boundary".to_string());
        }
        if !self.seen_file {
            return Err("multipart body missing required file part".to_string());
        }
        if self.file_bytes == 0 {
            return Err("multipart file part is empty".to_string());
        }
        Ok(())
    }

    fn parse_one(&mut self) -> Result<ParseStep, String> {
        match self.phase {
            Phase::Preamble => {
                let delim = self.opening_delim();
                let Some(pos) = find_subslice(&self.pending, &delim) else {
                    if self.pending.len() > self.boundary.len() + 8 {
                        return Err("multipart boundary not found".to_string());
                    }
                    return Ok(ParseStep::Blocked);
                };
                self.pending.drain(..pos + delim.len());
                self.phase = Phase::Headers;
                Ok(ParseStep::Progress)
            }
            Phase::Headers => {
                let Some(end) = find_subslice(&self.pending, b"\r\n\r\n") else {
                    if self.pending.len() > self.limits.max_header_bytes {
                        return Err("multipart part headers exceed limit".to_string());
                    }
                    return Ok(ParseStep::Blocked);
                };
                if end > self.limits.max_header_bytes {
                    return Err("multipart part headers exceed limit".to_string());
                }
                let headers = std::str::from_utf8(&self.pending[..end])
                    .map_err(|_| "multipart part headers are not utf8".to_string())?
                    .lines()
                    .map(|line| line.to_string())
                    .collect::<Vec<_>>();
                let name = header_name_param(&headers)?;
                if name == "file" {
                    if self.seen_file {
                        return Err("duplicate file part".to_string());
                    }
                    self.seen_file = true;
                } else if name == "sha256" {
                    if self.seen_sha256 {
                        return Err("duplicate sha256 part".to_string());
                    }
                    self.seen_sha256 = true;
                }
                self.pending.drain(..end + 4);
                self.part_name = Some(name);
                self.phase = Phase::Content;
                Ok(ParseStep::Progress)
            }
            Phase::Content => {
                let part_name = self.part_name.as_deref().unwrap_or_default();
                let delim = closing_delim(self.boundary);
                if let Some(pos) = find_subslice(&self.pending, &delim) {
                    if part_name == "file" {
                        self.file_bytes = self
                            .file_bytes
                            .checked_add(pos as u64)
                            .ok_or_else(|| "file size overflow".to_string())?;
                        if self.file_bytes > self.limits.max_archive_bytes {
                            return Err(format!(
                                "archive exceeds max_archive_bytes ({})",
                                self.limits.max_archive_bytes
                            ));
                        }
                        let chunk = self.pending[..pos].to_vec();
                        self.pending.drain(..pos + delim.len());
                        if self.pending.starts_with(b"--") || self.pending == b"-" {
                            self.enter_closing()?;
                            self.part_name = None;
                            return Ok(ParseStep::Event(MultipartEvent::FileChunk(chunk)));
                        }
                        if self.pending.starts_with(b"\r\n") {
                            self.pending.drain(..2);
                            self.phase = Phase::Headers;
                        } else if self.pending == b"\r" {
                            // 下一 part 开头的 CRLF 被切分，保持到下一 chunk。
                            self.phase = Phase::Headers;
                        } else if self.pending.is_empty() {
                            self.phase = Phase::Boundary;
                        } else if !self.pending.is_empty() {
                            return Err("malformed multipart boundary delimiter".to_string());
                        }
                        self.part_name = None;
                        return Ok(ParseStep::Event(MultipartEvent::FileChunk(chunk)));
                    }

                    if pos > 0 {
                        let tail = std::str::from_utf8(&self.pending[..pos])
                            .map_err(|_| "multipart field is not utf8".to_string())?;
                        if self.field_buf.len() + tail.len() > self.limits.max_field_bytes {
                            return Err("multipart field exceeds limit".to_string());
                        }
                        self.field_buf.push_str(tail);
                    }
                    let name = self.part_name.clone().unwrap_or_default();
                    let value = self.field_buf.trim_end_matches('\r').to_string();
                    self.field_buf = String::new();
                    if value.is_empty() {
                        return Err("multipart part has empty content".to_string());
                    }
                    self.pending.drain(..pos + delim.len());
                    if self.pending.starts_with(b"--") || self.pending == b"-" {
                        self.enter_closing()?;
                        self.part_name = None;
                        return Ok(ParseStep::Event(MultipartEvent::Field { name, value }));
                    } else if self.pending.starts_with(b"\r\n") {
                        self.pending.drain(..2);
                        self.phase = Phase::Headers;
                    } else if self.pending == b"\r" {
                        // 下一 part 开头的 CRLF 被切分，保持到下一 chunk。
                        self.phase = Phase::Headers;
                    } else if self.pending.is_empty() {
                        self.phase = Phase::Boundary;
                    } else if !self.pending.is_empty() {
                        return Err("malformed multipart boundary delimiter".to_string());
                    }
                    self.part_name = None;
                    return Ok(ParseStep::Event(MultipartEvent::Field { name, value }));
                }

                let keep = delim.len().saturating_sub(1);
                let emit = self.pending.len().saturating_sub(keep);
                if part_name == "file" {
                    if emit > 0 {
                        self.file_bytes = self
                            .file_bytes
                            .checked_add(emit as u64)
                            .ok_or_else(|| "file size overflow".to_string())?;
                        if self.file_bytes > self.limits.max_archive_bytes {
                            return Err(format!(
                                "archive exceeds max_archive_bytes ({})",
                                self.limits.max_archive_bytes
                            ));
                        }
                        let chunk = self.pending[..emit].to_vec();
                        self.pending.drain(..emit);
                        self.pending.shrink_to_fit();
                        return Ok(ParseStep::Event(MultipartEvent::FileChunk(chunk)));
                    }
                    return Ok(ParseStep::Blocked);
                }
                if emit > 0 {
                    let value = std::str::from_utf8(&self.pending[..emit])
                        .map_err(|_| "multipart field is not utf8".to_string())?;
                    if self.field_buf.len() + value.len() > self.limits.max_field_bytes {
                        return Err("multipart field exceeds limit".to_string());
                    }
                    self.field_buf.push_str(value);
                    self.pending.drain(..emit);
                    self.pending.shrink_to_fit();
                    return Ok(ParseStep::Progress);
                }
                return Ok(ParseStep::Blocked);
            }
            Phase::Boundary => {
                if self.pending.is_empty() || self.pending == b"\r" {
                    return Ok(ParseStep::Blocked);
                }
                if self.pending.starts_with(b"\r\n") {
                    self.pending.drain(..2);
                    self.phase = Phase::Headers;
                    return Ok(ParseStep::Progress);
                }
                if self.pending.starts_with(b"--") || self.pending == b"-" {
                    self.enter_closing()?;
                    return Ok(ParseStep::Progress);
                }
                Err("malformed multipart boundary delimiter".to_string())
            }
            Phase::Closing => {
                if self.pending == b"-" || self.pending == b"--" || self.pending == b"--\r" {
                    return Ok(ParseStep::Blocked);
                }
                self.enter_closing()?;
                Ok(ParseStep::Progress)
            }
            Phase::Finished => Ok(ParseStep::Blocked),
        }
    }

    /// 关闭边界处理：`--boundary--\r\n` 的收尾。`--`/`--\r` 可能被 chunk
    /// 切分，先在 Closing 相位等齐再判定。
    fn enter_closing(&mut self) -> Result<(), String> {
        if self.pending == b"-" || self.pending == b"--" || self.pending == b"--\r" {
            self.phase = Phase::Closing;
            return Ok(());
        }
        if !self.pending.starts_with(b"--") {
            return Err("malformed closing boundary".to_string());
        }
        if !self.pending.starts_with(b"--\r\n") {
            return Err("malformed closing boundary".to_string());
        }
        self.pending.drain(..4);
        if !self.pending.is_empty() {
            return Err("multipart body has trailing data after closing boundary".to_string());
        }
        self.phase = Phase::Finished;
        Ok(())
    }

    fn opening_delim(&self) -> Vec<u8> {
        let mut delimiter = b"--".to_vec();
        delimiter.extend_from_slice(self.boundary);
        delimiter.extend_from_slice(b"\r\n");
        delimiter
    }
}

fn closing_delim(boundary: &[u8]) -> Vec<u8> {
    let mut delimiter = b"\r\n--".to_vec();
    delimiter.extend_from_slice(boundary);
    delimiter
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn header_name_param(headers: &[String]) -> Result<String, String> {
    for header in headers {
        if header
            .to_ascii_lowercase()
            .starts_with("content-disposition:")
        {
            let value = header.splitn(2, ':').nth(1).unwrap_or("");
            for part in split_semicolons(value) {
                let part = part.trim();
                if let Some(v) = part.strip_prefix("name=") {
                    let v = v.trim_matches('"');
                    if v.is_empty() {
                        return Err("empty multipart field name".to_string());
                    }
                    return Ok(v.to_string());
                }
            }
        }
    }
    Err("missing name in multipart part header".to_string())
}

fn split_semicolons(value: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    for (i, byte) in value.bytes().enumerate() {
        if byte == b';' {
            out.push(&value[start..i]);
            start = i + 1;
        }
    }
    out.push(&value[start..]);
    out
}
