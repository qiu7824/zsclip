#![cfg_attr(windows, allow(dead_code))]

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

pub(crate) const LAN_DISCOVERY_PORT_DEFAULT: u16 = 38472;
pub(crate) const LAN_TCP_PORT_DEFAULT: u16 = 38473;
pub(crate) const LAN_IMAGE_MAX_BYTES: usize = 10 * 1024 * 1024;
pub(crate) const LAN_FILE_AUTO_MAX_BYTES: u64 = 50 * 1024 * 1024;

pub(crate) const LAN_MAGIC: &str = "ZSCLIP_LAN_V1";
pub(crate) const LAN_PROTOCOL: u32 = 1;
pub(crate) const HTTP_MAX_BODY: usize = 12 * 1024 * 1024;
pub(crate) const DISCOVERY_INTERVAL_MS: u64 = 5000;
pub(crate) const LAN_FILE_MAX_BYTES: u64 = 1024 * 1024 * 1024;
pub(crate) const LAN_FILE_CHUNK_BYTES: usize = 512 * 1024;
pub(crate) const MOBILE_IMAGE_LIST_LIMIT: i64 = 50;
pub(crate) const MOBILE_ITEM_LIST_LIMIT_DEFAULT: i64 = 50;
pub(crate) const MOBILE_ITEM_LIST_LIMIT_MAX: i64 = 100;
pub(crate) const WPS_TASKPANE_ITEM_LIMIT: i64 = 80;
pub(crate) const LAN_SEEN_MESSAGE_MAX: usize = 4096;
pub(crate) const LAN_PAIR_REQUEST_TTL_MS: u64 = 10 * 60 * 1000;
pub(crate) const LAN_TCP_BIND_CANDIDATE_COUNT: u16 = 20;

static LAN_TOKEN_COUNTER: AtomicU64 = AtomicU64::new(1);

fn current_time_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

pub(crate) fn lan_tcp_bind_candidates(
    base: u16,
    loopback_only: bool,
) -> impl Iterator<Item = SocketAddr> {
    let host = if loopback_only {
        [127, 0, 0, 1]
    } else {
        [0, 0, 0, 0]
    };
    (0..LAN_TCP_BIND_CANDIDATE_COUNT).filter_map(move |offset| {
        base.checked_add(offset)
            .map(|port| SocketAddr::from((host, port)))
    })
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct LanDevice {
    pub(crate) device_id: String,
    pub(crate) name: String,
    pub(crate) addr: String,
    pub(crate) tcp_port: u16,
    pub(crate) token: String,
    pub(crate) last_seen_ms: u64,
    pub(crate) trusted: bool,
    #[serde(default)]
    pub(crate) capabilities: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct LanFileMeta {
    pub(crate) name: String,
    pub(crate) size: u64,
    pub(crate) relative_path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct LanClipEnvelope {
    pub(crate) message_id: String,
    pub(crate) origin_device_id: String,
    pub(crate) origin_seq: u64,
    pub(crate) kind: String,
    pub(crate) hash: String,
    pub(crate) created_at_ms: u64,
    pub(crate) preview: String,
    pub(crate) text: Option<String>,
    pub(crate) image_png_base64: Option<String>,
    #[serde(default)]
    pub(crate) file_meta: Vec<LanFileMeta>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LanIncomingClip {
    pub(crate) envelope: LanClipEnvelope,
    pub(crate) source_device_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LanPairPrompt {
    pub(crate) pair_id: String,
    pub(crate) code: String,
    pub(crate) device_name: String,
    pub(crate) addr: String,
    pub(crate) created_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LanPendingPair {
    pub(crate) prompt: LanPairPrompt,
    pub(crate) requester_device_id: String,
    pub(crate) requester_tcp_port: u16,
    pub(crate) requester_capabilities: Vec<String>,
    pub(crate) token: String,
    pub(crate) accepted: bool,
    pub(crate) rejected: bool,
    pub(crate) created_at_ms: u64,
}

impl LanPendingPair {
    pub(crate) fn is_active(&self, now_ms: u64) -> bool {
        !self.accepted
            && !self.rejected
            && now_ms.saturating_sub(self.created_at_ms) < LAN_PAIR_REQUEST_TTL_MS
    }

    pub(crate) fn mark_accepted(&mut self) {
        self.accepted = true;
        self.rejected = false;
    }

    pub(crate) fn mark_rejected(&mut self) {
        self.rejected = true;
        self.accepted = false;
    }

    pub(crate) fn to_trusted_device(&self, now_ms: u64) -> LanDevice {
        LanDevice {
            device_id: self.requester_device_id.clone(),
            name: self.prompt.device_name.clone(),
            addr: self.prompt.addr.clone(),
            tcp_port: self.requester_tcp_port,
            token: self.token.clone(),
            last_seen_ms: now_ms,
            trusted: true,
            capabilities: normalize_lan_capabilities(
                self.requester_capabilities.clone(),
                self.requester_tcp_port,
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LanRuntimeCoreConfig {
    pub(crate) device_id: String,
    pub(crate) device_name: String,
    pub(crate) tcp_port: u16,
    pub(crate) udp_port: u16,
    pub(crate) lan_enabled: bool,
    pub(crate) wps_taskpane_enabled: bool,
}

#[derive(Clone)]
pub(crate) struct LanRuntimeConfig {
    pub(crate) platform: LanRuntimePlatformContext,
    pub(crate) device_id: String,
    pub(crate) device_name: String,
    pub(crate) tcp_port: u16,
    pub(crate) udp_port: u16,
    pub(crate) lan_enabled: bool,
    pub(crate) wps_taskpane_enabled: bool,
}

impl LanRuntimeConfig {
    pub(crate) fn from_core_config(
        platform: LanRuntimePlatformContext,
        core_config: LanRuntimeCoreConfig,
    ) -> Self {
        Self {
            platform,
            device_id: core_config.device_id,
            device_name: core_config.device_name,
            tcp_port: core_config.tcp_port,
            udp_port: core_config.udp_port,
            lan_enabled: core_config.lan_enabled,
            wps_taskpane_enabled: core_config.wps_taskpane_enabled,
        }
    }

    pub(crate) fn core_config(&self) -> LanRuntimeCoreConfig {
        LanRuntimeCoreConfig {
            device_id: self.device_id.clone(),
            device_name: self.device_name.clone(),
            tcp_port: self.tcp_port,
            udp_port: self.udp_port,
            lan_enabled: self.lan_enabled,
            wps_taskpane_enabled: self.wps_taskpane_enabled,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LanServiceRuntimeState {
    pub(crate) device_id: String,
    pub(crate) tcp_port: u16,
    pub(crate) udp_port: u16,
    pub(crate) lan_enabled: bool,
    pub(crate) wps_taskpane_enabled: bool,
}

impl LanServiceRuntimeState {
    pub(crate) fn from_core_config(config: &LanRuntimeCoreConfig, actual_tcp_port: u16) -> Self {
        Self {
            device_id: config.device_id.clone(),
            tcp_port: actual_tcp_port,
            udp_port: config.udp_port,
            lan_enabled: config.lan_enabled,
            wps_taskpane_enabled: config.wps_taskpane_enabled,
        }
    }

    pub(crate) fn requires_restart_for(&self, config: &LanRuntimeCoreConfig) -> bool {
        self.device_id != config.device_id
            || self.tcp_port != config.tcp_port
            || self.udp_port != config.udp_port
            || self.lan_enabled != config.lan_enabled
            || self.wps_taskpane_enabled != config.wps_taskpane_enabled
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LanServiceLifecyclePlan {
    pub(crate) tcp_port: u16,
    pub(crate) bind_loopback_only: bool,
    pub(crate) start_tcp_server: bool,
    pub(crate) start_udp_listener: bool,
    pub(crate) start_udp_sender: bool,
    pub(crate) wake_udp_on_stop: bool,
}

impl LanServiceLifecyclePlan {
    pub(crate) fn for_config(config: &LanRuntimeCoreConfig) -> Self {
        Self {
            tcp_port: config.tcp_port,
            bind_loopback_only: !config.lan_enabled,
            start_tcp_server: true,
            start_udp_listener: config.lan_enabled,
            start_udp_sender: config.lan_enabled,
            wake_udp_on_stop: config.lan_enabled,
        }
    }

    pub(crate) fn for_state(state: &LanServiceRuntimeState) -> Self {
        Self {
            tcp_port: state.tcp_port,
            bind_loopback_only: !state.lan_enabled,
            start_tcp_server: true,
            start_udp_listener: state.lan_enabled,
            start_udp_sender: state.lan_enabled,
            wake_udp_on_stop: state.lan_enabled,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LanHttpRoutePolicy {
    pub(crate) lan_enabled: bool,
    pub(crate) wps_taskpane_enabled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LanHttpRoute {
    Info,
    PairRequest,
    PairStatus,
    Clip,
    Latest,
    WpsTaskpane,
    WpsItems,
    WpsImage,
    WpsEvents,
    MobileSetup,
    MobileImages,
    MobileImage,
    MobileItems,
    MobileItemImage,
    MobileItemFile,
    MultiSyncManifest,
    FileStart,
    FileChunk,
    FileFinish,
    MultiSyncFile,
    NotFound,
}

impl LanHttpRoute {
    pub(crate) fn is_wps_taskpane(self) -> bool {
        matches!(
            self,
            Self::WpsTaskpane | Self::WpsItems | Self::WpsImage | Self::WpsEvents
        )
    }
}

impl LanHttpRoutePolicy {
    pub(crate) fn for_config(config: &LanRuntimeCoreConfig) -> Self {
        Self {
            lan_enabled: config.lan_enabled,
            wps_taskpane_enabled: config.wps_taskpane_enabled,
        }
    }

    pub(crate) fn route_available(&self, method: &str, path: &str) -> bool {
        self.route_available_for_route(lan_http_route_for(method, path))
    }

    pub(crate) fn route_available_for_route(&self, route: LanHttpRoute) -> bool {
        if route.is_wps_taskpane() {
            return self.wps_taskpane_enabled;
        }
        self.lan_enabled
    }
}

pub(crate) fn is_wps_taskpane_route(method: &str, path: &str) -> bool {
    lan_http_route_for(method, path).is_wps_taskpane()
}

pub(crate) fn lan_http_route_for(method: &str, path: &str) -> LanHttpRoute {
    match (method, path) {
        ("GET", "/v1/info") => LanHttpRoute::Info,
        ("POST", "/v1/pair/request") => LanHttpRoute::PairRequest,
        ("GET", "/v1/pair/status") => LanHttpRoute::PairStatus,
        ("POST", "/v1/clip") => LanHttpRoute::Clip,
        ("GET", "/v1/latest") => LanHttpRoute::Latest,
        ("GET", "/office/wps/taskpane") => LanHttpRoute::WpsTaskpane,
        ("GET", "/office/wps/items") => LanHttpRoute::WpsItems,
        ("GET", "/office/wps/image") => LanHttpRoute::WpsImage,
        ("GET", "/office/wps/events") => LanHttpRoute::WpsEvents,
        ("GET", "/mobile/setup") => LanHttpRoute::MobileSetup,
        ("GET", "/mobile/images") => LanHttpRoute::MobileImages,
        ("GET", "/mobile/image") => LanHttpRoute::MobileImage,
        ("GET", "/v1/mobile/items") => LanHttpRoute::MobileItems,
        ("GET", item_path) if mobile_item_image_path_id(item_path).is_some() => {
            LanHttpRoute::MobileItemImage
        }
        ("GET", item_path) if mobile_item_file_path_parts(item_path).is_some() => {
            LanHttpRoute::MobileItemFile
        }
        ("GET", manifest_path) if is_multi_sync_manifest_path(manifest_path) => {
            LanHttpRoute::MultiSyncManifest
        }
        ("POST", "/v1/file/start") => LanHttpRoute::FileStart,
        ("POST", "/v1/file/chunk") => LanHttpRoute::FileChunk,
        ("POST", "/v1/file/finish") => LanHttpRoute::FileFinish,
        ("GET", file_path) if file_path.starts_with("/file/") => LanHttpRoute::MultiSyncFile,
        _ => LanHttpRoute::NotFound,
    }
}

pub(crate) fn is_multi_sync_manifest_path(path: &str) -> bool {
    let Some(file_name) = path.strip_prefix('/') else {
        return false;
    };
    file_name == crate::multi_sync::MULTI_SYNC_MANIFEST_FILE_NAME
        || file_name == crate::multi_sync::MULTI_SYNC_LEGACY_MANIFEST_FILE_NAME
}

pub(crate) fn mobile_item_image_path_id(path: &str) -> Option<i64> {
    let rest = path.strip_prefix("/v1/mobile/items/")?;
    let id = rest.strip_suffix("/image")?;
    id.parse::<i64>().ok().filter(|id| *id > 0)
}

pub(crate) fn mobile_item_file_path_parts(path: &str) -> Option<(i64, usize)> {
    let rest = path.strip_prefix("/v1/mobile/items/")?;
    let (id, index) = rest.split_once("/file/")?;
    if index.contains('/') || index.is_empty() {
        return None;
    }
    let id = id.parse::<i64>().ok().filter(|id| *id > 0)?;
    let index = index.parse::<usize>().ok()?;
    Some((id, index))
}

pub(crate) fn lan_desktop_capabilities() -> Vec<String> {
    ["text", "image", "latest", "manual_file", "receive_clip"]
        .iter()
        .map(|value| value.to_string())
        .collect()
}

pub(crate) fn normalize_lan_capabilities(
    mut capabilities: Vec<String>,
    tcp_port: u16,
) -> Vec<String> {
    capabilities = capabilities
        .into_iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect();
    capabilities.sort_unstable();
    capabilities.dedup();
    if !capabilities.is_empty() {
        return capabilities;
    }
    if tcp_port == 0 {
        vec!["client_only".to_string(), "pull_only".to_string()]
    } else {
        lan_desktop_capabilities()
    }
}

pub(crate) fn lan_device_can_receive_clip(device: &LanDevice) -> bool {
    device.trusted
        && device.tcp_port > 0
        && !device.capabilities.iter().any(|cap| {
            cap.eq_ignore_ascii_case("client_only") || cap.eq_ignore_ascii_case("pull_only")
        })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HttpRequest {
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) body: Vec<u8>,
    pub(crate) peer: SocketAddr,
}

impl HttpRequest {
    pub(crate) fn path_without_query(&self) -> &str {
        http_path_without_query(&self.path)
    }

    pub(crate) fn header_value(&self, name: &str) -> Option<&str> {
        header_value(self, name)
    }

    pub(crate) fn query_param(&self, key: &str) -> String {
        query_param(&self.path, key)
    }

    pub(crate) fn query_param_decoded(&self, key: &str) -> String {
        query_param_decoded(&self.path, key)
    }
}

pub(crate) fn http_path_without_query(path: &str) -> &str {
    path.split('?').next().unwrap_or(path)
}

pub(crate) fn read_http_request(
    reader: &mut impl Read,
    peer: SocketAddr,
) -> io::Result<HttpRequest> {
    let mut buf = Vec::with_capacity(8192);
    let mut tmp = [0u8; 4096];
    let mut header_end = None;
    while header_end.is_none() && buf.len() < 64 * 1024 {
        let n = reader.read(&mut tmp)?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        header_end = find_header_end(&buf);
    }
    let Some(header_end) = header_end else {
        return Err(io::Error::from(io::ErrorKind::InvalidData));
    };
    let header_text = String::from_utf8_lossy(&buf[..header_end]);
    let mut lines = header_text.lines();
    let request_line = lines.next().unwrap_or_default();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or("/").to_string();
    let mut headers = Vec::new();
    let mut content_len = 0usize;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim().to_string();
        let value = value.trim().to_string();
        if name.eq_ignore_ascii_case("content-length") {
            content_len = value.parse::<usize>().unwrap_or(0);
        }
        headers.push((name, value));
    }
    if content_len > HTTP_MAX_BODY {
        return Err(io::Error::from(io::ErrorKind::InvalidData));
    }
    let body_start = header_end + 4;
    let mut body = buf.get(body_start..).unwrap_or_default().to_vec();
    while body.len() < content_len {
        let n = reader.read(&mut tmp)?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&tmp[..n]);
    }
    body.truncate(content_len);
    Ok(HttpRequest {
        method,
        path,
        headers,
        body,
        peer,
    })
}

pub(crate) fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

pub(crate) fn write_http_json(
    writer: &mut impl Write,
    status: u16,
    value: &serde_json::Value,
) -> io::Result<()> {
    let body = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
    write!(
        writer,
        "HTTP/1.1 {} {}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        status,
        http_reason(status),
        body.len()
    )?;
    writer.write_all(&body)
}

pub(crate) fn write_http_bytes(
    writer: &mut impl Write,
    status: u16,
    content_type: &str,
    body: &[u8],
    extra_headers: &[(&str, String)],
) -> io::Result<()> {
    write!(
        writer,
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n",
        status,
        http_reason(status),
        content_type,
        body.len()
    )?;
    for (name, value) in extra_headers {
        write!(writer, "{name}: {value}\r\n")?;
    }
    writer.write_all(b"\r\n")?;
    writer.write_all(body)
}

pub(crate) fn write_http_file(
    writer: &mut impl Write,
    status: u16,
    content_type: &str,
    path: &Path,
    content_len: u64,
    extra_headers: &[(&str, String)],
) -> io::Result<()> {
    write!(
        writer,
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n",
        status,
        http_reason(status),
        content_type,
        content_len
    )?;
    for (name, value) in extra_headers {
        write!(writer, "{name}: {value}\r\n")?;
    }
    writer.write_all(b"\r\n")?;
    let mut file = fs::File::open(path)?;
    io::copy(&mut file, writer)?;
    Ok(())
}

pub(crate) fn http_reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        413 => "Payload Too Large",
        501 => "Not Implemented",
        _ => "OK",
    }
}

pub(crate) fn http_request(
    method: &str,
    host: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: Option<&[u8]>,
    timeout: Duration,
) -> io::Result<Vec<u8>> {
    let mut stream = TcpStream::connect(host)?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    let body = body.unwrap_or_default();
    write!(
        stream,
        "{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Length: {}\r\n",
        method,
        path,
        host,
        body.len()
    )?;
    for (name, value) in headers {
        write!(stream, "{name}: {value}\r\n")?;
    }
    stream.write_all(b"\r\n")?;
    stream.write_all(body)?;
    let mut resp = Vec::new();
    stream.read_to_end(&mut resp)?;
    let Some(header_end) = find_header_end(&resp) else {
        return Ok(resp);
    };
    Ok(resp.get(header_end + 4..).unwrap_or_default().to_vec())
}

pub(crate) fn normalize_lan_host(raw: &str, default_port: u16) -> String {
    let mut s = raw.trim().to_string();
    if let Some(rest) = s.strip_prefix("http://") {
        s = rest.to_string();
    }
    if let Some(rest) = s.strip_prefix("https://") {
        s = rest.to_string();
    }
    if let Some((host, _path)) = s.split_once('/') {
        s = host.to_string();
    }
    if s.is_empty() {
        return String::new();
    }
    if s.contains(':') {
        s
    } else {
        format!("{s}:{default_port}")
    }
}

pub(crate) fn url_encode_component(value: &str) -> String {
    let mut out = String::new();
    for byte in value.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

pub(crate) fn safe_lan_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            ch if ch.is_control() => '_',
            ch => ch,
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.').trim();
    if trimmed.is_empty() {
        "file.bin".to_string()
    } else {
        trimmed.chars().take(120).collect()
    }
}

pub(crate) fn lan_file_session_key(source_device_id: &str, transfer_id: &str) -> String {
    format!("{}:{}", source_device_id.trim(), transfer_id.trim())
}

pub(crate) fn lan_file_content_hasher(total_size: u64) -> crc32fast::Hasher {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(&(b"file".len() as u64).to_le_bytes());
    hasher.update(b"file");
    hasher.update(&total_size.to_le_bytes());
    hasher
}

pub(crate) fn lan_hash_string(text: &str) -> String {
    format!("{:x}", md5::compute(text.as_bytes()))
}

pub(crate) fn make_lan_pair_code(a: &str, b: &str, now_ms: u64) -> String {
    let seed = format!("{}:{}:{}", a, b, now_ms / 60_000);
    let digest = md5::compute(seed.as_bytes());
    let value = u32::from_le_bytes([digest[0], digest[1], digest[2], digest[3]]) % 1_000_000;
    format!("{value:06}")
}

pub(crate) fn make_lan_token(bytes: usize, now_ms: u64) -> String {
    let mut out = String::new();
    while out.len() < bytes * 2 {
        let seed = format!(
            "{}:{}:{}",
            now_ms,
            std::process::id(),
            LAN_TOKEN_COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let digest = md5::compute(seed.as_bytes());
        for byte in digest.0 {
            out.push_str(&format!("{byte:02x}"));
            if out.len() >= bytes * 2 {
                break;
            }
        }
    }
    out
}

pub(crate) fn remember_lan_seen_message_key(seen: &mut HashSet<String>, key: String) -> bool {
    if seen.len() > LAN_SEEN_MESSAGE_MAX {
        seen.clear();
    }
    seen.insert(key)
}

pub(crate) fn lan_pair_status_response_value(
    pair: Option<&LanPendingPair>,
    config: &LanRuntimeCoreConfig,
    capabilities: Vec<String>,
) -> serde_json::Value {
    let Some(pair) = pair else {
        return serde_json::json!({"status":"missing"});
    };
    if pair.rejected {
        return serde_json::json!({"status":"rejected"});
    }
    if pair.accepted {
        return serde_json::json!({
            "status":"accepted",
            "token": pair.token,
            "device_id": config.device_id,
            "name": config.device_name,
            "tcp_port": config.tcp_port,
            "capabilities": capabilities
        });
    }
    serde_json::json!({"status":"pending"})
}

pub(crate) fn header_value<'a>(req: &'a HttpRequest, name: &str) -> Option<&'a str> {
    req.headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

pub(crate) fn query_param(path: &str, key: &str) -> String {
    let Some((_, query)) = path.split_once('?') else {
        return String::new();
    };
    for part in query.split('&') {
        let Some((k, v)) = part.split_once('=') else {
            continue;
        };
        if k == key {
            return v.to_string();
        }
    }
    String::new()
}

pub(crate) fn query_param_decoded(path: &str, key: &str) -> String {
    percent_decode(&query_param(path, key))
}

pub(crate) fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex_value(bytes[i + 1]), hex_value(bytes[i + 2])) {
                out.push((hi << 4) | lo);
                i += 3;
                continue;
            }
        }
        out.push(if bytes[i] == b'+' { b' ' } else { bytes[i] });
        i += 1;
    }
    String::from_utf8(out).unwrap_or_default()
}

fn hex_value(ch: u8) -> Option<u8> {
    match ch {
        b'0'..=b'9' => Some(ch - b'0'),
        b'a'..=b'f' => Some(ch - b'a' + 10),
        b'A'..=b'F' => Some(ch - b'A' + 10),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LanRuntimeSettings {
    pub(crate) lan_sync_enabled: bool,
    pub(crate) wps_taskpane_enabled: bool,
    pub(crate) device_id: String,
    pub(crate) device_name: String,
    pub(crate) tcp_port: u16,
    pub(crate) udp_port: u16,
}

impl LanRuntimeSettings {
    pub(crate) fn runtime_enabled(&self) -> bool {
        self.lan_sync_enabled || self.wps_taskpane_enabled
    }

    pub(crate) fn normalized_tcp_port(&self) -> u16 {
        self.tcp_port.max(1)
    }

    pub(crate) fn normalized_udp_port(&self) -> u16 {
        self.udp_port.max(1)
    }

    pub(crate) fn core_config(&self) -> LanRuntimeCoreConfig {
        LanRuntimeCoreConfig {
            device_id: self.device_id.trim().to_string(),
            device_name: self.device_name.clone(),
            tcp_port: self.normalized_tcp_port(),
            udp_port: self.normalized_udp_port(),
            lan_enabled: self.lan_sync_enabled,
            wps_taskpane_enabled: self.wps_taskpane_enabled,
        }
    }

    pub(crate) fn pair_request_body(&self, capabilities: Vec<String>) -> PairRequestBody {
        PairRequestBody {
            device_id: self.device_id.clone(),
            name: self.device_name.clone(),
            tcp_port: self.normalized_tcp_port(),
            capabilities,
        }
    }
}

pub(crate) fn lan_runtime_settings_from_settings_json(
    settings_json: &serde_json::Value,
) -> LanRuntimeSettings {
    fn bool_field(settings_json: &serde_json::Value, key: &str) -> bool {
        settings_json
            .get(key)
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    }
    fn string_field(settings_json: &serde_json::Value, key: &str) -> String {
        settings_json
            .get(key)
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .trim()
            .to_string()
    }
    fn u16_field(settings_json: &serde_json::Value, key: &str, fallback: u16) -> u16 {
        settings_json
            .get(key)
            .and_then(|value| {
                value
                    .as_u64()
                    .and_then(|number| u16::try_from(number).ok())
                    .or_else(|| {
                        value
                            .as_str()
                            .and_then(|text| text.trim().parse::<u16>().ok())
                    })
            })
            .filter(|port| *port > 0)
            .unwrap_or(fallback)
    }

    LanRuntimeSettings {
        lan_sync_enabled: bool_field(settings_json, "lan_sync_enabled"),
        wps_taskpane_enabled: bool_field(settings_json, "wps_taskpane_enabled"),
        device_id: string_field(settings_json, "lan_device_id"),
        device_name: string_field(settings_json, "lan_device_name"),
        tcp_port: u16_field(settings_json, "lan_tcp_port", LAN_TCP_PORT_DEFAULT),
        udp_port: u16_field(settings_json, "lan_udp_port", LAN_DISCOVERY_PORT_DEFAULT),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LanRuntimeEventSink {
    None,
    PlatformMainWindow { raw_handle: isize },
}

impl LanRuntimeEventSink {
    pub(crate) fn platform_main_window(raw_handle: isize) -> Self {
        if raw_handle == 0 {
            Self::None
        } else {
            Self::PlatformMainWindow { raw_handle }
        }
    }

    pub(crate) fn raw_platform_main_window_handle(&self) -> Option<isize> {
        match self {
            Self::None => None,
            Self::PlatformMainWindow { raw_handle } if *raw_handle != 0 => Some(*raw_handle),
            Self::PlatformMainWindow { .. } => None,
        }
    }
}

pub(crate) type LanSecretCodec = fn(&str) -> Option<String>;

#[derive(Clone)]
pub(crate) struct LanRuntimePlatformContext {
    pub(crate) data_dir: PathBuf,
    pub(crate) event_sink: LanRuntimeEventSink,
    pub(crate) encrypt_secret: LanSecretCodec,
    pub(crate) decrypt_secret: LanSecretCodec,
}

impl LanRuntimePlatformContext {
    pub(crate) fn new(
        data_dir: PathBuf,
        event_sink: LanRuntimeEventSink,
        encrypt_secret: LanSecretCodec,
        decrypt_secret: LanSecretCodec,
    ) -> Self {
        Self {
            data_dir,
            event_sink,
            encrypt_secret,
            decrypt_secret,
        }
    }

    pub(crate) fn device_book_path(&self) -> PathBuf {
        lan_device_book_path(&self.data_dir)
    }

    pub(crate) fn load_devices(
        &self,
        normalize_capabilities: impl FnMut(Vec<String>, u16) -> Vec<String>,
    ) -> Vec<LanDevice> {
        load_lan_devices_from_store(
            self.device_book_path(),
            self.decrypt_secret,
            normalize_capabilities,
        )
    }

    pub(crate) fn save_devices(
        &self,
        devices: &[LanDevice],
        normalize_capabilities: impl FnMut(Vec<String>, u16) -> Vec<String>,
    ) -> io::Result<()> {
        save_lan_devices_to_store(
            &self.data_dir,
            devices,
            self.encrypt_secret,
            normalize_capabilities,
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DiscoveryPacket {
    pub(crate) magic: String,
    pub(crate) protocol: u32,
    pub(crate) device_id: String,
    pub(crate) name: String,
    pub(crate) tcp_port: u16,
    pub(crate) capabilities: Vec<String>,
}

impl DiscoveryPacket {
    pub(crate) fn new(config: &LanRuntimeCoreConfig, capabilities: Vec<String>) -> Self {
        Self {
            magic: LAN_MAGIC.to_string(),
            protocol: LAN_PROTOCOL,
            device_id: config.device_id.clone(),
            name: config.device_name.clone(),
            tcp_port: config.tcp_port,
            capabilities,
        }
    }

    pub(crate) fn uses_current_protocol(&self) -> bool {
        self.magic == LAN_MAGIC && self.protocol == LAN_PROTOCOL
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct PairRequestBody {
    pub(crate) device_id: String,
    pub(crate) name: String,
    pub(crate) tcp_port: u16,
    #[serde(default)]
    pub(crate) capabilities: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub(crate) struct StoredDeviceBook {
    pub(crate) devices: Vec<StoredLanDevice>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct StoredLanDevice {
    pub(crate) device_id: String,
    pub(crate) name: String,
    pub(crate) addr: String,
    pub(crate) tcp_port: u16,
    pub(crate) token_encrypted: String,
    pub(crate) last_seen_ms: u64,
    pub(crate) trusted: bool,
    #[serde(default)]
    pub(crate) capabilities: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub(crate) struct StoredPendingPairBook {
    pub(crate) pairs: Vec<StoredLanPendingPair>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub(crate) struct StoredDiscoveredDeviceBook {
    pub(crate) devices: Vec<StoredLanDiscoveredDevice>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct StoredLanDiscoveredDevice {
    pub(crate) device_id: String,
    pub(crate) name: String,
    pub(crate) addr: String,
    pub(crate) tcp_port: u16,
    #[serde(default)]
    pub(crate) token_encrypted: Option<String>,
    pub(crate) last_seen_ms: u64,
    pub(crate) trusted: bool,
    #[serde(default)]
    pub(crate) capabilities: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct StoredLanPendingPair {
    pub(crate) pair_id: String,
    pub(crate) code: String,
    pub(crate) device_name: String,
    pub(crate) addr: String,
    pub(crate) created_at_ms: u64,
    pub(crate) requester_device_id: String,
    pub(crate) requester_tcp_port: u16,
    #[serde(default)]
    pub(crate) requester_capabilities: Vec<String>,
    pub(crate) token_encrypted: String,
    pub(crate) accepted: bool,
    pub(crate) rejected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LanPendingPairDecision {
    pub(crate) pair_id: String,
    pub(crate) accepted: bool,
    pub(crate) trusted_device: Option<LanDevice>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LanBackgroundClipSyncTarget {
    pub(crate) device_id: String,
    pub(crate) endpoint: String,
    pub(crate) can_push_clip: bool,
    pub(crate) can_pull_latest: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LanBackgroundClipSyncPlan {
    pub(crate) local_device_id: String,
    pub(crate) push_targets: Vec<LanBackgroundClipSyncTarget>,
    pub(crate) pull_targets: Vec<LanBackgroundClipSyncTarget>,
    pub(crate) has_latest_clip: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LanBackgroundClipSyncExecution {
    pub(crate) plan: LanBackgroundClipSyncPlan,
    pub(crate) pushed_count: usize,
    pub(crate) pulled_count: usize,
    pub(crate) failed_count: usize,
    pub(crate) incoming_clips: Vec<LanIncomingClip>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LanFilePayloadTransferPlan {
    pub(crate) local_device_id: String,
    pub(crate) target_device_ids: Vec<String>,
    pub(crate) file_paths: Vec<PathBuf>,
    pub(crate) total_bytes: u64,
    pub(crate) skipped_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LanFilePayloadTransferExecution {
    pub(crate) plan: LanFilePayloadTransferPlan,
    pub(crate) pushed_count: usize,
    pub(crate) failed_count: usize,
}

pub(crate) fn lan_devices_from_stored_book(
    book: StoredDeviceBook,
    mut decrypt_token: impl FnMut(&str) -> Option<String>,
    mut normalize_capabilities: impl FnMut(Vec<String>, u16) -> Vec<String>,
) -> Vec<LanDevice> {
    book.devices
        .into_iter()
        .filter_map(|stored| {
            let token = decrypt_token(&stored.token_encrypted)?;
            Some(LanDevice {
                device_id: stored.device_id,
                name: stored.name,
                addr: stored.addr,
                tcp_port: stored.tcp_port,
                token,
                last_seen_ms: stored.last_seen_ms,
                trusted: stored.trusted,
                capabilities: normalize_capabilities(stored.capabilities, stored.tcp_port),
            })
        })
        .collect()
}

pub(crate) fn stored_book_from_lan_devices(
    devices: &[LanDevice],
    mut encrypt_token: impl FnMut(&str) -> Option<String>,
    mut normalize_capabilities: impl FnMut(Vec<String>, u16) -> Vec<String>,
) -> StoredDeviceBook {
    StoredDeviceBook {
        devices: devices
            .iter()
            .filter(|device| device.trusted)
            .filter_map(|device| {
                let token_encrypted = encrypt_token(&device.token)?;
                Some(StoredLanDevice {
                    device_id: device.device_id.clone(),
                    name: device.name.clone(),
                    addr: device.addr.clone(),
                    tcp_port: device.tcp_port,
                    token_encrypted,
                    last_seen_ms: device.last_seen_ms,
                    trusted: device.trusted,
                    capabilities: normalize_capabilities(
                        device.capabilities.clone(),
                        device.tcp_port,
                    ),
                })
            })
            .collect(),
    }
}

pub(crate) fn lan_pending_pairs_from_stored_book(
    book: StoredPendingPairBook,
    mut decrypt_token: impl FnMut(&str) -> Option<String>,
    mut normalize_capabilities: impl FnMut(Vec<String>, u16) -> Vec<String>,
) -> Vec<LanPendingPair> {
    book.pairs
        .into_iter()
        .filter_map(|stored| {
            let token = decrypt_token(&stored.token_encrypted)?;
            Some(LanPendingPair {
                prompt: LanPairPrompt {
                    pair_id: stored.pair_id,
                    code: stored.code,
                    device_name: stored.device_name,
                    addr: stored.addr,
                    created_at_ms: stored.created_at_ms,
                },
                requester_device_id: stored.requester_device_id,
                requester_tcp_port: stored.requester_tcp_port,
                requester_capabilities: normalize_capabilities(
                    stored.requester_capabilities,
                    stored.requester_tcp_port,
                ),
                token,
                accepted: stored.accepted,
                rejected: stored.rejected,
                created_at_ms: stored.created_at_ms,
            })
        })
        .collect()
}

pub(crate) fn stored_book_from_lan_pending_pairs(
    pairs: &[LanPendingPair],
    mut encrypt_token: impl FnMut(&str) -> Option<String>,
    mut normalize_capabilities: impl FnMut(Vec<String>, u16) -> Vec<String>,
) -> StoredPendingPairBook {
    StoredPendingPairBook {
        pairs: pairs
            .iter()
            .filter_map(|pair| {
                let token_encrypted = encrypt_token(&pair.token)?;
                Some(StoredLanPendingPair {
                    pair_id: pair.prompt.pair_id.clone(),
                    code: pair.prompt.code.clone(),
                    device_name: pair.prompt.device_name.clone(),
                    addr: pair.prompt.addr.clone(),
                    created_at_ms: pair.created_at_ms,
                    requester_device_id: pair.requester_device_id.clone(),
                    requester_tcp_port: pair.requester_tcp_port,
                    requester_capabilities: normalize_capabilities(
                        pair.requester_capabilities.clone(),
                        pair.requester_tcp_port,
                    ),
                    token_encrypted,
                    accepted: pair.accepted,
                    rejected: pair.rejected,
                })
            })
            .collect(),
    }
}

pub(crate) fn lan_discovered_devices_from_stored_book(
    book: StoredDiscoveredDeviceBook,
    mut decrypt_token: impl FnMut(&str) -> Option<String>,
    mut normalize_capabilities: impl FnMut(Vec<String>, u16) -> Vec<String>,
) -> Vec<LanDevice> {
    book.devices
        .into_iter()
        .map(|stored| {
            let token = stored
                .token_encrypted
                .as_deref()
                .and_then(&mut decrypt_token)
                .unwrap_or_default();
            LanDevice {
                device_id: stored.device_id,
                name: stored.name,
                addr: stored.addr,
                tcp_port: stored.tcp_port,
                token,
                last_seen_ms: stored.last_seen_ms,
                trusted: stored.trusted,
                capabilities: normalize_capabilities(stored.capabilities, stored.tcp_port),
            }
        })
        .collect()
}

pub(crate) fn stored_book_from_lan_discovered_devices(
    devices: &[LanDevice],
    mut encrypt_token: impl FnMut(&str) -> Option<String>,
    mut normalize_capabilities: impl FnMut(Vec<String>, u16) -> Vec<String>,
) -> StoredDiscoveredDeviceBook {
    StoredDiscoveredDeviceBook {
        devices: devices
            .iter()
            .map(|device| StoredLanDiscoveredDevice {
                device_id: device.device_id.clone(),
                name: device.name.clone(),
                addr: device.addr.clone(),
                tcp_port: device.tcp_port,
                token_encrypted: (!device.token.trim().is_empty())
                    .then(|| encrypt_token(&device.token))
                    .flatten(),
                last_seen_ms: device.last_seen_ms,
                trusted: device.trusted,
                capabilities: normalize_capabilities(device.capabilities.clone(), device.tcp_port),
            })
            .collect(),
    }
}

pub(crate) fn lan_device_book_path(data_dir: impl AsRef<Path>) -> PathBuf {
    data_dir.as_ref().join("lan_devices.json")
}

pub(crate) fn lan_pending_pair_book_path(data_dir: impl AsRef<Path>) -> PathBuf {
    data_dir.as_ref().join("lan_pending_pairs.json")
}

pub(crate) fn lan_discovered_device_cache_path(data_dir: impl AsRef<Path>) -> PathBuf {
    data_dir.as_ref().join("lan_discovered_devices.json")
}

pub(crate) fn load_lan_devices_from_store(
    device_book_path: impl AsRef<Path>,
    decrypt_token: impl FnMut(&str) -> Option<String>,
    normalize_capabilities: impl FnMut(Vec<String>, u16) -> Vec<String>,
) -> Vec<LanDevice> {
    let text = fs::read_to_string(device_book_path).unwrap_or_default();
    let Ok(book) = serde_json::from_str::<StoredDeviceBook>(&text) else {
        return Vec::new();
    };
    lan_devices_from_stored_book(book, decrypt_token, normalize_capabilities)
}

pub(crate) fn load_lan_pending_pairs_from_store(
    pending_pair_book_path: impl AsRef<Path>,
    decrypt_token: impl FnMut(&str) -> Option<String>,
    normalize_capabilities: impl FnMut(Vec<String>, u16) -> Vec<String>,
) -> Vec<LanPendingPair> {
    let text = fs::read_to_string(pending_pair_book_path).unwrap_or_default();
    let Ok(book) = serde_json::from_str::<StoredPendingPairBook>(&text) else {
        return Vec::new();
    };
    lan_pending_pairs_from_stored_book(book, decrypt_token, normalize_capabilities)
}

pub(crate) fn load_lan_discovered_devices_from_store(
    discovered_device_cache_path: impl AsRef<Path>,
    decrypt_token: impl FnMut(&str) -> Option<String>,
    normalize_capabilities: impl FnMut(Vec<String>, u16) -> Vec<String>,
) -> Vec<LanDevice> {
    let text = fs::read_to_string(discovered_device_cache_path).unwrap_or_default();
    let Ok(book) = serde_json::from_str::<StoredDiscoveredDeviceBook>(&text) else {
        return Vec::new();
    };
    lan_discovered_devices_from_stored_book(book, decrypt_token, normalize_capabilities)
}

pub(crate) fn save_lan_devices_to_store(
    data_dir: impl AsRef<Path>,
    devices: &[LanDevice],
    encrypt_token: impl FnMut(&str) -> Option<String>,
    normalize_capabilities: impl FnMut(Vec<String>, u16) -> Vec<String>,
) -> io::Result<()> {
    let data_dir = data_dir.as_ref();
    fs::create_dir_all(data_dir)?;
    let stored = stored_book_from_lan_devices(devices, encrypt_token, normalize_capabilities);
    let text = serde_json::to_string_pretty(&stored)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    fs::write(lan_device_book_path(data_dir), text)
}

pub(crate) fn save_lan_pending_pairs_to_store(
    data_dir: impl AsRef<Path>,
    pairs: &[LanPendingPair],
    encrypt_token: impl FnMut(&str) -> Option<String>,
    normalize_capabilities: impl FnMut(Vec<String>, u16) -> Vec<String>,
) -> io::Result<()> {
    let data_dir = data_dir.as_ref();
    fs::create_dir_all(data_dir)?;
    let stored = stored_book_from_lan_pending_pairs(pairs, encrypt_token, normalize_capabilities);
    let text = serde_json::to_string_pretty(&stored)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    fs::write(lan_pending_pair_book_path(data_dir), text)
}

pub(crate) fn save_lan_discovered_devices_to_store(
    data_dir: impl AsRef<Path>,
    devices: &[LanDevice],
    encrypt_token: impl FnMut(&str) -> Option<String>,
    normalize_capabilities: impl FnMut(Vec<String>, u16) -> Vec<String>,
) -> io::Result<()> {
    let data_dir = data_dir.as_ref();
    fs::create_dir_all(data_dir)?;
    let stored =
        stored_book_from_lan_discovered_devices(devices, encrypt_token, normalize_capabilities);
    let text = serde_json::to_string_pretty(&stored)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    fs::write(lan_discovered_device_cache_path(data_dir), text)
}

pub(crate) fn merge_lan_discovered_devices(
    devices: &mut Vec<LanDevice>,
    new_device: LanDevice,
    now_ms: u64,
) {
    devices.retain(|device| now_ms.saturating_sub(device.last_seen_ms) < 5 * 60 * 1000);
    if let Some(existing) = devices
        .iter_mut()
        .find(|existing| existing.device_id == new_device.device_id)
    {
        *existing = new_device;
    } else {
        devices.push(new_device);
    }
    devices.sort_by(|a, b| {
        b.trusted
            .cmp(&a.trusted)
            .then_with(|| b.last_seen_ms.cmp(&a.last_seen_ms))
            .then_with(|| a.name.cmp(&b.name))
    });
}

pub(crate) fn merge_lan_device_book_and_discovery_cache(
    trusted_devices: Vec<LanDevice>,
    discovered_devices: Vec<LanDevice>,
) -> Vec<LanDevice> {
    let mut devices = Vec::new();
    for device in discovered_devices.into_iter().chain(trusted_devices) {
        if let Some(index) = devices
            .iter()
            .position(|existing: &LanDevice| existing.device_id == device.device_id)
        {
            if device.trusted || !devices[index].trusted {
                devices[index] = device;
            }
        } else {
            devices.push(device);
        }
    }
    devices.sort_by(|a, b| {
        b.trusted
            .cmp(&a.trusted)
            .then_with(|| b.last_seen_ms.cmp(&a.last_seen_ms))
            .then_with(|| a.name.cmp(&b.name))
    });
    devices
}

pub(crate) fn lan_background_clip_sync_plan(
    config: &LanRuntimeCoreConfig,
    trusted_devices: &[LanDevice],
    latest_clip: Option<&LanClipEnvelope>,
) -> LanBackgroundClipSyncPlan {
    let targets = trusted_devices
        .iter()
        .filter(|device| {
            device.trusted
                && device.tcp_port > 0
                && !device.addr.trim().is_empty()
                && device.device_id != config.device_id
        })
        .map(|device| {
            let can_pull_latest = device
                .capabilities
                .iter()
                .any(|capability| capability.eq_ignore_ascii_case("latest"));
            LanBackgroundClipSyncTarget {
                device_id: device.device_id.clone(),
                endpoint: format!("{}:{}", device.addr.trim(), device.tcp_port),
                can_push_clip: lan_device_can_receive_clip(device) && latest_clip.is_some(),
                can_pull_latest,
            }
        })
        .collect::<Vec<_>>();
    LanBackgroundClipSyncPlan {
        local_device_id: config.device_id.clone(),
        push_targets: targets
            .iter()
            .filter(|target| target.can_push_clip)
            .cloned()
            .collect(),
        pull_targets: targets
            .into_iter()
            .filter(|target| target.can_pull_latest)
            .collect(),
        has_latest_clip: latest_clip.is_some(),
    }
}

pub(crate) fn lan_clip_envelope_from_native_clip_item(
    device_id: &str,
    item: &crate::app_core::ClipItem,
    origin_seq: u64,
    created_at_ms: u64,
) -> Option<LanClipEnvelope> {
    let device_id = device_id.trim();
    if device_id.is_empty() {
        return None;
    }
    let preview = item.preview.chars().take(160).collect::<String>();
    let base = |kind: &str, hash: String| LanClipEnvelope {
        message_id: format!("{}-native-{}", device_id, item.id.max(0)),
        origin_device_id: device_id.to_string(),
        origin_seq,
        kind: kind.to_string(),
        hash,
        created_at_ms,
        preview: preview.clone(),
        text: None,
        image_png_base64: None,
        file_meta: Vec::new(),
    };
    match item.kind {
        crate::app_core::ClipKind::Text | crate::app_core::ClipKind::Phrase => {
            let text = item.text.clone()?;
            if text.trim().is_empty() {
                return None;
            }
            let mut envelope = base("text", lan_hash_string(&text));
            envelope.text = Some(text);
            Some(envelope)
        }
        crate::app_core::ClipKind::Image => {
            let png_bytes = lan_native_clip_item_png_bytes(item)?;
            if png_bytes.len() > LAN_IMAGE_MAX_BYTES {
                return None;
            }
            let mut envelope = base("image", format!("{:x}", md5::compute(&png_bytes)));
            envelope.image_png_base64 = Some(base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                png_bytes,
            ));
            Some(envelope)
        }
        crate::app_core::ClipKind::Files => None,
    }
}

fn lan_native_clip_item_png_bytes(item: &crate::app_core::ClipItem) -> Option<Vec<u8>> {
    if let Some(path) = item.image_path.as_deref() {
        let bytes = fs::read(path).ok()?;
        if bytes.len() <= LAN_IMAGE_MAX_BYTES && lan_png_dimensions_from_bytes(&bytes).is_some() {
            return Some(bytes);
        }
    }
    let bytes = item.image_bytes.as_ref()?;
    lan_encode_rgba_png_bytes(bytes, item.image_width as u32, item.image_height as u32)
}

fn lan_encode_rgba_png_bytes(bytes: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    if width == 0 || height == 0 || bytes.len() != width as usize * height as usize * 4 {
        return None;
    }
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(bytes).ok()?;
    }
    Some(out)
}

fn lan_png_dimensions_from_bytes(bytes: &[u8]) -> Option<(usize, usize)> {
    let cursor = std::io::Cursor::new(bytes);
    let decoder = png::Decoder::new(cursor);
    let reader = decoder.read_info().ok()?;
    let info = reader.info();
    Some((info.width as usize, info.height as usize))
}

pub(crate) fn execute_lan_background_clip_sync_once(
    config: &LanRuntimeConfig,
    trusted_devices: &[LanDevice],
    latest_clip: Option<LanClipEnvelope>,
    timeout: Duration,
) -> LanBackgroundClipSyncExecution {
    let plan =
        lan_background_clip_sync_plan(&config.core_config(), trusted_devices, latest_clip.as_ref());
    let mut pushed_count = 0usize;
    let mut pulled_count = 0usize;
    let mut failed_count = 0usize;
    let mut incoming_clips = Vec::new();

    if let Some(latest_clip) = latest_clip.as_ref() {
        let body = serde_json::to_vec(latest_clip).unwrap_or_default();
        for target in &plan.push_targets {
            let Some(device) = trusted_devices
                .iter()
                .find(|device| device.device_id == target.device_id)
            else {
                failed_count += 1;
                continue;
            };
            let result = http_request(
                "POST",
                &target.endpoint,
                "/v1/clip",
                &[
                    ("Content-Type", "application/json"),
                    ("X-ZSClip-Device", &config.device_id),
                    ("X-ZSClip-Token", &device.token),
                ],
                Some(&body),
                timeout,
            );
            if result.is_ok() {
                pushed_count += 1;
            } else {
                failed_count += 1;
            }
        }
    }

    for target in &plan.pull_targets {
        let Some(device) = trusted_devices
            .iter()
            .find(|device| device.device_id == target.device_id)
        else {
            failed_count += 1;
            continue;
        };
        let result = http_request(
            "GET",
            &target.endpoint,
            "/v1/latest",
            &[
                ("X-ZSClip-Device", &config.device_id),
                ("X-ZSClip-Token", &device.token),
            ],
            None,
            timeout,
        );
        let Ok(response) = result else {
            failed_count += 1;
            continue;
        };
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&response) else {
            failed_count += 1;
            continue;
        };
        let Some(clip_value) = value.get("clip").filter(|value| !value.is_null()) else {
            continue;
        };
        let Ok(envelope) = serde_json::from_value::<LanClipEnvelope>(clip_value.clone()) else {
            failed_count += 1;
            continue;
        };
        if envelope.origin_device_id == config.device_id {
            continue;
        }
        incoming_clips.push(LanIncomingClip {
            envelope,
            source_device_name: device.name.clone(),
        });
        pulled_count += 1;
    }

    LanBackgroundClipSyncExecution {
        plan,
        pushed_count,
        pulled_count,
        failed_count,
        incoming_clips,
    }
}

pub(crate) fn lan_file_payload_transfer_plan(
    config: &LanRuntimeCoreConfig,
    trusted_devices: &[LanDevice],
    file_paths: &[PathBuf],
    limit: u64,
) -> LanFilePayloadTransferPlan {
    let mut total_bytes = 0u64;
    let mut skipped_count = 0usize;
    let mut accepted_paths = Vec::new();
    for path in file_paths {
        let Ok(metadata) = fs::metadata(path) else {
            skipped_count += 1;
            continue;
        };
        if !path.is_file() || metadata.len() == 0 {
            skipped_count += 1;
            continue;
        }
        if metadata.len() > limit || total_bytes.saturating_add(metadata.len()) > limit {
            skipped_count += 1;
            continue;
        }
        total_bytes += metadata.len();
        accepted_paths.push(path.clone());
    }

    let target_device_ids = trusted_devices
        .iter()
        .filter(|device| {
            device.trusted
                && device.tcp_port > 0
                && !device.addr.trim().is_empty()
                && device.device_id != config.device_id
                && lan_device_can_receive_clip(device)
        })
        .map(|device| device.device_id.clone())
        .collect();

    LanFilePayloadTransferPlan {
        local_device_id: config.device_id.clone(),
        target_device_ids,
        file_paths: accepted_paths,
        total_bytes,
        skipped_count,
    }
}

pub(crate) fn execute_lan_file_payload_transfer_once(
    config: &LanRuntimeConfig,
    trusted_devices: &[LanDevice],
    file_paths: &[PathBuf],
    limit: u64,
    timeout: Duration,
) -> LanFilePayloadTransferExecution {
    let plan =
        lan_file_payload_transfer_plan(&config.core_config(), trusted_devices, file_paths, limit);
    let mut pushed_count = 0usize;
    let mut failed_count = 0usize;

    for device_id in &plan.target_device_ids {
        let Some(device) = trusted_devices
            .iter()
            .find(|device| device.device_id == *device_id)
        else {
            failed_count += plan.file_paths.len();
            continue;
        };
        for path in &plan.file_paths {
            if push_lan_file_payload_to_device(&config.device_id, device, path, timeout).is_ok() {
                pushed_count += 1;
            } else {
                failed_count += 1;
            }
        }
    }

    LanFilePayloadTransferExecution {
        plan,
        pushed_count,
        failed_count,
    }
}

pub(crate) fn push_lan_file_payload_to_device(
    sender_id: &str,
    device: &LanDevice,
    path: &Path,
    timeout: Duration,
) -> io::Result<()> {
    let metadata = fs::metadata(path)?;
    let total_size = metadata.len();
    if total_size == 0 || total_size > LAN_FILE_MAX_BYTES {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
    }
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("file.bin")
        .to_string();
    let transfer_id = make_lan_token(12, current_time_millis());
    let addr = format!("{}:{}", device.addr.trim(), device.tcp_port);
    let auth_headers = [
        ("Content-Type", "application/json"),
        ("X-ZSClip-Device", sender_id.trim()),
        ("X-ZSClip-Token", device.token.as_str()),
    ];
    let start = serde_json::json!({
        "transfer_id": &transfer_id,
        "file_name": &file_name,
        "total_size": total_size
    });
    let start_body = serde_json::to_vec(&start).unwrap_or_default();
    http_request(
        "POST",
        &addr,
        "/v1/file/start",
        &auth_headers,
        Some(&start_body),
        timeout,
    )?;

    let mut file = fs::File::open(path)?;
    let mut offset = 0u64;
    let mut buf = vec![0u8; LAN_FILE_CHUNK_BYTES];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        let chunk = serde_json::json!({
            "transfer_id": &transfer_id,
            "offset": offset,
            "data_base64": base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                &buf[..n],
            )
        });
        let body = serde_json::to_vec(&chunk).unwrap_or_default();
        http_request(
            "POST",
            &addr,
            "/v1/file/chunk",
            &auth_headers,
            Some(&body),
            timeout,
        )?;
        offset += n as u64;
    }

    let finish = serde_json::json!({"transfer_id": &transfer_id});
    let finish_body = serde_json::to_vec(&finish).unwrap_or_default();
    http_request(
        "POST",
        &addr,
        "/v1/file/finish",
        &auth_headers,
        Some(&finish_body),
        timeout,
    )?;
    Ok(())
}

pub(crate) fn upsert_lan_discovered_device_in_store(
    data_dir: impl AsRef<Path>,
    new_device: LanDevice,
    now_ms: u64,
    decrypt_token: impl FnMut(&str) -> Option<String>,
    encrypt_token: impl FnMut(&str) -> Option<String>,
    normalize_capabilities: impl FnMut(Vec<String>, u16) -> Vec<String> + Copy,
) -> io::Result<Vec<LanDevice>> {
    let data_dir = data_dir.as_ref();
    let mut devices = load_lan_discovered_devices_from_store(
        lan_discovered_device_cache_path(data_dir),
        decrypt_token,
        normalize_capabilities,
    );
    merge_lan_discovered_devices(&mut devices, new_device, now_ms);
    save_lan_discovered_devices_to_store(
        data_dir,
        &devices,
        encrypt_token,
        normalize_capabilities,
    )?;
    Ok(devices)
}

pub(crate) fn lan_discovered_device_from_packet(
    packet: DiscoveryPacket,
    peer: SocketAddr,
    local_device_id: &str,
    now_ms: u64,
    trusted_devices: &[LanDevice],
    mut normalize_capabilities: impl FnMut(Vec<String>, u16) -> Vec<String>,
) -> Option<LanDevice> {
    if !packet.uses_current_protocol() || packet.device_id == local_device_id {
        return None;
    }
    let mut device = LanDevice {
        device_id: packet.device_id,
        name: packet.name,
        addr: peer.ip().to_string(),
        tcp_port: packet.tcp_port,
        token: String::new(),
        last_seen_ms: now_ms,
        trusted: false,
        capabilities: normalize_capabilities(packet.capabilities, packet.tcp_port),
    };
    if let Some(trusted) = trusted_devices
        .iter()
        .find(|saved| saved.device_id == device.device_id && saved.trusted)
    {
        device.token = trusted.token.clone();
        device.trusted = true;
    }
    Some(device)
}

pub(crate) fn probe_lan_discovery_once(
    config: &LanRuntimeConfig,
    timeout: Duration,
) -> io::Result<Vec<LanDevice>> {
    if !config.lan_enabled || config.device_id.trim().is_empty() {
        return Ok(Vec::new());
    }

    let listener = UdpSocket::bind(("0.0.0.0", config.udp_port))?;
    listener.set_read_timeout(Some(timeout.min(Duration::from_millis(800))))?;
    let sender = UdpSocket::bind("0.0.0.0:0")?;
    sender.set_broadcast(true)?;
    let packet = DiscoveryPacket::new(&config.core_config(), lan_desktop_capabilities());
    let body = serde_json::to_vec(&packet)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    let _ = sender.send_to(
        &body,
        format!("255.255.255.255:{}", config.udp_port).as_str(),
    );

    let trusted_devices = config
        .platform
        .load_devices(normalize_lan_capabilities)
        .into_iter()
        .filter(|device| device.trusted)
        .collect::<Vec<_>>();
    let mut discovered = Vec::new();
    let mut buf = [0u8; 4096];
    let started = Instant::now();
    while started.elapsed() < timeout {
        let Ok((len, peer)) = listener.recv_from(&mut buf) else {
            break;
        };
        let Ok(packet) = serde_json::from_slice::<DiscoveryPacket>(&buf[..len]) else {
            continue;
        };
        let Some(device) = lan_discovered_device_from_packet(
            packet,
            peer,
            &config.device_id,
            current_time_millis(),
            &trusted_devices,
            normalize_lan_capabilities,
        ) else {
            continue;
        };
        let cache = upsert_lan_discovered_device_in_store(
            &config.platform.data_dir,
            device.clone(),
            current_time_millis(),
            config.platform.decrypt_secret,
            config.platform.encrypt_secret,
            normalize_lan_capabilities,
        )?;
        discovered = cache;
    }
    Ok(discovered)
}

pub(crate) fn upsert_lan_device_in_store(
    data_dir: impl AsRef<Path>,
    new_device: LanDevice,
    decrypt_token: impl FnMut(&str) -> Option<String>,
    encrypt_token: impl FnMut(&str) -> Option<String>,
    normalize_capabilities: impl FnMut(Vec<String>, u16) -> Vec<String> + Copy,
) -> io::Result<()> {
    let data_dir = data_dir.as_ref();
    let mut devices = load_lan_devices_from_store(
        lan_device_book_path(data_dir),
        decrypt_token,
        normalize_capabilities,
    );
    devices.retain(|device| device.device_id != new_device.device_id);
    devices.push(new_device);
    save_lan_devices_to_store(data_dir, &devices, encrypt_token, normalize_capabilities)
}

pub(crate) fn apply_lan_pending_pair_decision_in_store(
    data_dir: impl AsRef<Path>,
    pair_id: Option<&str>,
    accept: bool,
    now_ms: u64,
    decrypt_token: impl FnMut(&str) -> Option<String> + Copy,
    encrypt_token: impl FnMut(&str) -> Option<String> + Copy,
    normalize_capabilities: impl FnMut(Vec<String>, u16) -> Vec<String> + Copy,
) -> io::Result<Option<LanPendingPairDecision>> {
    let data_dir = data_dir.as_ref();
    let mut pairs = load_lan_pending_pairs_from_store(
        lan_pending_pair_book_path(data_dir),
        decrypt_token,
        normalize_capabilities,
    );
    let pair_index = pairs.iter().position(|pair| {
        pair.is_active(now_ms) && pair_id.map_or(true, |id| pair.prompt.pair_id == id)
    });
    let Some(pair_index) = pair_index else {
        save_lan_pending_pairs_to_store(data_dir, &pairs, encrypt_token, normalize_capabilities)?;
        return Ok(None);
    };
    let decision_pair_id = pairs[pair_index].prompt.pair_id.clone();
    let trusted_device = if accept {
        pairs[pair_index].mark_accepted();
        Some(pairs[pair_index].to_trusted_device(now_ms))
    } else {
        pairs[pair_index].mark_rejected();
        None
    };
    save_lan_pending_pairs_to_store(data_dir, &pairs, encrypt_token, normalize_capabilities)?;
    if let Some(device) = trusted_device.clone() {
        upsert_lan_device_in_store(
            data_dir,
            device,
            decrypt_token,
            encrypt_token,
            normalize_capabilities,
        )?;
    }
    Ok(Some(LanPendingPairDecision {
        pair_id: decision_pair_id,
        accepted: accept,
        trusted_device,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lan_clip_envelope_json_round_trips_without_host_runtime() {
        let envelope = LanClipEnvelope {
            message_id: "message-1".to_string(),
            origin_device_id: "device-1".to_string(),
            origin_seq: 7,
            kind: "file".to_string(),
            hash: "sha256:test".to_string(),
            created_at_ms: 1234,
            preview: "report.pdf".to_string(),
            text: None,
            image_png_base64: None,
            file_meta: vec![LanFileMeta {
                name: "report.pdf".to_string(),
                size: 42,
                relative_path: "docs/report.pdf".to_string(),
            }],
        };

        let encoded = serde_json::to_string(&envelope).expect("LAN envelope serializes");
        let decoded: LanClipEnvelope =
            serde_json::from_str(&encoded).expect("LAN envelope deserializes");

        assert_eq!(decoded, envelope);
        assert_eq!(LAN_MAGIC, "ZSCLIP_LAN_V1");
        assert_eq!(LAN_PROTOCOL, 1);
        assert_eq!(LAN_TCP_PORT_DEFAULT, 38473);
        assert!(LAN_IMAGE_MAX_BYTES < HTTP_MAX_BODY);
    }

    #[test]
    fn tcp_bind_candidates_are_shared_runtime_policy() {
        let loopback = lan_tcp_bind_candidates(LAN_TCP_PORT_DEFAULT, true).collect::<Vec<_>>();
        assert_eq!(loopback.len(), LAN_TCP_BIND_CANDIDATE_COUNT as usize);
        assert_eq!(
            loopback.first().copied(),
            Some(SocketAddr::from(([127, 0, 0, 1], LAN_TCP_PORT_DEFAULT)))
        );
        assert_eq!(
            loopback.last().copied(),
            Some(SocketAddr::from((
                [127, 0, 0, 1],
                LAN_TCP_PORT_DEFAULT + LAN_TCP_BIND_CANDIDATE_COUNT - 1
            )))
        );
        assert!(loopback.iter().all(|addr| addr.ip().is_loopback()));

        let lan = lan_tcp_bind_candidates(LAN_TCP_PORT_DEFAULT, false).collect::<Vec<_>>();
        assert_eq!(
            lan.first().copied(),
            Some(SocketAddr::from(([0, 0, 0, 0], LAN_TCP_PORT_DEFAULT)))
        );
        assert!(lan.iter().all(|addr| addr.ip().is_unspecified()));

        let high_ports = lan_tcp_bind_candidates(u16::MAX - 2, false).collect::<Vec<_>>();
        assert_eq!(high_ports.len(), 3);
        assert_eq!(high_ports[2].port(), u16::MAX);
    }

    #[test]
    fn lan_device_missing_capabilities_defaults_to_empty_vec() {
        let decoded: LanDevice = serde_json::from_str(
            r#"{
                "device_id":"device-1",
                "name":"Phone",
                "addr":"127.0.0.1",
                "tcp_port":38473,
                "token":"token",
                "last_seen_ms":1,
                "trusted":true
            }"#,
        )
        .expect("legacy LAN device without capabilities deserializes");

        assert!(decoded.capabilities.is_empty());
    }

    #[test]
    fn discovery_packet_is_built_from_core_runtime_config() {
        let config = LanRuntimeCoreConfig {
            device_id: "device-1".to_string(),
            device_name: "Desktop".to_string(),
            tcp_port: LAN_TCP_PORT_DEFAULT,
            udp_port: LAN_DISCOVERY_PORT_DEFAULT,
            lan_enabled: true,
            wps_taskpane_enabled: false,
        };

        let packet = DiscoveryPacket::new(
            &config,
            vec!["text".to_string(), "receive_clip".to_string()],
        );
        let encoded = serde_json::to_string(&packet).expect("discovery packet serializes");
        let decoded: DiscoveryPacket =
            serde_json::from_str(&encoded).expect("discovery packet deserializes");

        assert_eq!(decoded, packet);
        assert!(decoded.uses_current_protocol());
        assert_eq!(decoded.device_id, "device-1");
        assert_eq!(decoded.tcp_port, LAN_TCP_PORT_DEFAULT);
    }

    #[test]
    fn runtime_settings_parse_lan_fields_from_settings_json() {
        let settings = lan_runtime_settings_from_settings_json(&serde_json::json!({
            "lan_sync_enabled": true,
            "wps_taskpane_enabled": true,
            "lan_device_id": " desktop-1 ",
            "lan_device_name": " Workstation ",
            "lan_tcp_port": "38480",
            "lan_udp_port": 38481
        }));

        assert!(settings.lan_sync_enabled);
        assert!(settings.wps_taskpane_enabled);
        assert_eq!(settings.device_id, "desktop-1");
        assert_eq!(settings.device_name, "Workstation");
        assert_eq!(settings.tcp_port, 38480);
        assert_eq!(settings.udp_port, 38481);
        assert_eq!(settings.normalized_tcp_port(), 38480);
        assert_eq!(settings.normalized_udp_port(), 38481);
    }

    #[test]
    fn discovery_packet_merges_into_discovered_device_cache() {
        let data_dir = std::env::temp_dir().join(format!(
            "zsclip-lan-discovered-cache-{}",
            LAN_TOKEN_COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        let trusted = LanDevice {
            device_id: "phone".to_string(),
            name: "Trusted Phone".to_string(),
            addr: "192.168.1.8".to_string(),
            tcp_port: LAN_TCP_PORT_DEFAULT,
            token: "trusted-token".to_string(),
            last_seen_ms: 1,
            trusted: true,
            capabilities: vec!["receive_clip".to_string(), "text".to_string()],
        };
        let packet = DiscoveryPacket {
            magic: LAN_MAGIC.to_string(),
            protocol: LAN_PROTOCOL,
            device_id: "phone".to_string(),
            name: "Phone Broadcast".to_string(),
            tcp_port: LAN_TCP_PORT_DEFAULT,
            capabilities: vec!["text".to_string()],
        };
        let device = lan_discovered_device_from_packet(
            packet,
            SocketAddr::from(([192, 168, 1, 20], LAN_DISCOVERY_PORT_DEFAULT)),
            "desktop",
            100,
            &[trusted],
            normalize_lan_capabilities,
        )
        .expect("trusted remote packet should become a discovered device");

        assert!(device.trusted);
        assert_eq!(device.token, "trusted-token");
        assert_eq!(device.addr, "192.168.1.20");

        let cache = upsert_lan_discovered_device_in_store(
            &data_dir,
            device,
            100,
            |encrypted| encrypted.strip_prefix("enc:").map(str::to_string),
            |token| Some(format!("enc:{token}")),
            normalize_lan_capabilities,
        )
        .expect("discovered cache should save");
        assert_eq!(cache.len(), 1);

        let loaded = load_lan_discovered_devices_from_store(
            lan_discovered_device_cache_path(&data_dir),
            |encrypted| encrypted.strip_prefix("enc:").map(str::to_string),
            normalize_lan_capabilities,
        );
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].device_id, "phone");
        assert_eq!(loaded[0].token, "trusted-token");
        assert_eq!(
            lan_discovered_device_cache_path(&data_dir),
            data_dir.join("lan_discovered_devices.json")
        );
        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn background_clip_sync_plan_selects_push_and_pull_targets() {
        let config = LanRuntimeCoreConfig {
            device_id: "desktop".to_string(),
            device_name: "Desktop".to_string(),
            tcp_port: LAN_TCP_PORT_DEFAULT,
            udp_port: LAN_DISCOVERY_PORT_DEFAULT,
            lan_enabled: true,
            wps_taskpane_enabled: false,
        };
        let latest = LanClipEnvelope {
            message_id: "desktop-1".to_string(),
            origin_device_id: "desktop".to_string(),
            origin_seq: 1,
            kind: "text".to_string(),
            hash: "hash".to_string(),
            created_at_ms: 1,
            preview: "hello".to_string(),
            text: Some("hello".to_string()),
            image_png_base64: None,
            file_meta: Vec::new(),
        };
        let devices = vec![
            LanDevice {
                device_id: "phone".to_string(),
                name: "Phone".to_string(),
                addr: "192.168.1.20".to_string(),
                tcp_port: LAN_TCP_PORT_DEFAULT,
                token: "tok".to_string(),
                last_seen_ms: 1,
                trusted: true,
                capabilities: vec!["latest".to_string(), "receive_clip".to_string()],
            },
            LanDevice {
                device_id: "watch".to_string(),
                name: "Watch".to_string(),
                addr: "192.168.1.21".to_string(),
                tcp_port: 0,
                token: "watch".to_string(),
                last_seen_ms: 1,
                trusted: true,
                capabilities: vec!["client_only".to_string(), "pull_only".to_string()],
            },
            LanDevice {
                device_id: "unknown".to_string(),
                name: "Unknown".to_string(),
                addr: "192.168.1.22".to_string(),
                tcp_port: LAN_TCP_PORT_DEFAULT,
                token: String::new(),
                last_seen_ms: 1,
                trusted: false,
                capabilities: vec!["latest".to_string(), "receive_clip".to_string()],
            },
        ];

        let plan = lan_background_clip_sync_plan(&config, &devices, Some(&latest));

        assert!(plan.has_latest_clip);
        assert_eq!(plan.local_device_id, "desktop");
        assert_eq!(plan.push_targets.len(), 1);
        assert_eq!(plan.push_targets[0].device_id, "phone");
        assert_eq!(plan.pull_targets.len(), 1);
        assert_eq!(plan.pull_targets[0].device_id, "phone");
    }

    #[test]
    fn file_payload_transfer_plan_selects_small_files_and_trusted_receivers() {
        let dir = std::env::temp_dir().join(format!(
            "zsclip-lan-file-transfer-plan-{}",
            LAN_TOKEN_COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        let good_file = dir.join("report.txt");
        let empty_file = dir.join("empty.txt");
        fs::write(&good_file, b"ok").unwrap();
        fs::write(&empty_file, b"").unwrap();
        let config = LanRuntimeCoreConfig {
            device_id: "desktop".to_string(),
            device_name: "Desktop".to_string(),
            tcp_port: LAN_TCP_PORT_DEFAULT,
            udp_port: LAN_DISCOVERY_PORT_DEFAULT,
            lan_enabled: true,
            wps_taskpane_enabled: false,
        };
        let devices = vec![
            LanDevice {
                device_id: "phone".to_string(),
                name: "Phone".to_string(),
                addr: "192.168.1.20".to_string(),
                tcp_port: LAN_TCP_PORT_DEFAULT,
                token: "tok".to_string(),
                last_seen_ms: 1,
                trusted: true,
                capabilities: vec!["receive_clip".to_string()],
            },
            LanDevice {
                device_id: "desktop".to_string(),
                name: "Self".to_string(),
                addr: "127.0.0.1".to_string(),
                tcp_port: LAN_TCP_PORT_DEFAULT,
                token: "self".to_string(),
                last_seen_ms: 1,
                trusted: true,
                capabilities: vec!["receive_clip".to_string()],
            },
            LanDevice {
                device_id: "unknown".to_string(),
                name: "Unknown".to_string(),
                addr: "192.168.1.22".to_string(),
                tcp_port: LAN_TCP_PORT_DEFAULT,
                token: String::new(),
                last_seen_ms: 1,
                trusted: false,
                capabilities: vec!["receive_clip".to_string()],
            },
        ];

        let plan = lan_file_payload_transfer_plan(
            &config,
            &devices,
            &[good_file.clone(), empty_file, dir.join("missing.txt")],
            LAN_FILE_AUTO_MAX_BYTES,
        );

        assert_eq!(plan.local_device_id, "desktop");
        assert_eq!(plan.target_device_ids, vec!["phone"]);
        assert_eq!(plan.file_paths, vec![good_file]);
        assert_eq!(plan.total_bytes, 2);
        assert_eq!(plan.skipped_count, 2);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn native_clip_item_envelope_supports_text_and_image_but_keeps_files_out_of_inline_payload() {
        let text = crate::app_core::ClipItem {
            id: 7,
            kind: crate::app_core::ClipKind::Text,
            preview: "hello".to_string(),
            text: Some("hello".to_string()),
            source_app: "test".to_string(),
            file_paths: None,
            image_bytes: None,
            image_path: None,
            image_width: 0,
            image_height: 0,
            pinned: false,
            group_id: 0,
            created_at: String::new(),
        };
        let text_envelope =
            lan_clip_envelope_from_native_clip_item("desktop", &text, 99, 1234).unwrap();
        assert_eq!(text_envelope.kind, "text");
        assert_eq!(text_envelope.text.as_deref(), Some("hello"));
        assert_eq!(text_envelope.origin_seq, 99);
        assert_eq!(text_envelope.created_at_ms, 1234);

        let mut image = text.clone();
        image.id = 8;
        image.kind = crate::app_core::ClipKind::Image;
        image.text = None;
        image.image_bytes = Some(vec![255, 0, 0, 255]);
        image.image_width = 1;
        image.image_height = 1;
        let image_envelope =
            lan_clip_envelope_from_native_clip_item("desktop", &image, 100, 1235).unwrap();
        assert_eq!(image_envelope.kind, "image");
        assert!(image_envelope.image_png_base64.is_some());
        assert!(image_envelope.text.is_none());

        let mut files = text.clone();
        files.id = 9;
        files.kind = crate::app_core::ClipKind::Files;
        files.text = None;
        files.file_paths = Some(vec!["/tmp/report.txt".to_string()]);
        assert!(lan_clip_envelope_from_native_clip_item("desktop", &files, 101, 1236).is_none());
    }

    #[test]
    fn runtime_event_sink_keeps_zero_handle_as_noop() {
        assert_eq!(
            LanRuntimeEventSink::platform_main_window(0),
            LanRuntimeEventSink::None
        );
        assert_eq!(
            LanRuntimeEventSink::platform_main_window(42).raw_platform_main_window_handle(),
            Some(42)
        );
    }

    #[test]
    fn platform_context_derives_device_book_path_and_uses_codecs() {
        fn enc(value: &str) -> Option<String> {
            Some(format!("enc:{value}"))
        }
        fn dec(value: &str) -> Option<String> {
            value.strip_prefix("enc:").map(|secret| secret.to_string())
        }

        let context = LanRuntimePlatformContext::new(
            PathBuf::from("zsclip-data"),
            LanRuntimeEventSink::platform_main_window(7),
            enc,
            dec,
        );

        assert_eq!(
            context.device_book_path(),
            PathBuf::from("zsclip-data").join("lan_devices.json")
        );
        assert_eq!(
            context.event_sink.raw_platform_main_window_handle(),
            Some(7)
        );
        assert_eq!(
            (context.encrypt_secret)("token"),
            Some("enc:token".to_string())
        );
        assert_eq!(
            (context.decrypt_secret)("enc:token"),
            Some("token".to_string())
        );
    }

    #[test]
    fn runtime_config_combines_platform_context_with_core_config() {
        fn enc(value: &str) -> Option<String> {
            Some(format!("enc:{value}"))
        }
        fn dec(value: &str) -> Option<String> {
            value.strip_prefix("enc:").map(|secret| secret.to_string())
        }

        let platform = LanRuntimePlatformContext::new(
            PathBuf::from("zsclip-data"),
            LanRuntimeEventSink::platform_main_window(99),
            enc,
            dec,
        );
        let core_config = LanRuntimeCoreConfig {
            device_id: "device-1".to_string(),
            device_name: "Desktop".to_string(),
            tcp_port: LAN_TCP_PORT_DEFAULT,
            udp_port: LAN_DISCOVERY_PORT_DEFAULT,
            lan_enabled: true,
            wps_taskpane_enabled: true,
        };

        let runtime = LanRuntimeConfig::from_core_config(platform, core_config.clone());

        assert_eq!(runtime.core_config(), core_config);
        assert_eq!(
            runtime
                .platform
                .event_sink
                .raw_platform_main_window_handle(),
            Some(99)
        );
        assert_eq!(
            runtime.platform.device_book_path(),
            PathBuf::from("zsclip-data").join("lan_devices.json")
        );
    }

    #[test]
    fn runtime_settings_normalize_ports_and_build_pair_request() {
        let settings = LanRuntimeSettings {
            lan_sync_enabled: true,
            wps_taskpane_enabled: false,
            device_id: " device-1 ".to_string(),
            device_name: "Desktop".to_string(),
            tcp_port: 0,
            udp_port: 0,
        };

        let core_config = settings.core_config();
        assert!(settings.runtime_enabled());
        assert_eq!(core_config.device_id, "device-1");
        assert_eq!(core_config.tcp_port, 1);
        assert_eq!(core_config.udp_port, 1);

        let body = settings.pair_request_body(vec!["text".to_string()]);
        assert_eq!(body.device_id, " device-1 ");
        assert_eq!(body.tcp_port, 1);
        assert_eq!(body.capabilities, vec!["text"]);
    }

    #[test]
    fn service_runtime_state_detects_restart_relevant_changes() {
        let config = LanRuntimeCoreConfig {
            device_id: "device-1".to_string(),
            device_name: "Desktop".to_string(),
            tcp_port: LAN_TCP_PORT_DEFAULT,
            udp_port: LAN_DISCOVERY_PORT_DEFAULT,
            lan_enabled: true,
            wps_taskpane_enabled: false,
        };
        let state = LanServiceRuntimeState::from_core_config(&config, LAN_TCP_PORT_DEFAULT);

        assert!(!state.requires_restart_for(&config));

        let mut changed = config.clone();
        changed.wps_taskpane_enabled = true;
        assert!(state.requires_restart_for(&changed));
    }

    #[test]
    fn lifecycle_plan_keeps_wps_only_server_loopback_only() {
        let config = LanRuntimeCoreConfig {
            device_id: String::new(),
            device_name: "Desktop".to_string(),
            tcp_port: LAN_TCP_PORT_DEFAULT,
            udp_port: LAN_DISCOVERY_PORT_DEFAULT,
            lan_enabled: false,
            wps_taskpane_enabled: true,
        };

        let plan = LanServiceLifecyclePlan::for_config(&config);

        assert!(plan.bind_loopback_only);
        assert!(plan.start_tcp_server);
        assert!(!plan.start_udp_listener);
        assert!(!plan.start_udp_sender);
        assert!(!plan.wake_udp_on_stop);
    }

    #[test]
    fn lifecycle_plan_enables_udp_for_lan_service() {
        let state = LanServiceRuntimeState {
            device_id: "device-1".to_string(),
            tcp_port: LAN_TCP_PORT_DEFAULT,
            udp_port: LAN_DISCOVERY_PORT_DEFAULT,
            lan_enabled: true,
            wps_taskpane_enabled: false,
        };

        let plan = LanServiceLifecyclePlan::for_state(&state);

        assert!(!plan.bind_loopback_only);
        assert!(plan.start_tcp_server);
        assert!(plan.start_udp_listener);
        assert!(plan.start_udp_sender);
        assert!(plan.wake_udp_on_stop);
    }

    #[test]
    fn http_route_policy_keeps_wps_routes_independent_from_lan_routes() {
        let mut config = LanRuntimeCoreConfig {
            device_id: String::new(),
            device_name: "ZSClip".to_string(),
            tcp_port: LAN_TCP_PORT_DEFAULT,
            udp_port: LAN_DISCOVERY_PORT_DEFAULT,
            lan_enabled: false,
            wps_taskpane_enabled: true,
        };

        let policy = LanHttpRoutePolicy::for_config(&config);
        assert!(policy.route_available("GET", "/office/wps/taskpane"));
        assert!(policy.route_available("GET", "/office/wps/items"));
        assert!(policy.route_available("GET", "/office/wps/image"));
        assert!(policy.route_available("GET", "/office/wps/events"));
        assert!(!policy.route_available("GET", "/v1/info"));

        config.wps_taskpane_enabled = false;
        config.lan_enabled = true;
        let policy = LanHttpRoutePolicy::for_config(&config);
        assert!(!policy.route_available("GET", "/office/wps/taskpane"));
        assert!(policy.route_available("GET", "/v1/info"));
        assert!(is_wps_taskpane_route("GET", "/office/wps/events"));
        assert!(!is_wps_taskpane_route("POST", "/office/wps/events"));
    }

    #[test]
    fn http_route_classifier_covers_static_and_dynamic_lan_routes() {
        assert_eq!(lan_http_route_for("GET", "/v1/info"), LanHttpRoute::Info);
        assert_eq!(
            lan_http_route_for("POST", "/v1/pair/request"),
            LanHttpRoute::PairRequest
        );
        assert_eq!(
            lan_http_route_for("GET", "/office/wps/taskpane"),
            LanHttpRoute::WpsTaskpane
        );
        assert_eq!(
            lan_http_route_for("GET", "/v1/mobile/items/42/image"),
            LanHttpRoute::MobileItemImage
        );
        assert_eq!(
            lan_http_route_for("GET", "/v1/mobile/items/42/file/3"),
            LanHttpRoute::MobileItemFile
        );
        assert_eq!(
            lan_http_route_for("GET", "/zsSyncClipboard.json"),
            LanHttpRoute::MultiSyncManifest
        );
        assert_eq!(
            lan_http_route_for("GET", "/SyncClipboard.json"),
            LanHttpRoute::MultiSyncManifest
        );
        assert_eq!(
            lan_http_route_for("GET", "/file/docs/report.pdf"),
            LanHttpRoute::MultiSyncFile
        );
        assert_eq!(
            lan_http_route_for("GET", "/v1/mobile/items/0/image"),
            LanHttpRoute::NotFound
        );
        assert_eq!(
            lan_http_route_for("POST", "/office/wps/taskpane"),
            LanHttpRoute::NotFound
        );
    }

    #[test]
    fn http_route_policy_can_gate_preclassified_routes() {
        let policy = LanHttpRoutePolicy {
            lan_enabled: false,
            wps_taskpane_enabled: true,
        };

        assert!(policy.route_available_for_route(LanHttpRoute::WpsTaskpane));
        assert!(!policy.route_available_for_route(LanHttpRoute::Info));

        let policy = LanHttpRoutePolicy {
            lan_enabled: true,
            wps_taskpane_enabled: false,
        };

        assert!(policy.route_available_for_route(LanHttpRoute::Info));
        assert!(policy.route_available_for_route(LanHttpRoute::NotFound));
        assert!(!policy.route_available_for_route(LanHttpRoute::WpsTaskpane));
    }

    #[test]
    fn lan_capabilities_normalize_and_fallback_by_endpoint_kind() {
        assert_eq!(
            normalize_lan_capabilities(
                vec![
                    " TEXT ".to_string(),
                    "receive_clip".to_string(),
                    "text".to_string(),
                    "".to_string(),
                ],
                LAN_TCP_PORT_DEFAULT,
            ),
            vec!["receive_clip".to_string(), "text".to_string()]
        );
        assert_eq!(
            normalize_lan_capabilities(Vec::new(), 0),
            vec!["client_only".to_string(), "pull_only".to_string()]
        );
        assert!(normalize_lan_capabilities(Vec::new(), LAN_TCP_PORT_DEFAULT)
            .iter()
            .any(|cap| cap == "receive_clip"));
    }

    #[test]
    fn lan_device_receive_clip_policy_skips_pull_only_clients() {
        let mut device = LanDevice {
            device_id: "device-1".to_string(),
            name: "Device".to_string(),
            addr: "127.0.0.1".to_string(),
            tcp_port: LAN_TCP_PORT_DEFAULT,
            token: "token".to_string(),
            last_seen_ms: 0,
            trusted: true,
            capabilities: vec!["text".to_string(), "receive_clip".to_string()],
        };

        assert!(lan_device_can_receive_clip(&device));

        device.capabilities = vec!["pull_only".to_string()];
        assert!(!lan_device_can_receive_clip(&device));

        device.capabilities = lan_desktop_capabilities();
        device.trusted = false;
        assert!(!lan_device_can_receive_clip(&device));

        device.trusted = true;
        device.tcp_port = 0;
        assert!(!lan_device_can_receive_clip(&device));
    }

    #[test]
    fn http_request_model_exposes_shared_header_path_and_query_helpers() {
        let request = HttpRequest {
            method: "GET".to_string(),
            path: "/mobile/setup?device=ios-%E6%89%8B%E6%9C%BA&token=a%2Bb".to_string(),
            headers: vec![("Host".to_string(), "127.0.0.1:38473".to_string())],
            body: Vec::new(),
            peer: "127.0.0.1:50000".parse().expect("valid socket addr"),
        };

        assert_eq!(request.path_without_query(), "/mobile/setup");
        assert_eq!(request.header_value("host"), Some("127.0.0.1:38473"));
        assert_eq!(request.query_param("token"), "a%2Bb");
        assert_eq!(request.query_param_decoded("device"), "ios-手机");
        assert_eq!(request.query_param_decoded("token"), "a+b");
    }

    #[test]
    fn http_request_parser_reads_headers_and_body_from_shared_transport() {
        let raw = b"POST /v1/clip?device=desk HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 11\r\n\r\nhello world";
        let mut reader = std::io::Cursor::new(raw.as_slice());
        let request = read_http_request(
            &mut reader,
            "127.0.0.1:50000".parse().expect("valid socket addr"),
        )
        .expect("request parses");

        assert_eq!(request.method, "POST");
        assert_eq!(request.path_without_query(), "/v1/clip");
        assert_eq!(request.query_param("device"), "desk");
        assert_eq!(request.header_value("host"), Some("127.0.0.1"));
        assert_eq!(request.body, b"hello world");
    }

    #[test]
    fn http_request_parser_rejects_missing_header_terminator() {
        let raw = b"GET /v1/info HTTP/1.1\r\nHost: 127.0.0.1";
        let mut reader = std::io::Cursor::new(raw.as_slice());
        let result = read_http_request(
            &mut reader,
            "127.0.0.1:50000".parse().expect("valid socket addr"),
        );

        assert!(result.is_err());
        assert_eq!(find_header_end(b"GET / HTTP/1.1\r\n\r\nbody"), Some(14));
        assert_eq!(find_header_end(raw), None);
    }

    #[test]
    fn http_response_writers_emit_headers_and_body_without_platform_runtime() {
        let mut json_response = Vec::new();
        write_http_json(
            &mut json_response,
            404,
            &serde_json::json!({"error":"missing"}),
        )
        .expect("json response writes");
        let json_text = String::from_utf8(json_response).expect("response is utf-8");

        assert!(json_text.starts_with("HTTP/1.1 404 Not Found\r\n"));
        assert!(json_text.contains("Content-Type: application/json; charset=utf-8\r\n"));
        assert!(json_text.ends_with(r#"{"error":"missing"}"#));

        let mut bytes_response = Vec::new();
        write_http_bytes(
            &mut bytes_response,
            200,
            "text/plain",
            b"hello",
            &[("X-Test", "ok".to_string())],
        )
        .expect("bytes response writes");
        let bytes_text = String::from_utf8(bytes_response).expect("response is utf-8");

        assert!(bytes_text.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(bytes_text.contains("Content-Type: text/plain\r\n"));
        assert!(bytes_text.contains("X-Test: ok\r\n"));
        assert!(bytes_text.ends_with("\r\n\r\nhello"));
    }

    #[test]
    fn http_file_response_streams_file_body() {
        let path = std::env::temp_dir().join("zsclip-lan-http-file-response.txt");
        fs::write(&path, b"file-body").expect("test file writes");

        let mut response = Vec::new();
        write_http_file(
            &mut response,
            200,
            "application/octet-stream",
            &path,
            9,
            &[("Content-Disposition", "attachment".to_string())],
        )
        .expect("file response writes");
        let text = String::from_utf8(response).expect("response is utf-8");

        assert!(text.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(text.contains("Content-Type: application/octet-stream\r\n"));
        assert!(text.contains("Content-Disposition: attachment\r\n"));
        assert!(text.ends_with("\r\n\r\nfile-body"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn http_client_request_sends_headers_and_returns_response_body() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener binds");
        let addr = listener.local_addr().expect("listener has local addr");
        let server = std::thread::spawn(move || {
            use std::io::{Read as _, Write as _};

            let (mut stream, _) = listener.accept().expect("server accepts one request");
            let mut buf = [0u8; 1024];
            let len = stream.read(&mut buf).expect("server reads request");
            let request = String::from_utf8_lossy(&buf[..len]).to_string();
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 7\r\nConnection: close\r\n\r\naccepted",
                )
                .expect("server writes response");
            request
        });

        let body = http_request(
            "POST",
            &addr.to_string(),
            "/v1/pair/request",
            &[("X-Test", "ok")],
            Some(b"payload"),
            Duration::from_secs(5),
        )
        .expect("client request succeeds");
        let request = server.join().expect("server thread joins");

        assert_eq!(body, b"accepted");
        assert!(request.starts_with("POST /v1/pair/request HTTP/1.1\r\n"));
        assert!(request.contains("Host: "));
        assert!(request.contains("X-Test: ok\r\n"));
        assert!(request.ends_with("\r\n\r\npayload"));
    }

    #[test]
    fn lan_host_and_url_helpers_normalize_mobile_pair_inputs() {
        assert_eq!(
            normalize_lan_host(" http://192.168.1.8:38473/mobile/setup ", 38473),
            "192.168.1.8:38473"
        );
        assert_eq!(
            normalize_lan_host("https://phone.local/path", 38473),
            "phone.local:38473"
        );
        assert_eq!(
            normalize_lan_host("192.168.1.9", 38473),
            "192.168.1.9:38473"
        );
        assert_eq!(normalize_lan_host("  ", 38473), "");
        assert_eq!(
            url_encode_component("192.168.1.8:38473"),
            "192.168.1.8%3A38473"
        );
        assert_eq!(url_encode_component("ios 手机"), "ios%20%E6%89%8B%E6%9C%BA");
    }

    #[test]
    fn lan_file_helpers_sanitize_names_and_scope_transfer_state() {
        assert_eq!(
            safe_lan_file_name(" report:final?.pdf "),
            "report_final_.pdf"
        );
        assert_eq!(safe_lan_file_name("../"), "_");
        assert_eq!(
            lan_file_session_key(" android-a ", " transfer-1 "),
            "android-a:transfer-1"
        );
        assert_ne!(
            lan_file_session_key("android-a", "transfer-1"),
            lan_file_session_key("android-b", "transfer-1")
        );

        let mut first = lan_file_content_hasher(3);
        first.update(b"abc");
        let mut second = lan_file_content_hasher(3);
        second.update(b"abc");
        assert_eq!(first.finalize(), second.finalize());
    }

    #[test]
    fn lan_pair_code_and_token_helpers_are_protocol_level() {
        let code = make_lan_pair_code("desktop-a", "phone-b", 120_000);
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|ch| ch.is_ascii_digit()));
        assert_eq!(code, make_lan_pair_code("desktop-a", "phone-b", 120_999));
        assert_ne!(code, make_lan_pair_code("desktop-a", "phone-b", 180_000));

        let token = make_lan_token(12, 123_456);
        assert_eq!(token.len(), 24);
        assert!(token.chars().all(|ch| ch.is_ascii_hexdigit()));
        assert_eq!(lan_hash_string("zsclip").len(), 32);
    }

    #[test]
    fn lan_pending_pair_projects_status_and_trusted_device() {
        let prompt = LanPairPrompt {
            pair_id: "pair-1".to_string(),
            code: "123456".to_string(),
            device_name: "Phone".to_string(),
            addr: "192.168.1.20".to_string(),
            created_at_ms: 1_000,
        };
        let mut pair = LanPendingPair {
            prompt,
            requester_device_id: "phone-1".to_string(),
            requester_tcp_port: LAN_TCP_PORT_DEFAULT,
            requester_capabilities: vec![" TEXT ".to_string(), "receive_clip".to_string()],
            token: "token".to_string(),
            accepted: false,
            rejected: false,
            created_at_ms: 1_000,
        };
        let config = LanRuntimeCoreConfig {
            device_id: "desktop-1".to_string(),
            device_name: "Desktop".to_string(),
            tcp_port: LAN_TCP_PORT_DEFAULT,
            udp_port: LAN_DISCOVERY_PORT_DEFAULT,
            lan_enabled: true,
            wps_taskpane_enabled: false,
        };

        assert!(pair.is_active(1_000 + LAN_PAIR_REQUEST_TTL_MS - 1));
        assert!(!pair.is_active(1_000 + LAN_PAIR_REQUEST_TTL_MS));
        assert_eq!(
            lan_pair_status_response_value(Some(&pair), &config, lan_desktop_capabilities())
                ["status"],
            "pending"
        );

        pair.mark_accepted();
        let device = pair.to_trusted_device(9_000);
        assert!(device.trusted);
        assert_eq!(device.device_id, "phone-1");
        assert_eq!(device.last_seen_ms, 9_000);
        assert_eq!(device.capabilities, vec!["receive_clip", "text"]);
        let accepted =
            lan_pair_status_response_value(Some(&pair), &config, lan_desktop_capabilities());
        assert_eq!(accepted["status"], "accepted");
        assert_eq!(accepted["device_id"], "desktop-1");
        assert_eq!(accepted["token"], "token");

        pair.mark_rejected();
        let rejected =
            lan_pair_status_response_value(Some(&pair), &config, lan_desktop_capabilities());
        assert_eq!(rejected["status"], "rejected");
        assert_eq!(
            lan_pair_status_response_value(None, &config, Vec::new())["status"],
            "missing"
        );
    }

    #[test]
    fn lan_seen_message_set_rejects_duplicates_and_prunes_when_large() {
        let mut seen = HashSet::new();

        assert!(remember_lan_seen_message_key(
            &mut seen,
            "message-1".to_string()
        ));
        assert!(!remember_lan_seen_message_key(
            &mut seen,
            "message-1".to_string()
        ));

        for index in 0..=LAN_SEEN_MESSAGE_MAX {
            seen.insert(format!("old-{index}"));
        }
        assert!(seen.len() > LAN_SEEN_MESSAGE_MAX);
        assert!(remember_lan_seen_message_key(
            &mut seen,
            "fresh".to_string()
        ));
        assert_eq!(seen.len(), 1);
        assert!(seen.contains("fresh"));
    }

    #[test]
    fn http_query_decode_supports_android_plus_spaces() {
        let path = "/mobile/images?device=android+phone&token=tok%2Ben";

        assert_eq!(http_path_without_query(path), "/mobile/images");
        assert_eq!(query_param_decoded(path, "device"), "android phone");
        assert_eq!(query_param_decoded(path, "token"), "tok+en");
        assert_eq!(percent_decode("bad%zz"), "bad%zz");
    }

    #[test]
    fn pair_request_body_missing_capabilities_defaults_to_empty_vec() {
        let decoded: PairRequestBody = serde_json::from_str(
            r#"{
                "device_id":"phone-1",
                "name":"Phone",
                "tcp_port":38473
            }"#,
        )
        .expect("legacy pair request without capabilities deserializes");

        assert!(decoded.capabilities.is_empty());
        assert_eq!(decoded.device_id, "phone-1");
    }

    #[test]
    fn stored_device_book_round_trips_trusted_devices_through_secret_codec() {
        let devices = vec![
            LanDevice {
                device_id: "trusted".to_string(),
                name: "Trusted".to_string(),
                addr: "192.168.1.10".to_string(),
                tcp_port: LAN_TCP_PORT_DEFAULT,
                token: "secret".to_string(),
                last_seen_ms: 123,
                trusted: true,
                capabilities: vec!["text".to_string()],
            },
            LanDevice {
                device_id: "untrusted".to_string(),
                name: "Untrusted".to_string(),
                addr: "192.168.1.11".to_string(),
                tcp_port: LAN_TCP_PORT_DEFAULT,
                token: "skip".to_string(),
                last_seen_ms: 456,
                trusted: false,
                capabilities: vec!["text".to_string()],
            },
        ];
        let normalize = |mut capabilities: Vec<String>, _tcp_port: u16| {
            capabilities.sort();
            capabilities
        };

        let book =
            stored_book_from_lan_devices(&devices, |token| Some(format!("enc:{token}")), normalize);
        assert_eq!(book.devices.len(), 1);
        assert_eq!(book.devices[0].token_encrypted, "enc:secret");

        let decoded = lan_devices_from_stored_book(
            book,
            |encrypted| {
                encrypted
                    .strip_prefix("enc:")
                    .map(|token| token.to_string())
            },
            normalize,
        );
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].device_id, "trusted");
        assert_eq!(decoded[0].token, "secret");
    }

    #[test]
    fn stored_device_book_skips_devices_when_secret_codec_fails() {
        let book = StoredDeviceBook {
            devices: vec![StoredLanDevice {
                device_id: "device-1".to_string(),
                name: "Device".to_string(),
                addr: "127.0.0.1".to_string(),
                tcp_port: LAN_TCP_PORT_DEFAULT,
                token_encrypted: "bad".to_string(),
                last_seen_ms: 1,
                trusted: true,
                capabilities: Vec::new(),
            }],
        };

        let decoded = lan_devices_from_stored_book(book, |_| None, |caps, _| caps);

        assert!(decoded.is_empty());
    }

    #[test]
    fn device_book_path_is_derived_from_platform_data_dir() {
        assert_eq!(
            lan_device_book_path(Path::new("zsclip-data")),
            PathBuf::from("zsclip-data").join("lan_devices.json")
        );
    }

    #[test]
    fn upsert_lan_device_in_store_replaces_existing_device() {
        let data_dir = std::env::temp_dir().join(format!(
            "zsclip-lan-device-upsert-{}",
            LAN_TOKEN_COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        let normalize = |mut capabilities: Vec<String>, _tcp_port: u16| {
            capabilities.sort();
            capabilities
        };
        let original = LanDevice {
            device_id: "phone".to_string(),
            name: "Old Phone".to_string(),
            addr: "192.168.1.10".to_string(),
            tcp_port: LAN_TCP_PORT_DEFAULT,
            token: "old".to_string(),
            last_seen_ms: 1,
            trusted: true,
            capabilities: vec!["text".to_string()],
        };
        save_lan_devices_to_store(
            &data_dir,
            &[original],
            |token| Some(format!("enc:{token}")),
            normalize,
        )
        .expect("initial device book should save");

        let updated = LanDevice {
            device_id: "phone".to_string(),
            name: "New Phone".to_string(),
            addr: "192.168.1.11".to_string(),
            tcp_port: LAN_TCP_PORT_DEFAULT + 1,
            token: "new".to_string(),
            last_seen_ms: 2,
            trusted: true,
            capabilities: vec!["image".to_string(), "text".to_string()],
        };
        upsert_lan_device_in_store(
            &data_dir,
            updated,
            |encrypted| encrypted.strip_prefix("enc:").map(str::to_string),
            |token| Some(format!("enc:{token}")),
            normalize,
        )
        .expect("updated device should save");

        let devices = load_lan_devices_from_store(
            lan_device_book_path(&data_dir),
            |encrypted| encrypted.strip_prefix("enc:").map(str::to_string),
            normalize,
        );
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].device_id, "phone");
        assert_eq!(devices[0].name, "New Phone");
        assert_eq!(devices[0].token, "new");
        assert_eq!(devices[0].capabilities, vec!["image", "text"]);
        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn pending_pair_decision_store_accepts_and_rejects_pairs() {
        let data_dir = std::env::temp_dir().join(format!(
            "zsclip-lan-pending-pair-{}",
            LAN_TOKEN_COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        let normalize = |mut capabilities: Vec<String>, _tcp_port: u16| {
            capabilities.sort();
            capabilities
        };
        let pairs = vec![
            LanPendingPair {
                prompt: LanPairPrompt {
                    pair_id: "pair-accept".to_string(),
                    code: "123456".to_string(),
                    device_name: "Phone".to_string(),
                    addr: "192.168.1.30".to_string(),
                    created_at_ms: 1,
                },
                requester_device_id: "phone".to_string(),
                requester_tcp_port: LAN_TCP_PORT_DEFAULT,
                requester_capabilities: vec!["receive_clip".to_string(), "text".to_string()],
                token: "secret".to_string(),
                accepted: false,
                rejected: false,
                created_at_ms: 1,
            },
            LanPendingPair {
                prompt: LanPairPrompt {
                    pair_id: "pair-reject".to_string(),
                    code: "654321".to_string(),
                    device_name: "Tablet".to_string(),
                    addr: "192.168.1.31".to_string(),
                    created_at_ms: 1,
                },
                requester_device_id: "tablet".to_string(),
                requester_tcp_port: LAN_TCP_PORT_DEFAULT,
                requester_capabilities: vec!["text".to_string()],
                token: "reject-secret".to_string(),
                accepted: false,
                rejected: false,
                created_at_ms: 1,
            },
        ];
        save_lan_pending_pairs_to_store(
            &data_dir,
            &pairs,
            |token| Some(format!("enc:{token}")),
            normalize,
        )
        .expect("pending pair book should save");

        let accepted = apply_lan_pending_pair_decision_in_store(
            &data_dir,
            Some("pair-accept"),
            true,
            2,
            |encrypted| encrypted.strip_prefix("enc:").map(str::to_string),
            |token| Some(format!("enc:{token}")),
            normalize,
        )
        .expect("accept decision should save")
        .expect("accept decision should match a pair");
        assert!(accepted.accepted);
        assert_eq!(accepted.pair_id, "pair-accept");
        assert_eq!(
            accepted
                .trusted_device
                .as_ref()
                .map(|device| device.device_id.as_str()),
            Some("phone")
        );

        let devices = load_lan_devices_from_store(
            lan_device_book_path(&data_dir),
            |encrypted| encrypted.strip_prefix("enc:").map(str::to_string),
            normalize,
        );
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].device_id, "phone");
        assert_eq!(devices[0].token, "secret");

        let rejected = apply_lan_pending_pair_decision_in_store(
            &data_dir,
            Some("pair-reject"),
            false,
            2,
            |encrypted| encrypted.strip_prefix("enc:").map(str::to_string),
            |token| Some(format!("enc:{token}")),
            normalize,
        )
        .expect("reject decision should save")
        .expect("reject decision should match a pair");
        assert!(!rejected.accepted);
        assert_eq!(rejected.pair_id, "pair-reject");
        assert!(rejected.trusted_device.is_none());

        let stored_pairs = load_lan_pending_pairs_from_store(
            lan_pending_pair_book_path(&data_dir),
            |encrypted| encrypted.strip_prefix("enc:").map(str::to_string),
            normalize,
        );
        assert!(stored_pairs.iter().any(|pair| pair.accepted));
        assert!(stored_pairs.iter().any(|pair| pair.rejected));
        let _ = fs::remove_dir_all(data_dir);
    }
}
