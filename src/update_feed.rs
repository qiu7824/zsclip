//! Public update metadata. No downloaded program is trusted without its digest.
use serde::{Deserialize, Serialize};

pub(crate) const MAX_MANIFEST_BYTES: usize = 128 * 1024;
pub(crate) const MAX_INSTALLER_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateSource {
    #[serde(default)]
    pub(crate) manifest_url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct InstallerAsset {
    pub(crate) url: String,
    pub(crate) sha256: String,
    pub(crate) size: u64,
    #[serde(default)]
    pub(crate) file_name: Option<String>,
    /// Optional public mirror. Size and hash always come from the canonical release.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) mirror_url: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct ReleaseInfo {
    pub(crate) version: String,
    pub(crate) page_url: String,
    pub(crate) notes: String,
    pub(crate) installer: Option<InstallerAsset>,
}

pub(crate) fn https_url(value: &str) -> bool {
    if value.len() > 4096
        || value
            .bytes()
            .any(|c| c.is_ascii_control() || c == b' ' || c == b'\\')
    {
        return false;
    }
    let Some(rest) = value.strip_prefix("https://") else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = authority.strip_suffix(":443").unwrap_or(authority);
    !host.is_empty()
        && host.contains('.')
        && !host.starts_with('.')
        && !host.ends_with('.')
        && host
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'.' || c == b'-')
}

pub(crate) fn validate_source(source: &UpdateSource) -> Result<(), String> {
    let url = source.manifest_url.trim();
    if url.is_empty() {
        return Ok(());
    }
    if !https_url(url) {
        return Err("版本信息地址必须是公开 HTTPS 地址。".into());
    }
    if url.split('?').next().unwrap_or(url).contains("/console/") {
        return Err("这是网盘管理后台地址，请填写可公开读取的版本信息地址。".into());
    }
    Ok(())
}

fn parts(value: &str) -> Option<Vec<u32>> {
    let value = value.trim().trim_start_matches(['v', 'V']);
    let pieces = value.split('.').collect::<Vec<_>>();
    if !(3..=4).contains(&pieces.len()) {
        return None;
    }
    let mut numbers = pieces
        .iter()
        .map(|piece| {
            if piece.is_empty() || !piece.bytes().all(|c| c.is_ascii_digit()) {
                None
            } else {
                piece.parse::<u32>().ok()
            }
        })
        .collect::<Option<Vec<_>>>()?;
    numbers.resize(4, 0);
    Some(numbers)
}

pub(crate) fn newer(latest: &str, current: &str) -> bool {
    parts(latest)
        .zip(parts(current))
        .is_some_and(|(a, b)| a > b)
}

fn validate_asset(mut asset: InstallerAsset) -> Result<InstallerAsset, String> {
    if !https_url(&asset.url)
        || !(4096..=MAX_INSTALLER_BYTES).contains(&asset.size)
        || asset.sha256.len() != 64
        || !asset.sha256.bytes().all(|c| c.is_ascii_hexdigit())
    {
        return Err("更新安装包的地址、大小或 SHA-256 校验信息无效。".into());
    }
    if asset.file_name.as_ref().is_some_and(|name| {
        name.is_empty()
            || name.len() > 160
            || !name
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.'))
            || !name.ends_with(".exe")
    }) {
        return Err("更新安装包文件名无效。".into());
    }
    if asset
        .mirror_url
        .as_deref()
        .is_some_and(|url| !crate::lanzou_update::is_share_url(url))
    {
        return Err("安装包镜像必须是公开蓝奏分享链接。".into());
    }
    asset.sha256.make_ascii_lowercase();
    Ok(asset)
}

pub(crate) fn parse_release(
    body: &[u8],
    custom: bool,
    no_lan: bool,
) -> Result<ReleaseInfo, String> {
    if body.len() > MAX_MANIFEST_BYTES {
        return Err("版本信息超过大小限制。".into());
    }
    let json: serde_json::Value = serde_json::from_slice(body)
        .map_err(|_| "版本信息不是有效的 JSON；网盘分享页不能直接作为版本信息地址。")?;
    let version_key = if custom { "version" } else { "tag_name" };
    let version = json
        .get(version_key)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .trim()
        .to_string();
    if parts(&version).is_none() {
        return Err("版本信息缺少有效版本号。".into());
    }
    if custom && json.get("format").and_then(|v| v.as_str()) != Some("zsclip-update-v1") {
        return Err("版本信息格式不受支持。".into());
    }
    let page_key = if custom { "page_url" } else { "html_url" };
    let page_url = json
        .get(page_key)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    if !https_url(&page_url) {
        return Err("发布页面地址无效。".into());
    }
    let notes_key = if custom { "notes" } else { "body" };
    let notes = json
        .get(notes_key)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .chars()
        .take(16_000)
        .collect();
    let installer = if custom {
        let key = if no_lan {
            "windows_x64_no_lan"
        } else {
            "windows_x64"
        };
        json.get(key)
            .filter(|value| !value.is_null())
            .map(|value| {
                serde_json::from_value::<InstallerAsset>(value.clone())
                    .map_err(|_| "安装包信息不完整。".to_string())
                    .and_then(validate_asset)
            })
            .transpose()?
    } else {
        let name = format!(
            "zsclip-v{}-setup{}.exe",
            version.trim_start_matches(['v', 'V']),
            if no_lan { "-no-lan" } else { "" }
        );
        json.get("assets")
            .and_then(|v| v.as_array())
            .and_then(|assets| {
                assets
                    .iter()
                    .find(|asset| asset.get("name").and_then(|v| v.as_str()) == Some(&name))
            })
            .and_then(|asset| {
                Some(InstallerAsset {
                    url: asset.get("browser_download_url")?.as_str()?.into(),
                    sha256: asset
                        .get("digest")?
                        .as_str()?
                        .strip_prefix("sha256:")?
                        .into(),
                    size: asset.get("size")?.as_u64()?,
                    file_name: Some(name.clone()),
                    mirror_url: None,
                })
            })
            .map(validate_asset)
            .transpose()?
    };
    Ok(ReleaseInfo {
        version,
        page_url,
        notes,
        installer,
    })
}

fn github_asset<'a>(
    json: &'a serde_json::Value,
    name: &str,
) -> Result<Option<&'a serde_json::Value>, String> {
    let mut matches = json
        .get("assets")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter(|asset| asset.get("name").and_then(|v| v.as_str()) == Some(name));
    let first = matches.next();
    if matches.next().is_some() {
        return Err(format!("发布中有多个同名资产 {name}。"));
    }
    Ok(first)
}

fn canonical_asset_url<'a>(
    asset: &'a serde_json::Value,
    repository: &str,
    tag: &str,
    name: &str,
) -> Result<&'a str, String> {
    let url = asset
        .get("browser_download_url")
        .and_then(|v| v.as_str())
        .ok_or("发布文件缺少下载地址。")?;
    if url
        != format!(
            "{}/releases/download/{tag}/{name}",
            repository.trim_end_matches('/')
        )
    {
        return Err("更新文件不属于当前官方发布。".into());
    }
    Ok(url)
}

fn checksum(body: &[u8], name: &str) -> Result<String, String> {
    if body.len() > MAX_MANIFEST_BYTES {
        return Err("校验清单超过大小限制。".into());
    }
    let text = std::str::from_utf8(body).map_err(|_| "发布校验清单不是有效文本。")?;
    let mut matches = text.lines().filter_map(|line| {
        let mut words = line.split_whitespace();
        let hash = words.next()?;
        let file = words.next()?.trim_start_matches('*');
        (file == name).then_some((hash, words.next().is_none()))
    });
    let (hash, complete) = matches.next().ok_or("安装包缺少 SHA-256 校验值。")?;
    if matches.next().is_some()
        || !complete
        || hash.len() != 64
        || !hash.bytes().all(|c| c.is_ascii_hexdigit())
    {
        return Err("安装包缺少唯一有效的 SHA-256 校验值。".into());
    }
    Ok(hash.to_ascii_lowercase())
}

fn mirror(body: &[u8], version: &str, name: &str) -> Result<Option<String>, String> {
    if body.len() > MAX_MANIFEST_BYTES {
        return Err("镜像清单超过大小限制。".into());
    }
    let json: serde_json::Value = serde_json::from_slice(body).map_err(|_| "镜像清单格式无效。")?;
    if json.get("version").and_then(|v| v.as_str()).and_then(parts) != parts(version) {
        return Err("蓝奏镜像清单与发布版本不一致。".into());
    }
    let files = json
        .get("files")
        .and_then(|v| v.as_array())
        .ok_or("镜像清单缺少文件列表。")?;
    let mut matches = files
        .iter()
        .filter(|file| file.get("name").and_then(|v| v.as_str()) == Some(name));
    let Some(entry) = matches.next() else {
        return Ok(None);
    };
    if matches.next().is_some() {
        return Err("镜像清单包含重复的安装包。".into());
    }
    if entry
        .get("code")
        .and_then(|v| v.as_str())
        .is_some_and(|code| !code.is_empty())
    {
        return Err("自动更新镜像需要无需提取码的公开分享。".into());
    }
    let url = entry
        .get("share_url")
        .and_then(|v| v.as_str())
        .ok_or("镜像清单缺少分享地址。")?;
    if !crate::lanzou_update::is_share_url(url) {
        return Err("镜像清单中的蓝奏分享地址无效。".into());
    }
    Ok(Some(url.into()))
}

/// Official releases bind the mirror to the exact release asset, SHA-256 and size.
pub(crate) fn github_release(
    body: &[u8],
    no_lan: bool,
    repository: &str,
    fetch: &mut dyn FnMut(&str, bool) -> Result<Vec<u8>, String>,
) -> Result<ReleaseInfo, String> {
    if body.len() > MAX_MANIFEST_BYTES {
        return Err("版本信息超过大小限制。".into());
    }
    let json: serde_json::Value =
        serde_json::from_slice(body).map_err(|_| "GitHub 版本信息格式无效。")?;
    if json.get("draft").and_then(|v| v.as_bool()) == Some(true)
        || json.get("prerelease").and_then(|v| v.as_bool()) == Some(true)
    {
        return Err("该发布尚不是正式版本。".into());
    }
    let mut release = parse_release(body, false, no_lan)?;
    let tag = &release.version;
    if release.page_url != format!("{}/releases/tag/{tag}", repository.trim_end_matches('/')) {
        return Err("版本页面不属于当前官方发布。".into());
    }
    let name = format!(
        "zsclip-v{}-setup{}.exe",
        tag.trim_start_matches(['v', 'V']),
        if no_lan { "-no-lan" } else { "" }
    );
    let Some(asset) = github_asset(&json, &name)? else {
        return Ok(release);
    };
    let url = canonical_asset_url(asset, repository, tag, &name)?;
    let size = asset
        .get("size")
        .and_then(|v| v.as_u64())
        .ok_or("发布文件缺少准确字节数。")?;
    let digest = asset
        .get("digest")
        .and_then(|v| v.as_str())
        .and_then(|digest| digest.strip_prefix("sha256:"));
    let sums = github_asset(&json, "SHA256SUMS.txt")?.or(github_asset(&json, "SHA256SUMS")?);
    let sha256 = if let Some(sums) = sums {
        let sums_name = sums
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or("校验清单缺少文件名。")?;
        let sums_url = canonical_asset_url(sums, repository, tag, sums_name)?;
        let hash = checksum(&fetch(sums_url, false)?, &name)?;
        if digest.is_some_and(|digest| !digest.eq_ignore_ascii_case(&hash)) {
            return Err("GitHub 文件摘要与 SHA256SUMS 校验清单不一致。".into());
        }
        hash
    } else if let Some(digest) = digest {
        digest.to_string()
    } else {
        release.installer = None;
        return Ok(release);
    };
    let mirror_url = if let Some(mirrors) = github_asset(&json, "mirrors.json")? {
        let url = canonical_asset_url(mirrors, repository, tag, "mirrors.json")?;
        mirror(&fetch(url, false)?, tag, &name)?
    } else {
        None
    };
    release.installer = Some(validate_asset(InstallerAsset {
        url: url.into(),
        sha256,
        size,
        file_name: Some(name),
        mirror_url,
    })?);
    Ok(release)
}

pub(crate) fn verify_installer(
    path: &std::path::Path,
    asset: &InstallerAsset,
) -> Result<(), String> {
    use sha2::{Digest, Sha256};
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path).map_err(|_| "无法打开下载的安装包。")?;
    if file.metadata().map_err(|_| "无法读取安装包信息。")?.len() != asset.size {
        return Err("安装包大小不一致，已停止安装。".into());
    }
    let mut header = [0u8; 64];
    file.read_exact(&mut header)
        .map_err(|_| "安装包内容不完整。")?;
    if &header[..2] != b"MZ" {
        return Err("下载结果不是 Windows 安装程序，可能是网盘网页。".into());
    }
    let offset = u32::from_le_bytes(header[60..64].try_into().unwrap()) as u64;
    if offset < 64 || offset > asset.size.saturating_sub(4) {
        return Err("安装程序格式无效。".into());
    }
    file.seek(SeekFrom::Start(offset))
        .map_err(|_| "安装程序格式无效。")?;
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)
        .map_err(|_| "安装程序格式无效。")?;
    if &magic != b"PE\0\0" {
        return Err("安装程序格式无效。".into());
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|_| "无法校验安装程序。")?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(|_| "无法校验安装程序。")?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    if format!("{:x}", hash.finalize()) != asset.sha256.to_ascii_lowercase() {
        return Err("安装包 SHA-256 不一致，已停止安装，请重新检查更新。".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn official(version: &str, no_lan: bool) -> serde_json::Value {
        let name = format!(
            "zsclip-v{version}-setup{}.exe",
            if no_lan { "-no-lan" } else { "" }
        );
        let asset = |name: &str| serde_json::json!({"name":name,"size":4096,"browser_download_url":format!("https://github.com/qiu7824/zsclip/releases/download/{version}/{name}")});
        serde_json::json!({"tag_name":version,"html_url":format!("https://github.com/qiu7824/zsclip/releases/tag/{version}"),"assets":[asset(&name),asset("SHA256SUMS.txt"),asset("mirrors.json")]})
    }

    #[test]
    fn official_release_selects_exact_variant_and_checksums_before_mirror() {
        for no_lan in [false, true] {
            let name = format!(
                "zsclip-v1.0.7-setup{}.exe",
                if no_lan { "-no-lan" } else { "" }
            );
            let mut requests = Vec::new();
            let release=github_release(official("1.0.7",no_lan).to_string().as_bytes(),no_lan,"https://github.com/qiu7824/zsclip/",&mut |url,post| {
                assert!(!post);requests.push(url.to_string());
                Ok(if url.ends_with("SHA256SUMS.txt") {format!("{} *{name}\n", "AB".repeat(32)).into_bytes()}
                else {serde_json::json!({"version":"v1.0.7","files":[{"name":name,"share_url":"https://www.ilanzou.com/s/public"}]}).to_string().into_bytes()})
            }).unwrap();
            let asset = release.installer.unwrap();
            assert!(asset.url.starts_with("https://github.com/"));
            assert_eq!(asset.sha256, "ab".repeat(32));
            assert_eq!(asset.size, 4096);
            assert_eq!(asset.file_name.as_deref(), Some(name.as_str()));
            assert_eq!(
                asset.mirror_url.as_deref(),
                Some("https://www.ilanzou.com/s/public")
            );
            assert!(requests[0].ends_with("SHA256SUMS.txt"));
            assert!(requests[1].ends_with("mirrors.json"));
        }
    }

    #[test]
    fn official_release_rejects_substitution_duplicate_hashes_and_wrong_mirror_version() {
        let mut release = official("1.0.7", false);
        release["assets"][0]["browser_download_url"] =
            serde_json::json!("https://example.com/setup.exe");
        assert!(github_release(
            release.to_string().as_bytes(),
            false,
            "https://github.com/qiu7824/zsclip",
            &mut |_, _| panic!("foreign asset cannot be fetched")
        )
        .is_err());
        let name = "zsclip-v1.0.7-setup.exe";
        let line = format!("{}  {name}\n", "ab".repeat(32));
        assert!(checksum(format!("{line}{line}").as_bytes(), name).is_err());
        assert!(checksum(format!("{}  other.exe\n", "ab".repeat(32)).as_bytes(), name).is_err());
        assert!(mirror(br#"{"version":"1.0.6","files":[]}"#, "1.0.7", name).is_err());
        assert!(mirror(br#"{"version":"1.0.7","files":[]}"#, "1.0.7", name)
            .unwrap()
            .is_none());
        release = official("1.0.7", false);
        release["assets"][0]["digest"] = serde_json::json!(format!("sha256:{}", "cd".repeat(32)));
        assert!(github_release(
            release.to_string().as_bytes(),
            false,
            "https://github.com/qiu7824/zsclip",
            &mut |_, _| Ok(line.as_bytes().to_vec())
        )
        .unwrap_err()
        .contains("不一致"));
    }

    #[test]
    fn legacy_github_digest_and_custom_manifest_remain_supported() {
        let mut release = official("1.0.7", false);
        release["assets"].as_array_mut().unwrap().truncate(1);
        release["assets"][0]["digest"] = serde_json::json!(format!("sha256:{}", "ab".repeat(32)));
        assert!(github_release(
            release.to_string().as_bytes(),
            false,
            "https://github.com/qiu7824/zsclip",
            &mut |_, _| panic!("no extra metadata needed")
        )
        .unwrap()
        .installer
        .is_some());
        assert!(github_release(
            release.to_string().as_bytes(),
            true,
            "https://github.com/qiu7824/zsclip",
            &mut |_, _| panic!("never switch variants")
        )
        .unwrap()
        .installer
        .is_none());
    }
    #[test]
    fn source_and_release_reject_management_pages_and_non_executable_metadata() {
        assert!(validate_source(&UpdateSource::default()).is_ok());
        for url in [
            "http://example.com/v.json",
            "file:///tmp/update",
            "https://user:secret@example.com/v",
            "https://example.com\\@evil.test",
            "https://www.ilanzou.com/console/files/0",
        ] {
            assert!(validate_source(&UpdateSource {
                manifest_url: url.into()
            })
            .is_err());
        }
        let body=serde_json::json!({"format":"zsclip-update-v1","version":"1.0.6","page_url":"https://www.ilanzou.com/s/public","notes":"更新内容","windows_x64":{"url":"https://download.example.com/setup.exe","sha256":"ab".repeat(32),"size":4096}}).to_string();
        let release = parse_release(body.as_bytes(), true, false).unwrap();
        assert_eq!(release.notes, "更新内容");
        assert!(release.installer.is_some());
        assert!(parse_release(body.as_bytes(), true, true)
            .unwrap()
            .installer
            .is_none());
        assert!(parse_release(br#"{"message":"Not Found"}"#, false, false).is_err());
        assert!(newer("1.0.6", "1.0.5"));
        assert!(!newer("1.0.5.0", "1.0.5"));
        assert!(!newer("1.0.6;run", "1.0.5"));
    }
    #[test]
    fn downloaded_html_tampering_and_wrong_size_never_pass_install_validation() {
        use sha2::{Digest, Sha256};
        let base = std::env::var_os("ZSCLIP_TEST_TEMP_ROOT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let path = base.join(format!(
            "zsclip-update-test-{}-{}.bin",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut bytes = vec![0u8; 4096];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[60..64].copy_from_slice(&128u32.to_le_bytes());
        bytes[128..132].copy_from_slice(b"PE\0\0");
        let asset = InstallerAsset {
            url: "https://example.com/setup.exe".into(),
            size: bytes.len() as u64,
            sha256: format!("{:x}", Sha256::digest(&bytes)),
            file_name: None,
            mirror_url: None,
        };
        std::fs::write(&path, &bytes).unwrap();
        assert!(verify_installer(&path, &asset).is_ok());
        bytes[2048] = 1;
        std::fs::write(&path, &bytes).unwrap();
        assert!(verify_installer(&path, &asset).is_err());
        bytes[..2].copy_from_slice(b"<h");
        std::fs::write(&path, &bytes).unwrap();
        assert!(verify_installer(&path, &asset).is_err());
        std::fs::write(&path, b"short").unwrap();
        assert!(verify_installer(&path, &asset).is_err());
        std::fs::remove_file(path).unwrap();
    }
}
