use std::ffi::OsString;
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Debug)]
pub(crate) struct WebDavConfig {
    pub(crate) base_url: String,
    pub(crate) username: String,
    pub(crate) password: String,
    pub(crate) remote_dir: String,
}

impl WebDavConfig {
    pub(crate) fn is_ready(&self) -> bool {
        !self.base_url.trim().is_empty()
    }

    fn normalized_base_url(&self) -> String {
        let mut base = self.base_url.trim().trim_end_matches('/').to_string();
        base.push('/');
        base
    }

    fn normalized_remote_dir(&self) -> String {
        self.remote_dir.trim().trim_matches('/').replace('\\', "/")
    }

    fn remote_path(&self, relative: &str) -> String {
        let rel = relative.trim().trim_start_matches('/').replace('\\', "/");
        let dir = self.normalized_remote_dir();
        if dir.is_empty() {
            rel
        } else if rel.is_empty() {
            dir
        } else {
            format!("{dir}/{rel}")
        }
    }

    fn remote_url(&self, remote_relative: &str) -> String {
        format!("{}{}", self.normalized_base_url(), encode_remote_path(&self.remote_path(remote_relative)))
    }
}

fn encode_remote_path(path: &str) -> String {
    let mut out = String::new();
    for (idx, seg) in path.split('/').filter(|s| !s.is_empty()).enumerate() {
        if idx > 0 {
            out.push('/');
        }
        out.push_str(&percent_encode(seg));
    }
    out
}

fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        let safe = matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~');
        if safe {
            out.push(b as char);
        } else {
            out.push('%');
            out.push(hex_digit((b >> 4) & 0x0F));
            out.push(hex_digit(b & 0x0F));
        }
    }
    out
}

fn hex_digit(v: u8) -> char {
    match v {
        0..=9 => (b'0' + v) as char,
        _ => (b'A' + (v - 10)) as char,
    }
}

fn curl_base_args(cfg: &WebDavConfig) -> Vec<OsString> {
    let mut args = vec![
        OsString::from("--silent"),
        OsString::from("--show-error"),
        OsString::from("--location"),
        OsString::from("--path-as-is"),
        OsString::from("--user-agent"),
        OsString::from("ZSClip-WebDAVLite/1.0"),
    ];
    if !cfg.username.is_empty() || !cfg.password.is_empty() {
        args.push(OsString::from("--user"));
        args.push(OsString::from(format!("{}:{}", cfg.username, cfg.password)));
    }
    args
}

fn run_curl_collect(mut args: Vec<OsString>) -> Result<(String, String), String> {
    args.push(OsString::from("--write-out"));
    args.push(OsString::from("\n%{http_code}"));

    let output = Command::new("curl.exe")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("unable to start curl.exe: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !output.status.success() && stdout.trim().is_empty() {
        return Err(if !stderr.is_empty() { stderr } else { "curl request failed".to_string() });
    }
    Ok((stdout, stderr))
}

fn parse_http_code(output: &str) -> (u16, String) {
    let normalized = output.replace("\r\n", "\n");
    if let Some((body, code_line)) = normalized.rsplit_once('\n') {
        let code = code_line.trim().parse::<u16>().unwrap_or(0);
        (code, body.to_string())
    } else {
        (0, normalized)
    }
}

fn validate_http(code: u16, body: &str, stderr: &str, ok_codes: &[u16], url: &str, cfg: &WebDavConfig) -> Result<(), String> {
    if ok_codes.contains(&code) {
        return Ok(());
    }
    if code == 401 || code == 403 {
        if cfg.base_url.to_ascii_lowercase().contains("jianguoyun.com") {
            return Err(format!("AUTH FAILED for {url}. Use app password and base url https://dav.jianguoyun.com/dav/"));
        }
        return Err(format!("AUTH FAILED for {url}"));
    }
    if code == 404 {
        return Err(format!("REMOTE FILE NOT FOUND: {url}"));
    }
    let detail = if !stderr.is_empty() { stderr.to_string() } else { body.trim().to_string() };
    Err(format!("HTTP {code} for {url}: {detail}"))
}

fn ensure_remote_dir(cfg: &WebDavConfig, remote_dir: &str) -> Result<(), String> {
    let dir = remote_dir.trim().trim_matches('/').replace('\\', "/");
    if dir.is_empty() {
        return Ok(());
    }

    let mut current = String::new();
    for seg in dir.split('/').filter(|s| !s.is_empty()) {
        if !current.is_empty() {
            current.push('/');
        }
        current.push_str(seg);
        let url = format!("{}{}", cfg.normalized_base_url(), encode_remote_path(&current));
        let mut args = curl_base_args(cfg);
        args.push(OsString::from("--request"));
        args.push(OsString::from("MKCOL"));
        args.push(OsString::from("--output"));
        args.push(OsString::from("-"));
        args.push(OsString::from(url.clone()));
        let (stdout, stderr) = run_curl_collect(args)?;
        let (code, body) = parse_http_code(&stdout);
        validate_http(code, &body, &stderr, &[200, 201, 204, 405], &url, cfg)?;
    }

    Ok(())
}

pub(crate) fn upload_file(cfg: &WebDavConfig, local_path: &Path, remote_relative: &str) -> Result<(), String> {
    if !cfg.is_ready() {
        return Err("please fill WebDAV URL first".to_string());
    }
    if !local_path.exists() {
        return Err(format!("local file not found: {}", local_path.display()));
    }

    let remote_path = cfg.remote_path(remote_relative);
    if let Some((parent, _)) = remote_path.rsplit_once('/') {
        ensure_remote_dir(cfg, parent)?;
    }

    let url = cfg.remote_url(remote_relative);
    let mut args = curl_base_args(cfg);
    args.push(OsString::from("--request"));
    args.push(OsString::from("PUT"));
    args.push(OsString::from("--header"));
    args.push(OsString::from("Content-Type: application/octet-stream"));
    args.push(OsString::from("--upload-file"));
    args.push(local_path.as_os_str().to_os_string());
    args.push(OsString::from("--output"));
    args.push(OsString::from("-"));
    args.push(OsString::from(url.clone()));

    let (stdout, stderr) = run_curl_collect(args)?;
    let (code, body) = parse_http_code(&stdout);
    validate_http(code, &body, &stderr, &[200, 201, 204], &url, cfg)
}

pub(crate) fn download_file(cfg: &WebDavConfig, remote_relative: &str, local_path: &Path) -> Result<(), String> {
    if !cfg.is_ready() {
        return Err("please fill WebDAV URL first".to_string());
    }
    if let Some(parent) = local_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let url = cfg.remote_url(remote_relative);
    let mut args = curl_base_args(cfg);
    args.push(OsString::from("--output"));
    args.push(local_path.as_os_str().to_os_string());
    args.push(OsString::from(url.clone()));

    let (stdout, stderr) = run_curl_collect(args)?;
    let (code, body) = parse_http_code(&stdout);
    match validate_http(code, &body, &stderr, &[200], &url, cfg) {
        Ok(()) => Ok(()),
        Err(err) => {
            let _ = fs::remove_file(local_path);
            Err(err)
        }
    }
}

pub(crate) fn upload_tree(cfg: &WebDavConfig, local_dir: &Path, remote_root: &str) -> Result<usize, String> {
    if !local_dir.exists() {
        return Ok(0);
    }

    let mut uploaded = 0usize;
    let mut stack = vec![PathBuf::from(local_dir)];
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).map_err(|e| format!("read dir failed: {e}"))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("read dir entry failed: {e}"))?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let rel = path
                .strip_prefix(local_dir)
                .map_err(|e| format!("strip prefix failed: {e}"))?
                .to_string_lossy()
                .replace('\\', "/");
            let remote = if remote_root.trim().is_empty() {
                rel
            } else {
                format!("{}/{}", remote_root.trim_matches('/'), rel)
            };
            upload_file(cfg, &path, &remote)?;
            uploaded += 1;
        }
    }

    Ok(uploaded)
}
