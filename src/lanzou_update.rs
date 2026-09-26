//! Anonymous public-share reader. Account cookies and login gates are never bypassed.
use aes_gcm::aes::{
    cipher::{generic_array::GenericArray, BlockEncrypt, KeyInit},
    Aes128,
};
use serde_json::Value;

pub(crate) const DEFAULT_SHARE_URL: &str = "https://www.ilanzou.com/s/PyjrO6mP";
pub(crate) const MANIFEST_NAME: &str = "zsclip-update.json";

fn share_id(url: &str) -> Option<&str> {
    let rest = url
        .strip_prefix("https://www.ilanzou.com/s/")
        .or_else(|| url.strip_prefix("https://ilanzou.com/s/"))?;
    let id = rest.split(['?', '#']).next()?;
    (!id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_')))
    .then_some(id)
}
pub(crate) fn is_share_url(url: &str) -> bool {
    share_id(url).is_some()
}

// Public website framing, not an account key or authentication credential.
fn site_hex(text: &str) -> String {
    let cipher = Aes128::new_from_slice(b"lanZouY-disk-app").unwrap();
    let mut bytes = text.as_bytes().to_vec();
    let padding = 16 - bytes.len() % 16;
    bytes.extend(std::iter::repeat_n(padding as u8, padding));
    for chunk in bytes.chunks_exact_mut(16) {
        cipher.encrypt_block(GenericArray::from_mut_slice(chunk));
    }
    bytes.iter().map(|b| format!("{b:02X}")).collect()
}
fn id_string(value: Option<&Value>) -> Option<String> {
    let value = value?;
    if let Some(id) = value.as_u64() {
        return Some(id.to_string());
    }
    value
        .as_str()
        .filter(|id| !id.is_empty() && id.len() <= 32 && id.bytes().all(|c| c.is_ascii_digit()))
        .map(str::to_string)
}
fn response(body: &[u8]) -> Result<Value, String> {
    if body.len() > crate::update_feed::MAX_MANIFEST_BYTES {
        return Err("蓝奏版本目录超过大小限制。".into());
    }
    let value: Value = serde_json::from_slice(body).map_err(|_| "蓝奏返回内容无法识别。")?;
    if value.get("code").and_then(Value::as_i64) != Some(200) {
        let code = value
            .get("code")
            .and_then(Value::as_i64)
            .map(|code| code.to_string())
            .unwrap_or_else(|| "未知".into());
        return Err(format!(
            "蓝奏公开接口未返回成功结果（代码 {code}），请打开分享页检查其状态。"
        ));
    }
    if let Some(status) = value
        .get("status")
        .and_then(Value::as_i64)
        .filter(|status| *status != 0)
    {
        return Err(format!(
            "蓝奏分享暂不可读取（状态 {status}），请打开公开分享页检查。"
        ));
    }
    Ok(value)
}
fn public_file_url(file: &Value, id: &str, uuid: &str, now: &str) -> Result<String, String> {
    // iconId classifies a filename; it does not describe authentication requirements.
    // The anonymous redirect endpoint decides whether this public file is downloadable.
    let file_id = id_string(file.get("fileId")).ok_or("蓝奏文件缺少有效标识。")?;
    Ok(format!("https://apix.ilanzou.com/unproved/file/redirect?downloadId={}&enable=0&devType=6&uuid={uuid}&shareId={id}&timestamp={}&auth={}",site_hex(&format!("{file_id}|")),site_hex(now),site_hex(&format!("{file_id}|{now}"))))
}

fn named_file<'a>(entries: &'a [Value], file_name: &str) -> Result<Option<&'a Value>, String> {
    let mut matches = entries
        .iter()
        .filter(|file| file.get("fileName").and_then(Value::as_str) == Some(file_name));
    let found = matches.next();
    if matches.next().is_some() {
        return Err(format!(
            "蓝奏分享中有多个同名文件 {file_name}，请只保留一个。"
        ));
    }
    Ok(found)
}

pub(crate) fn resolve_file<F: FnMut(&str, bool) -> Result<Vec<u8>, String> + ?Sized>(
    url: &str,
    file_name: &str,
    fetch: &mut F,
) -> Result<String, String> {
    let id = share_id(url).ok_or("蓝奏更新源需要公开分享链接。")?;
    if file_name.is_empty() || file_name.len() > 160 || file_name.contains(['/', '\\']) {
        return Err("更新文件名无效。".into());
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "系统时间无效。")?
        .as_millis()
        .to_string();
    let random = format!(
        "{:x}",
        md5::compute(format!(
            "{}:{}:{:?}",
            std::process::id(),
            now,
            std::time::Instant::now()
        ))
    );
    let uuid = format!(
        "{}-{}-4{}-8{}-{}",
        &random[..8],
        &random[8..12],
        &random[13..16],
        &random[17..20],
        &random[20..]
    );
    let common = format!(
        "devType=6&devModel=Chrome&uuid={uuid}&extra=2&timestamp={}&code=&shareId={id}",
        site_hex(&now)
    );
    let initial = response(&fetch(
        &format!(
            "https://apix.ilanzou.com/unproved/recommend/list?{common}&type=0&offset=1&limit=60"
        ),
        true,
    )?)?;
    let shares = initial
        .get("list")
        .and_then(Value::as_array)
        .ok_or("蓝奏分享不存在。")?;
    if shares.is_empty() {
        return Err("蓝奏分享尚未返回公开文件，请检查版本清单和安装包是否已发布。".into());
    }
    if shares.len() != 1 {
        return Err("蓝奏更新源需要一个明确的公开文件或目录。".into());
    }
    let share = &shares[0];
    if share
        .get("code")
        .and_then(Value::as_str)
        .is_some_and(|code| !code.is_empty())
        || share
            .get("amt")
            .and_then(Value::as_f64)
            .is_some_and(|amount| amount > 0.0)
    {
        return Err("蓝奏分享需要提取码或其它授权，请使用无需登录的公开更新源。".into());
    }
    let entries = share
        .get("fileList")
        .and_then(Value::as_array)
        .ok_or("蓝奏分享中没有文件。")?;
    if let Some(file) = named_file(entries, file_name)? {
        return public_file_url(file, id, &uuid, &now);
    }
    if entries.len() != 1 {
        return Err("请将版本信息与安装包放在同一个公开分享文件夹。".into());
    }
    let folder = id_string(entries[0].get("folderId")).ok_or("蓝奏分享未提供更新文件夹。")?;
    let mut found = None;
    for page in 1..=4 {
        let reply=response(&fetch(&format!("https://apix.ilanzou.com/unproved/share/list?{common}&folderId={folder}&offset={page}&limit=60"),true)?)?;
        let files = reply
            .get("list")
            .and_then(Value::as_array)
            .ok_or("蓝奏目录格式无效。")?;
        if let Some(file) = named_file(files, file_name)? {
            if found.is_some() {
                return Err(format!(
                    "蓝奏分享中有多个同名文件 {file_name}，请只保留一个。"
                ));
            }
            found = Some(public_file_url(file, id, &uuid, &now)?);
        }
        if files.len() < 60 {
            return found.ok_or_else(|| {
                format!("蓝奏公开文件夹中未找到 {file_name}，请先发布版本信息和对应安装包。")
            });
        }
    }
    Err("蓝奏更新目录超过 240 项，请为更新文件使用独立目录。".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn folder() -> Vec<u8> {
        serde_json::json!({"code":200,"status":0,"list":[{"type":1,"code":"","fileList":[{"folderId":394681765}]}]}).to_string().into_bytes()
    }
    #[test]
    fn public_reader_is_bounded_to_the_supplied_share_and_handles_empty_folder() {
        let mut calls = Vec::new();
        let result = resolve_file(DEFAULT_SHARE_URL, MANIFEST_NAME, &mut |url, post| {
            assert!(post);
            assert!(url.contains("shareId=PyjrO6mP"));
            assert!(!url.contains("appToken"));
            calls.push(url.to_string());
            Ok(if calls.len() == 1 {
                folder()
            } else {
                br#"{"code":200,"list":[],"total":0}"#.to_vec()
            })
        });
        assert!(result.unwrap_err().contains(MANIFEST_NAME));
        assert_eq!(calls.len(), 2);
        assert!(!is_share_url("https://www.ilanzou.com/console/files/0"));
        assert!(!is_share_url(
            "https://www.ilanzou.com.evil.test/s/PyjrO6mP"
        ));
    }
    #[test]
    fn executable_icons_use_the_public_redirect_protocol() {
        let mut count = 0;
        let result = resolve_file(DEFAULT_SHARE_URL, "zsclip-v1.0.6-setup.exe", &mut |_, _| {
            count += 1;
            Ok(if count == 1 {
                folder()
            } else {
                serde_json::json!({"code":200,"list":[{"fileName":"zsclip-v1.0.6-setup.exe","fileId":123,"iconId":13,"type":1}]}).to_string().into_bytes()
            })
        });
        assert!(result.unwrap().contains("enable=0"));
        assert_eq!(count, 2);
        let open = public_file_url(
            &serde_json::json!({"fileId":123,"iconId":1,"type":1}),
            "PyjrO6mP",
            "anonymous",
            "1720000000000",
        )
        .unwrap();
        assert!(open.starts_with("https://apix.ilanzou.com/unproved/file/redirect?"));
        assert!(open.contains("shareId=PyjrO6mP"));
        assert!(!open.contains("userId="));
        assert_eq!(
            site_hex("1789290000000"),
            "F16A22A15ABAECC6326021AECF7FA260"
        );
    }

    #[test]
    fn single_file_shares_and_explicit_gates_are_distinct() {
        let file = serde_json::json!({"fileName":"setup.exe","fileId":"123","iconId":13});
        let mut share = serde_json::json!({"code":200,"list":[{"fileList":[file]}]});
        let result = resolve_file(
            "https://www.ilanzou.com/s/public-id_1",
            "setup.exe",
            &mut |_, _| Ok(share.to_string().into_bytes()),
        )
        .unwrap();
        assert!(result.contains("shareId=public-id_1"));
        share["list"][0]["amt"] = serde_json::json!(1);
        assert!(
            resolve_file(DEFAULT_SHARE_URL, "setup.exe", &mut |_, _| Ok(share
                .to_string()
                .into_bytes()))
            .unwrap_err()
            .contains("授权")
        );
        share["list"][0]["amt"] = serde_json::json!(0);
        let duplicate = share["list"][0]["fileList"][0].clone();
        share["list"][0]["fileList"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(
            resolve_file(DEFAULT_SHARE_URL, "setup.exe", &mut |_, _| Ok(share
                .to_string()
                .into_bytes()))
            .unwrap_err()
            .contains("同名")
        );
    }
}
