//! QQ input method clipboard wire protocol and direct desktop upload.
//!
//! This legacy TEA framing is required by the service; HTTPS supplies transport
//! authentication. Never log account data, request URLs, bodies, or replies.

use serde::{Deserialize, Serialize};

const MAX_TEXT_UTF16: usize = 10_000;
const MAX_PROTOCOL_BYTES: usize = 128 * 1024;
const MAX_RESPONSE_BYTES: usize = MAX_PROTOCOL_BYTES * 3;
const SIGN_SUFFIX: &str = "p*&h>>=|[?@}q||6qqinput";
// QQ input method 8.7.15 libsecurity.so, protocol key number 0.
const TEA_KEY: [u32; 4] = [0x335f2639, 0x6b21497e, 0x462c4023, 0x79245e63];

// Deliberately no Debug: SGID is a bearer credential.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CloudAccount {
    pub(crate) sgid: String,
    pub(crate) device_id: String,
    pub(crate) version: String,
}

impl CloudAccount {
    pub(crate) fn validate(&self) -> Result<(), String> {
        for (value, limit, label) in [
            (&self.sgid, 4096, "QQ 云剪贴板授权"),
            (&self.device_id, 256, "设备标识"),
            (&self.version, 64, "输入法版本"),
        ] {
            if value.is_empty()
                || value.len() > limit
                || !value.bytes().all(|byte| byte.is_ascii_graphic())
            {
                return Err(format!("{label}无效，请在手机输入法中重新授权。"));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UploadOutcome {
    Uploaded,
    AlreadyPresent,
    Accepted,
}

impl UploadOutcome {
    pub(crate) fn status_text(self) -> &'static str {
        match self {
            Self::Uploaded => "已上传到 QQ 官方云剪贴板。",
            Self::AlreadyPresent => "QQ 官方云剪贴板中已存在该文本。",
            Self::Accepted => "QQ 云端已接受上传；未返回可核对的记录详情，请刷新手机云剪贴板查看。",
        }
    }
}

pub(crate) fn validate_text(text: &str) -> Result<(), String> {
    if text.trim().is_empty() {
        return Err("不能上传空白文本。".to_string());
    }
    if text.encode_utf16().take(MAX_TEXT_UTF16 + 1).count() > MAX_TEXT_UTF16 {
        return Err("超过本地单次上传上限（10,000 个 UTF-16 单元），请缩短文本。".to_string());
    }
    if text.contains('\0') {
        return Err("文本包含空字符，无法上传。".to_string());
    }
    Ok(())
}

pub(crate) fn upload_text(account: &CloudAccount, text: &str) -> Result<UploadOutcome, String> {
    upload_text_using(account, text, transport::post, transport::get)
}

fn upload_text_using(
    account: &CloudAccount,
    text: &str,
    post: impl FnOnce(&str, &str) -> Result<(u32, Vec<u8>), String>,
    get: impl FnOnce(&str) -> Result<(u32, Vec<u8>), String>,
) -> Result<UploadOutcome, String> {
    let (query, body) = prepare_upload(account, text)?;
    let (status, response) = post(&query, &body)?;
    let outcome = interpret_response(status, &response, text)?;
    if outcome != UploadOutcome::Accepted {
        return Ok(outcome);
    }
    // Some successful upload responses contain only a business acknowledgement.
    // Read back once; never submit the text again to compensate for missing details.
    let Ok(query) = prepare_query(account, serde_json::json!({"clipVersion":0}).to_string()) else {
        return Ok(UploadOutcome::Accepted);
    };
    Ok(match get(&query) {
        Ok((status, reply)) if readback_contains(status, &reply, text) => UploadOutcome::Uploaded,
        _ => UploadOutcome::Accepted,
    })
}

fn prepare_upload(account: &CloudAccount, text: &str) -> Result<(String, String), String> {
    account.validate()?;
    validate_text(text)?;
    let query = prepare_query(account, String::new())?;
    let body = serde_json::json!([{
        "guid": "", "cliptext": text, "action": 1, "accept": 3,
    }]);
    Ok((query, form_encode(&encode(body.to_string().as_bytes())?)))
}

fn prepare_query(account: &CloudAccount, data: String) -> Result<String, String> {
    account.validate()?;
    let query = serde_json::json!({
        "sgid": account.sgid,
        "ip": "",
        "deviceid": account.device_id,
        "platform": 2,
        "version": account.version,
        "sign": account_sign(&account.sgid),
        "data": data,
    });
    Ok(form_encode(&encode(query.to_string().as_bytes())?))
}

fn account_sign(sgid: &str) -> String {
    let mut state = md5::Context::new();
    state.consume(sgid.as_bytes());
    state.consume(SIGN_SUFFIX.as_bytes());
    format!("{:x}", state.compute())
}

// java.net.URLEncoder with ISO-8859-1: '*' is safe, '~' is escaped,
// space is '+', and bytes above 0x7f are escaped directly (not UTF-8).
fn form_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut result = String::with_capacity(bytes.len() * 3);
    for &byte in bytes {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'*' => {
                result.push(byte as char);
            }
            b' ' => result.push('+'),
            _ => {
                result.push('%');
                result.push(HEX[(byte >> 4) as usize] as char);
                result.push(HEX[(byte & 15) as usize] as char);
            }
        }
    }
    result
}

fn form_decode(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err("QQ 云剪贴板响应超过本地大小限制。".to_string());
    }
    let mut result = Vec::with_capacity(bytes.len().min(MAX_PROTOCOL_BYTES));
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                let pair = bytes
                    .get(i + 1..i + 3)
                    .ok_or("QQ 云剪贴板响应编码不完整。")?;
                let hi = hex_digit(pair[0]).ok_or("QQ 云剪贴板响应编码无效。")?;
                let lo = hex_digit(pair[1]).ok_or("QQ 云剪贴板响应编码无效。")?;
                result.push((hi << 4) | lo);
                i += 3;
            }
            b'+' => {
                result.push(b' ');
                i += 1;
            }
            byte if byte.is_ascii() => {
                result.push(byte);
                i += 1;
            }
            _ => return Err("QQ 云剪贴板响应编码无效。".to_string()),
        }
        if result.len() > MAX_PROTOCOL_BYTES {
            return Err("QQ 云剪贴板响应超过本地大小限制。".to_string());
        }
    }
    Ok(result)
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn tea_block(block: [u8; 8], decrypt: bool) -> [u8; 8] {
    let mut a = u32::from_be_bytes(block[..4].try_into().unwrap());
    let mut b = u32::from_be_bytes(block[4..].try_into().unwrap());
    let mut sum = if decrypt { 0xe3779b90u32 } else { 0u32 };
    let mix = |value: u32, sum: u32, k0: u32, k1: u32| {
        (value << 4).wrapping_add(k0) ^ value.wrapping_add(sum) ^ (value >> 5).wrapping_add(k1)
    };
    for _ in 0..16 {
        if decrypt {
            b = b.wrapping_sub(mix(a, sum, TEA_KEY[2], TEA_KEY[3]));
            a = a.wrapping_sub(mix(b, sum, TEA_KEY[0], TEA_KEY[1]));
            sum = sum.wrapping_sub(0x9e3779b9);
        } else {
            sum = sum.wrapping_add(0x9e3779b9);
            a = a.wrapping_add(mix(b, sum, TEA_KEY[0], TEA_KEY[1]));
            b = b.wrapping_add(mix(a, sum, TEA_KEY[2], TEA_KEY[3]));
        }
    }
    let mut result = [0; 8];
    result[..4].copy_from_slice(&a.to_be_bytes());
    result[4..].copy_from_slice(&b.to_be_bytes());
    result
}

fn encode_framed(framed: &[u8]) -> Vec<u8> {
    let mut before_cipher = [0; 8];
    let mut before_input = [0; 8];
    let mut result = Vec::with_capacity(framed.len());
    for chunk in framed.chunks_exact(8) {
        let mixed = std::array::from_fn(|i| chunk[i] ^ before_cipher[i]);
        let encrypted = tea_block(mixed, false);
        let ciphertext: [u8; 8] = std::array::from_fn(|i| encrypted[i] ^ before_input[i]);
        result.extend_from_slice(&ciphertext);
        before_input = mixed;
        before_cipher = ciphertext;
    }
    result
}

fn encode_with_random(data: &[u8], random: &[u8; 10]) -> Result<Vec<u8>, String> {
    if data.len() > MAX_PROTOCOL_BYTES - 24 {
        return Err("QQ 云剪贴板请求超过本地大小限制。".to_string());
    }
    let padding = (8 - (data.len() + 10) % 8) % 8;
    let mut framed = Vec::with_capacity(data.len() + padding + 10);
    framed.push((random[0] & 0xf8) | padding as u8);
    framed.extend_from_slice(&random[1..padding + 3]);
    framed.extend_from_slice(data);
    framed.extend_from_slice(&[0; 7]);
    Ok(encode_framed(&framed))
}

pub(crate) fn encode(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut random = [0; 10];
    transport::fill_random(&mut random)?;
    encode_with_random(data, &random)
}

pub(crate) fn decode(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < 16 || data.len() % 8 != 0 || data.len() > MAX_PROTOCOL_BYTES {
        return Err("QQ 云剪贴板响应加密长度无效。".to_string());
    }
    let mut before_cipher = [0; 8];
    let mut before_input = [0; 8];
    let mut framed = Vec::with_capacity(data.len());
    for chunk in data.chunks_exact(8) {
        let block = std::array::from_fn(|i| chunk[i] ^ before_input[i]);
        let mixed = tea_block(block, true);
        framed.extend((0..8).map(|i| mixed[i] ^ before_cipher[i]));
        before_input = mixed;
        before_cipher.copy_from_slice(chunk);
    }
    let padding = (framed[0] & 7) as usize;
    let start = padding + 3;
    let end = framed.len() - 7;
    if start > end || framed[end..].iter().any(|byte| *byte != 0) {
        return Err("QQ 云剪贴板响应加密填充无效。".to_string());
    }
    Ok(framed[start..end].to_vec())
}

fn interpret_response(
    http_status: u32,
    response: &[u8],
    expected_text: &str,
) -> Result<UploadOutcome, String> {
    if http_status != 200 && http_status != 202 {
        return Err(format!(
            "QQ 云剪贴板请求失败（HTTP {http_status}），未确认上传。"
        ));
    }
    let clear = zeroize::Zeroizing::new(decode(&form_decode(response)?)?);
    let reply: serde_json::Value = serde_json::from_slice(&clear)
        .map_err(|_| "QQ 云剪贴板响应格式无效，未确认上传。".to_string())?;
    let status = reply
        .get("status")
        .and_then(|value| value.as_i64().or_else(|| value.as_str()?.parse().ok()))
        .ok_or("QQ 云剪贴板响应缺少业务状态，未确认上传。")?;
    match status {
        0 => {}
        400 => return Err("QQ 官方云剪贴板已满，请清理云端记录后重试。".to_string()),
        500 => return Ok(UploadOutcome::AlreadyPresent),
        700 => return Err("QQ 云剪贴板登录已过期，请在手机输入法中重新授权电脑。".to_string()),
        _ => return Err(format!("QQ 云剪贴板拒绝上传（业务状态 {status}）。")),
    }
    // Android JSONObject.getString also stringifies a JSON array. Accept both
    // wire shapes, and distinguish a business acknowledgement from readback.
    Ok(if response_contains(&reply, expected_text) {
        UploadOutcome::Uploaded
    } else {
        UploadOutcome::Accepted
    })
}

fn parse_embedded_json(value: &serde_json::Value) -> Option<serde_json::Value> {
    if let Some(text) = value.as_str() {
        serde_json::from_str(text).ok()
    } else {
        Some(value.clone())
    }
}

fn response_contains(reply: &serde_json::Value, expected_text: &str) -> bool {
    let Some(mut data) = reply.get("data").and_then(parse_embedded_json) else { return false; };
    // The official download response wraps records in data.clipContent.
    if let Some(content) = data.get("clipContent") {
        let Some(content) = parse_embedded_json(content) else { return false; };
        data = content;
    }
    let Some(records) = data.as_array() else { return false; };
    records.iter().any(|record| {
        record.get("cliptext").and_then(serde_json::Value::as_str) == Some(expected_text)
            && record
                .get("guid")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|guid| !guid.is_empty())
    })
}

fn readback_contains(http_status: u32, response: &[u8], expected_text: &str) -> bool {
    if http_status != 200 { return false; }
    let Ok(bytes) = form_decode(response).and_then(|value| decode(&value)) else { return false; };
    let bytes = zeroize::Zeroizing::new(bytes);
    let Ok(reply) = serde_json::from_slice::<serde_json::Value>(&bytes) else { return false; };
    reply.get("status").and_then(|value| value.as_i64().or_else(|| value.as_str()?.parse().ok())) == Some(0)
        && response_contains(&reply, expected_text)
}

#[cfg(windows)]
mod transport {
    use super::MAX_RESPONSE_BYTES;
    use std::ffi::c_void;
    use std::ptr::{null, null_mut};
    use std::time::{Duration, Instant};

    const HOST: &str = "config.android.qqpy.sogou.com";
    const UPLOAD_PATH: &str = "/QQinput/clipboard/upload?q=";
    const TIMEOUT_MS: i32 = 15_000;

    #[link(name = "winhttp")]
    extern "system" {
        fn WinHttpOpen(
            agent: *const u16,
            access: u32,
            proxy: *const u16,
            bypass: *const u16,
            flags: u32,
        ) -> *mut c_void;
        fn WinHttpCloseHandle(handle: *mut c_void) -> i32;
        fn WinHttpConnect(
            session: *mut c_void,
            host: *const u16,
            port: u16,
            reserved: u32,
        ) -> *mut c_void;
        fn WinHttpOpenRequest(
            connection: *mut c_void,
            verb: *const u16,
            object: *const u16,
            version: *const u16,
            referer: *const u16,
            accept: *const *const u16,
            flags: u32,
        ) -> *mut c_void;
        fn WinHttpSetTimeouts(
            handle: *mut c_void,
            resolve: i32,
            connect: i32,
            send: i32,
            receive: i32,
        ) -> i32;
        fn WinHttpSetOption(
            handle: *mut c_void,
            option: u32,
            value: *mut c_void,
            length: u32,
        ) -> i32;
        fn WinHttpSendRequest(
            request: *mut c_void,
            headers: *const u16,
            headers_len: u32,
            body: *const c_void,
            body_len: u32,
            total_len: u32,
            context: usize,
        ) -> i32;
        fn WinHttpReceiveResponse(request: *mut c_void, reserved: *mut c_void) -> i32;
        fn WinHttpQueryHeaders(
            request: *mut c_void,
            level: u32,
            name: *const u16,
            buffer: *mut c_void,
            length: *mut u32,
            index: *mut u32,
        ) -> i32;
        fn WinHttpReadData(
            request: *mut c_void,
            buffer: *mut c_void,
            requested: u32,
            received: *mut u32,
        ) -> i32;
    }

    #[link(name = "bcrypt")]
    extern "system" {
        fn BCryptGenRandom(algorithm: *mut c_void, buffer: *mut u8, length: u32, flags: u32)
            -> i32;
    }

    pub(super) fn fill_random(bytes: &mut [u8]) -> Result<(), String> {
        let length = u32::try_from(bytes.len()).map_err(|_| "安全随机数请求过大。")?;
        // BCRYPT_USE_SYSTEM_PREFERRED_RNG, independent of clocks and account data.
        if unsafe { BCryptGenRandom(null_mut(), bytes.as_mut_ptr(), length, 2) } >= 0 {
            Ok(())
        } else {
            Err("无法生成安全随机数，已取消 QQ 云剪贴板请求。".to_string())
        }
    }

    struct InternetHandle(*mut c_void);
    impl InternetHandle {
        fn new(raw: *mut c_void) -> Result<Self, String> {
            if raw.is_null() {
                Err(network_error())
            } else {
                Ok(Self(raw))
            }
        }
    }
    impl Drop for InternetHandle {
        fn drop(&mut self) {
            unsafe {
                WinHttpCloseHandle(self.0);
            }
        }
    }

    fn network_error() -> String {
        let code = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
        format!("QQ 云剪贴板连接失败（系统错误 {code}），未确认上传。")
    }

    fn check(ok: i32) -> Result<(), String> {
        if ok == 0 {
            Err(network_error())
        } else {
            Ok(())
        }
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain([0]).collect()
    }

    fn set_option(handle: &InternetHandle, option: u32, mut value: u32) -> Result<(), String> {
        check(unsafe { WinHttpSetOption(handle.0, option, (&mut value as *mut u32).cast(), 4) })
    }

    pub(super) fn post(query: &str, body: &str) -> Result<(u32, Vec<u8>), String> {
        request(UPLOAD_PATH, query, Some(body))
    }

    pub(super) fn get(query: &str) -> Result<(u32, Vec<u8>), String> {
        request("/QQinput/clipboard/download?q=", query, None)
    }

    fn request(endpoint: &str, query: &str, body: Option<&str>) -> Result<(u32, Vec<u8>), String> {
        // Only this fixed HTTPS host is addressable. Request contents remain in
        // process memory; neither command lines nor temporary files are used.
        let agent = wide("ZSClip/QQCloud");
        let session =
            InternetHandle::new(unsafe { WinHttpOpen(agent.as_ptr(), 4, null(), null(), 0) })?;
        check(unsafe {
            WinHttpSetTimeouts(session.0, TIMEOUT_MS, TIMEOUT_MS, TIMEOUT_MS, TIMEOUT_MS)
        })?;
        // WINHTTP_OPTION_SECURE_PROTOCOLS: require TLS 1.2; no certificate bypass.
        set_option(&session, 84, 0x800)?;
        let host = wide(HOST);
        let connection =
            InternetHandle::new(unsafe { WinHttpConnect(session.0, host.as_ptr(), 443, 0) })?;
        let path = wide(&format!("{endpoint}{query}"));
        let verb = wide(if body.is_some() { "POST" } else { "GET" });
        let request = InternetHandle::new(unsafe {
            WinHttpOpenRequest(
                connection.0,
                verb.as_ptr(),
                path.as_ptr(),
                null(),
                null(),
                null(),
                0x0080_0080,
            )
        })?;
        // WINHTTP_OPTION_DISABLE_FEATURE: cookies | redirects | authentication.
        // Disallow redirecting encrypted bearer credentials to another endpoint.
        set_option(&request, 63, 1 | 2 | 4)?;
        // WINHTTP_OPTION_DECOMPRESSION, gzip and deflate, bounded after decoding.
        set_option(&request, 118, 1 | 2)?;
        let headers = wide("Content-Type: application/x-www-form-urlencoded\r\n");
        let body = body.unwrap_or_default();
        let body_len = u32::try_from(body.len()).map_err(|_| "上传请求过大。")?;
        check(unsafe {
            WinHttpSendRequest(
                request.0,
                headers.as_ptr(),
                (headers.len() - 1) as u32,
                body.as_ptr().cast(),
                body_len,
                body_len,
                0,
            )
        })?;
        check(unsafe { WinHttpReceiveResponse(request.0, null_mut()) })?;
        let mut status = 0u32;
        let mut status_size = 4u32;
        check(unsafe {
            WinHttpQueryHeaders(
                request.0,
                19 | 0x2000_0000,
                null(),
                (&mut status as *mut u32).cast(),
                &mut status_size,
                null_mut(),
            )
        })?;
        if status != 200 && status != 202 {
            return Ok((status, Vec::new()));
        }
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut response = Vec::new();
        let mut chunk = [0u8; 8192];
        loop {
            if Instant::now() >= deadline {
                return Err("QQ 云剪贴板响应超时，未确认上传。".to_string());
            }
            let mut read = 0;
            check(unsafe {
                WinHttpReadData(
                    request.0,
                    chunk.as_mut_ptr().cast(),
                    chunk.len() as u32,
                    &mut read,
                )
            })?;
            if read == 0 {
                break;
            }
            if response.len() + read as usize > MAX_RESPONSE_BYTES {
                return Err("QQ 云剪贴板响应超过本地大小限制。".to_string());
            }
            response.extend_from_slice(&chunk[..read as usize]);
        }
        Ok((status, response))
    }
}

#[cfg(not(windows))]
mod transport {
    pub(super) fn post(_query: &str, _body: &str) -> Result<(u32, Vec<u8>), String> {
        Err("QQ 官方云剪贴板独立上传目前仅支持 Windows。".to_string())
    }

    pub(super) fn get(_query: &str) -> Result<(u32, Vec<u8>), String> {
        Err("QQ 官方云剪贴板独立上传目前仅支持 Windows。".to_string())
    }

    pub(super) fn fill_random(bytes: &mut [u8]) -> Result<(), String> {
        #[cfg(unix)]
        {
            use std::io::Read;
            std::fs::File::open("/dev/urandom")
                .and_then(|mut file| file.read_exact(bytes))
                .map_err(|_| "无法生成安全随机数。".to_string())
        }
        #[cfg(not(unix))]
        {
            let _ = bytes;
            Err("当前系统不支持安全随机数。".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_random() -> [u8; 10] {
        std::array::from_fn(|i| (((i + 1) * 7919 + 17) & 255) as u8)
    }

    fn bytes_from_hex(hex: &str) -> Vec<u8> {
        hex.as_bytes()
            .chunks_exact(2)
            .map(|pair| (hex_digit(pair[0]).unwrap() << 4) | hex_digit(pair[1]).unwrap())
            .collect()
    }

    fn fixture_account() -> CloudAccount {
        CloudAccount {
            sgid: "fixture-account".to_string(),
            device_id: "fixture-device".to_string(),
            version: "8.7.15.1".to_string(),
        }
    }

    fn wire_reply(reply: serde_json::Value) -> String {
        form_encode(&encode_with_random(reply.to_string().as_bytes(), &fixture_random()).unwrap())
    }

    #[test]
    fn tea_matches_original_arm64_library_vectors() {
        // Original libsecurity.so SHA-256:
        // b35f21308014c6249a4ef9c39909201485b120d58c2ec92e9ff7116cea1ba7d6.
        // Reference values came from its unmodified ARM64 encryption routine,
        // with rand() returning (counter * 7919 + 17) & 0x7fffffff.
        let vectors = [
            (0, "8848114b90c89cd09f120d3bbb92591a"),
            (1, "6b6125af09c464d54d7a16f6d50d2c80"),
            (7, "cc1135b62c7c406a1711aeef41da21fcf039682231bac137"),
            (8, "8848114b90c89cd07b64607feaabd0f812e540f51e942a2f"),
            (15, "cc1135b62c7c406a1711aeef41da21fc1449ee70b2facd2adbdfeb30e347e788"),
            (16, "8848114b90c89cd07b64607feaabd0f876b9fca1b7fab944e55f56f82cc7deef"),
            (31, "cc1135b62c7c406a1711aeef41da21fc1449ee70b2facd2a488a9e5cb40cd41d1e3413ad74420888648b92d2cf9540c5"),
            (32, "8848114b90c89cd07b64607feaabd0f876b9fca1b7fab944e7fc6a58b739b98596ed68db8e164c4f2c0166d37f3970cd"),
            (128, "8848114b90c89cd07b64607feaabd0f876b9fca1b7fab944e7fc6a58b739b98596ed68db8e164c4f639dd6063a36d13f6f3295694198bb2e1c54b6fcc1452f1d2ae86f7f42a1868f3a0d31e34f10d2c66d80fef354e76d0b42ce84cd3af70e2128d3a63268d5c83cecbdf52b83854f6f1cade48c14745db79e9308efc2d03a0b8e0e0815e8ca0ccd95e0ddde344bd40e"),
        ];
        for (length, cipher) in vectors {
            let clear: Vec<u8> = (0..length).map(|i| ((i * 97 + 51) & 255) as u8).collect();
            let expected = bytes_from_hex(cipher);
            assert_eq!(
                encode_with_random(&clear, &fixture_random()).unwrap(),
                expected
            );
            assert_eq!(decode(&expected).unwrap(), clear);
        }
    }

    #[test]
    fn protocol_roundtrip_covers_every_padding_length_and_large_text() {
        for length in (0..80).chain([127, 128, 129, 255, 256, 257, 1024, 8192]) {
            let clear: Vec<u8> = (0..length).map(|i| ((i * 97 + 51) & 255) as u8).collect();
            let encrypted = encode_with_random(&clear, &fixture_random()).unwrap();
            assert_eq!(decode(&encrypted).unwrap(), clear);
        }
    }

    #[test]
    fn malformed_ciphertext_padding_and_trailers_are_rejected() {
        assert!(decode(&[]).is_err());
        assert!(decode(&[0; 8]).is_err());
        assert!(decode(&[0; 17]).is_err());
        assert!(decode(&vec![0; MAX_PROTOCOL_BYTES + 8]).is_err());
        // 16-byte frame cannot contain padding=7 plus salt and trailer.
        let mut invalid_padding = [0; 16];
        invalid_padding[0] = 7;
        assert!(decode(&encode_framed(&invalid_padding)).is_err());
        let mut invalid_trailer = [0; 16];
        invalid_trailer[15] = 1;
        assert!(decode(&encode_framed(&invalid_trailer)).is_err());
    }

    #[test]
    fn java_form_encoding_preserves_iso_bytes_and_special_characters() {
        let all_bytes: Vec<u8> = (0..=255).collect();
        let encoded = form_encode(&all_bytes);
        assert_eq!(form_decode(encoded.as_bytes()).unwrap(), all_bytes);
        assert_eq!(form_encode(b" *~+%&\xff"), "+*%7E%2B%25%26%FF");
        for invalid in ["%", "%0", "%xz", "中文"] {
            assert!(form_decode(invalid.as_bytes()).is_err());
        }
        assert!(form_decode(&vec![b'A'; MAX_RESPONSE_BYTES + 1]).is_err());
    }

    #[test]
    fn account_sign_matches_independent_md5_fixture() {
        assert_eq!(
            account_sign("fixture-account"),
            "ea0167c4378320b0cb81b1bd559138c8"
        );
    }

    #[test]
    fn upload_acknowledgement_decodes_original_arm64_encrypted_fixture() {
        // Synthetic response encrypted by the same unmodified ARM64 routine as
        // the block vectors above; this test does not call our encoder.
        let wire = concat!(
            "A%26%E0%3C%97%C2Q%85m%93%D6%BB%B8%84%0E%AB%26y%2B%E5%CF%88EC%CB",
            "%D2%5E%8DV%C8Hav%AB%A8MS%E0%C2%12%25z%80%23d_h%85q%C5%B3j%3C",
            "%2F%5C%84%B5%08KY%1D%BD%C8%9B%90%D5P%0C%D3H%8A%21%CC_%0A%26",
            "%B2%C8%84%FA%FC%F3%F2%9EN%5E%FF.X%EF%9A%E9%ECe%94%8D"
        );
        assert_eq!(
            interpret_response(200, wire.as_bytes(), "desktop fixture").unwrap(),
            UploadOutcome::Uploaded
        );
        assert_eq!(interpret_response(200, wire.as_bytes(), "different fixture").unwrap(), UploadOutcome::Accepted);
    }

    #[test]
    fn upload_serialization_preserves_unicode_and_exact_protocol_fields() {
        let account = fixture_account();
        let text = "  中文 😀\n\"quote\" & + % ";
        let (query, body) = prepare_upload(&account, text).unwrap();
        let query: serde_json::Value =
            serde_json::from_slice(&decode(&form_decode(query.as_bytes()).unwrap()).unwrap())
                .unwrap();
        let body: serde_json::Value =
            serde_json::from_slice(&decode(&form_decode(body.as_bytes()).unwrap()).unwrap())
                .unwrap();
        assert_eq!(query["sgid"], account.sgid);
        assert_eq!(query["deviceid"], account.device_id);
        assert_eq!(query["version"], account.version);
        assert_eq!(query["platform"], 2);
        assert_eq!(query["ip"], "");
        assert_eq!(query["data"], "");
        assert_eq!(query["sign"], account_sign(&account.sgid));
        assert_eq!(
            body,
            serde_json::json!([{"guid":"", "cliptext":text, "action":1, "accept":3}])
        );
    }

    #[test]
    fn blank_and_oversized_text_and_malformed_account_are_rejected() {
        assert!(validate_text(" \r\n\t").is_err());
        assert!(validate_text("\0").is_err());
        assert!(validate_text(&"字".repeat(10_000)).is_ok());
        assert!(validate_text(&"😀".repeat(5_000)).is_ok());
        assert!(validate_text(&"😀".repeat(5_001)).is_err());
        let mut account = fixture_account();
        account.sgid = "\ninvalid".to_string();
        assert!(account.validate().is_err());
        account.sgid = "x".repeat(4097);
        assert!(account.validate().is_err());
        account.sgid.clear();
        assert!(account.validate().is_err());
    }

    #[test]
    fn successful_upload_requires_business_success_and_matching_record() {
        let body = wire_reply(
            serde_json::json!({"status":0, "data":"[{\"guid\":\"fixture-guid\",\"cliptext\":\"你好\"}]"}),
        );
        assert_eq!(
            interpret_response(200, body.as_bytes(), "你好").unwrap(),
            UploadOutcome::Uploaded
        );
        assert_eq!(
            interpret_response(202, body.as_bytes(), "你好").unwrap(),
            UploadOutcome::Uploaded
        );
        assert_eq!(interpret_response(200, body.as_bytes(), "other").unwrap(), UploadOutcome::Accepted);
        for status in [201, 204, 301, 302, 307, 400, 500] {
            assert!(interpret_response(status, body.as_bytes(), "你好").is_err());
        }
        for reply in [
            serde_json::json!({"data":"[]"}),
            serde_json::json!({"status":"success", "data":"[]"}),
        ] {
            assert!(interpret_response(200, wire_reply(reply).as_bytes(), "你好").is_err());
        }
        for reply in [
            serde_json::json!({"status":"0", "data":"[]"}),
            serde_json::json!({"status":0}),
            serde_json::json!({"status":0, "data":"[]"}),
            serde_json::json!({"status":0, "data":"invalid"}),
            serde_json::json!({"status":0, "data":"[{\"guid\":\"\",\"cliptext\":\"你好\"}]"}),
        ] {
            assert_eq!(interpret_response(200, wire_reply(reply).as_bytes(), "你好").unwrap(), UploadOutcome::Accepted);
        }
    }

    #[test]
    fn native_array_ack_and_download_readback_preserve_exact_record_checks() {
        let records = serde_json::json!([{"guid":"fixture-guid","cliptext":"  原文\n😀  "}]);
        for data in [records.clone(), serde_json::Value::String(records.to_string())] {
            let direct = wire_reply(serde_json::json!({"status":0,"data":data}));
            assert_eq!(interpret_response(200, direct.as_bytes(), "  原文\n😀  ").unwrap(), UploadOutcome::Uploaded);
            let nested = serde_json::json!({"clipContent":data,"clipVersion":7});
            for data in [nested.clone(), serde_json::Value::String(nested.to_string())] {
                let download = wire_reply(serde_json::json!({"status":0,"data":data}));
                assert!(readback_contains(200, download.as_bytes(), "  原文\n😀  "));
                assert!(!readback_contains(200, download.as_bytes(), "原文\n😀"));
                assert!(!readback_contains(500, download.as_bytes(), "  原文\n😀  "));
            }
        }
        for reply in [serde_json::json!({"status":700,"data":records}), serde_json::json!({"status":0}), serde_json::json!({"status":0,"data":{"clipContent":"invalid"}})] {
            assert!(!readback_contains(200, wire_reply(reply).as_bytes(), "  原文\n😀  "));
        }
        let query = prepare_query(&fixture_account(), serde_json::json!({"clipVersion":0}).to_string()).unwrap();
        let decoded: serde_json::Value = serde_json::from_slice(&decode(&form_decode(query.as_bytes()).unwrap()).unwrap()).unwrap();
        assert_eq!(decoded["data"], "{\"clipVersion\":0}");
    }

    #[test]
    fn accepted_upload_is_read_back_once_without_resubmitting_or_false_failure() {
        let posts = std::cell::Cell::new(0);
        let gets = std::cell::Cell::new(0);
        let outcome = upload_text_using(&fixture_account(), "fixture", |_, _| {
            posts.set(posts.get() + 1);
            Ok((200, wire_reply(serde_json::json!({"status":0})).into_bytes()))
        }, |_| {
            gets.set(gets.get() + 1);
            Ok((200, wire_reply(serde_json::json!({"status":0,"data":{"clipContent":[{"guid":"g","cliptext":"fixture"}],"clipVersion":2}})).into_bytes()))
        }).unwrap();
        assert_eq!(outcome, UploadOutcome::Uploaded);
        assert_eq!((posts.get(), gets.get()), (1, 1));
        let outcome = upload_text_using(&fixture_account(), "fixture", |_, _| {
            Ok((200, wire_reply(serde_json::json!({"status":0})).into_bytes()))
        }, |_| Err("synthetic readback unavailable".into())).unwrap();
        assert_eq!(outcome, UploadOutcome::Accepted);
        assert!(upload_text_using(&fixture_account(), "fixture", |_, _| {
            Ok((200, wire_reply(serde_json::json!({"status":700})).into_bytes()))
        }, |_| panic!("business errors must not trigger readback")).is_err());
        assert_eq!(upload_text_using(&fixture_account(), "fixture", |_, _| {
            Ok((200, wire_reply(serde_json::json!({"status":500})).into_bytes()))
        }, |_| panic!("duplicate acknowledgement must not be resubmitted")).unwrap(), UploadOutcome::AlreadyPresent);
    }

    #[test]
    fn business_errors_and_duplicates_are_not_reported_as_uploaded() {
        for (status, expected) in [
            (400, "已满"),
            (700, "登录已过期"),
            (100, "拒绝上传"),
            (200, "拒绝上传"),
            (300, "拒绝上传"),
            (-1, "拒绝上传"),
        ] {
            let response = wire_reply(serde_json::json!({"status":status}));
            assert!(interpret_response(200, response.as_bytes(), "text")
                .unwrap_err()
                .contains(expected));
        }
        let duplicate = wire_reply(serde_json::json!({"status":500}));
        assert_eq!(
            interpret_response(200, duplicate.as_bytes(), "text").unwrap(),
            UploadOutcome::AlreadyPresent
        );
        assert!(interpret_response(200, b"{\"status\":0}", "text").is_err());
        let invalid = form_encode(&encode_with_random(b"not json", &fixture_random()).unwrap());
        assert!(interpret_response(200, invalid.as_bytes(), "text").is_err());
    }
}
