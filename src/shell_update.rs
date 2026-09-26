//! Explicit, user-initiated updates from bounded public HTTPS metadata.
use super::{update_check_state, UpdateCheckState, APP_VERSION};
use crate::update_feed::{self, InstallerAsset, ReleaseInfo, UpdateSource};
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

static JOB_SEQUENCE: AtomicU64 = AtomicU64::new(1);
const FILE_SHARE_READ_ONLY: u32 = 1;
const CONFIG_LIMIT: u64 = 16 * 1024;

fn source_file() -> PathBuf {
    crate::app::runtime::data_dir().join("update_source.json")
}

fn read_source(path: &Path) -> Result<UpdateSource, String> {
    match File::open(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(UpdateSource::default()),
        Err(_) => Err("无法读取更新源设置。".into()),
        Ok(file) => {
            let mut bytes = Vec::new();
            file.take(CONFIG_LIMIT + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| "无法读取更新源设置。")?;
            if bytes.len() as u64 > CONFIG_LIMIT {
                return Err("更新源设置文件过大。".into());
            }
            let source: UpdateSource =
                serde_json::from_slice(&bytes).map_err(|_| "更新源设置格式无效，请重新设置。")?;
            update_feed::validate_source(&source)?;
            Ok(source)
        }
    }
}

pub(crate) fn update_source() -> Result<UpdateSource, String> {
    read_source(&source_file())
}

fn write_source(path: &Path, source: &UpdateSource) -> Result<(), String> {
    let parent = path.parent().ok_or("更新源保存目录无效。")?;
    fs::create_dir_all(parent).map_err(|_| "无法创建更新源保存目录。")?;
    let temporary = parent.join(format!(
        ".update-source-{}-{}.part",
        std::process::id(),
        JOB_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|_| "无法保存更新源设置。")?;
        let bytes = serde_json::to_vec_pretty(source).map_err(|_| "无法保存更新源设置。")?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| "无法保存更新源设置。")?;
        drop(file);
        fs::rename(&temporary, path).map_err(|_| "无法保存更新源设置。")?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

pub(crate) fn set_update_source(mut source: UpdateSource) -> Result<(), String> {
    source.manifest_url = source.manifest_url.trim().to_string();
    update_feed::validate_source(&source)?;
    let mut state = update_check_state()
        .lock()
        .map_err(|_| "更新状态暂不可用。")?;
    if state.checking || state.downloading || state.install_started {
        return Err("更新正在进行，请完成后再更改更新源。".into());
    }
    write_source(&source_file(), &source)?;
    *state = UpdateCheckState::default();
    Ok(())
}

fn github_api() -> Result<String, String> {
    let repository = super::open_source_url().trim().trim_end_matches('/');
    let path = repository
        .strip_prefix("https://github.com/")
        .ok_or("未配置可用的 GitHub 更新仓库。")?;
    if path.split('/').count() != 2 {
        return Err("GitHub 更新仓库地址无效。".into());
    }
    Ok(format!(
        "https://api.github.com/repos/{path}/releases/latest"
    ))
}

fn check_with(
    source: &UpdateSource,
    no_lan: bool,
    fetch: &mut dyn FnMut(&str, bool) -> Result<Vec<u8>, String>,
) -> Result<ReleaseInfo, String> {
    update_feed::validate_source(source)?;
    let custom = !source.manifest_url.trim().is_empty();
    let mut url = if custom {
        source.manifest_url.clone()
    } else {
        github_api()?
    };
    if crate::lanzou_update::is_share_url(&url) {
        url = crate::lanzou_update::resolve_file(&url, crate::lanzou_update::MANIFEST_NAME, fetch)?;
    }
    if !update_feed::https_url(&url) {
        return Err("版本信息下载地址不是公开 HTTPS 地址。".into());
    }
    let body = fetch(&url, false)?;
    if custom {
        update_feed::parse_release(&body, true, no_lan)
    } else {
        update_feed::github_release(&body, no_lan, super::open_source_url(), fetch)
    }
}

fn system_curl() -> Result<PathBuf, String> {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetSystemDirectoryW(buffer: *mut u16, size: u32) -> u32;
    }
    let mut buffer = vec![0u16; 32768];
    let length = unsafe { GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) } as usize;
    if length == 0 || length >= buffer.len() {
        return Err("系统下载组件不可用。".into());
    }
    Ok(PathBuf::from(OsString::from_wide(&buffer[..length])).join("curl.exe"))
}

fn curl_arguments(
    url: &str,
    post: bool,
    maximum: u64,
    seconds: u32,
    json: bool,
) -> Result<Vec<OsString>, String> {
    if !update_feed::https_url(url) {
        return Err("更新下载仅支持公开 HTTPS 地址。".into());
    }
    let mut args: Vec<OsString> = [
        "--disable",
        "--silent",
        "--show-error",
        "--location",
        "--max-redirs",
        "3",
        "--proto",
        "=https",
        "--proto-redir",
        "=https",
        "--connect-timeout",
        "10",
        "--max-time",
    ]
    .into_iter()
    .map(OsString::from)
    .collect();
    args.push(seconds.to_string().into());
    args.push("--max-filesize".into());
    args.push(maximum.to_string().into());
    args.push("--user-agent".into());
    args.push(format!("ZSClip/{APP_VERSION}").into());
    if json {
        args.extend([
            "--fail".into(),
            "--compressed".into(),
            "--header".into(),
            "Accept: application/json".into(),
        ]);
    }
    // Keep small error bodies for file requests so explicit public-share login,
    // captcha and payment refusals can be reported. Every file is still bounded
    // and must pass the release's exact size, SHA-256 and PE checks before launch.
    let request_url = if post {
        // The ilanzou public API expects URL-encoded form fields in the body.
        // Keep the callback URL representation so deterministic tests need no HTTP client.
        let (endpoint, form) = url.split_once('?').ok_or("公开分享请求缺少表单参数。")?;
        if !matches!(
            endpoint,
            "https://apix.ilanzou.com/unproved/recommend/list"
                | "https://apix.ilanzou.com/unproved/share/list"
        ) {
            return Err("不支持的公开分享接口。".into());
        }
        args.extend([
            "--header".into(),
            "Content-Type: application/x-www-form-urlencoded".into(),
            "--header".into(),
            "Origin: https://www.ilanzou.com".into(),
            "--referer".into(),
            "https://www.ilanzou.com/".into(),
            "--data-raw".into(),
            form.into(),
        ]);
        endpoint
    } else {
        url
    };
    args.push("--".into());
    args.push(request_url.into());
    Ok(args)
}

fn curl_command(
    url: &str,
    post: bool,
    maximum: u64,
    seconds: u32,
    json: bool,
) -> Result<Command, String> {
    let mut command = super::hidden_command_path(&system_curl()?);
    command
        .args(curl_arguments(url, post, maximum, seconds, json)?)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    Ok(command)
}

fn request_failed(code: Option<i32>) -> String {
    match code {
        Some(28) => "连接更新源超时，请稍后重试。",
        Some(22) => "更新源未返回可用文件，请检查发布内容或访问权限。",
        Some(60) | Some(51) => "更新源 HTTPS 证书验证失败。",
        Some(63) => "更新文件超过允许的大小。",
        _ => "无法从更新源下载，请检查网络或更换更新源。",
    }
    .into()
}

fn public_fetch(url: &str, post: bool) -> Result<Vec<u8>, String> {
    let limit = update_feed::MAX_MANIFEST_BYTES;
    let mut child = curl_command(url, post, limit as u64, 30, true)?
        .spawn()
        .map_err(|_| "无法启动系统下载组件。")?;
    let mut bytes = Vec::new();
    let read = child
        .stdout
        .take()
        .ok_or("系统下载组件未返回数据。")?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes);
    if read.is_err() || bytes.len() > limit {
        let _ = child.kill();
        let _ = child.wait();
        return Err(if bytes.len() > limit {
            "更新源返回的数据超过大小限制。"
        } else {
            "读取更新源失败。"
        }
        .into());
    }
    let status = child.wait().map_err(|_| "系统下载组件异常结束。")?;
    if !status.success() {
        return Err(request_failed(status.code()));
    }
    Ok(bytes)
}

pub(crate) fn start_update_check<F>(notify: F)
where
    F: FnOnce() + Send + 'static,
{
    {
        let Ok(mut state) = update_check_state().lock() else {
            return;
        };
        if state.checking || state.downloading || state.install_started {
            return;
        }
        *state = UpdateCheckState {
            started: true,
            checking: true,
            ..Default::default()
        };
    }
    std::thread::spawn(move || {
        let result = update_source()
            .and_then(|source| check_with(&source, !cfg!(feature = "lan-sync"), &mut public_fetch));
        let mut next = UpdateCheckState {
            started: true,
            latest_url: super::latest_release_url(),
            ..Default::default()
        };
        match result {
            Ok(release) => {
                next.available = update_feed::newer(&release.version, APP_VERSION);
                next.latest_tag = release.version;
                next.latest_url = release.page_url;
                next.release_notes = release.notes;
                next.installer = release.installer;
            }
            Err(error) => next.error = error,
        }
        if let Ok(mut state) = update_check_state().lock() {
            *state = next;
        }
        notify();
    });
}

fn create_job(base: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(base).map_err(|_| "无法创建更新下载目录。")?;
    for _ in 0..8 {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let directory = base.join(format!(
            "{}-{stamp}-{}",
            std::process::id(),
            JOB_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        match fs::create_dir(&directory) {
            Ok(()) => return Ok(directory),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err("无法创建独立的更新下载目录。".into()),
        }
    }
    Err("无法分配更新下载目录。".into())
}

fn copy_exact<R: Read>(
    reader: &mut R,
    file: &mut File,
    expected: u64,
    progress: &mut dyn FnMut(u64),
) -> Result<(), String> {
    if expected < 4096 || expected > update_feed::MAX_INSTALLER_BYTES {
        return Err("安装包大小无效。".into());
    }
    let mut total = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    let mut notified = Instant::now();
    loop {
        let count = reader.read(&mut buffer).map_err(|_| "安装包下载中断。")?;
        if count == 0 {
            break;
        }
        total = total.checked_add(count as u64).ok_or("安装包大小无效。")?;
        if total > expected {
            return Err("下载内容超过发布的安装包大小，已停止下载。".into());
        }
        file.write_all(&buffer[..count])
            .map_err(|_| "无法写入安装包，请检查磁盘空间。")?;
        if notified.elapsed().as_millis() >= 250 || total == expected {
            progress(total);
            notified = Instant::now();
        }
    }
    if total != expected {
        return Err("安装包下载不完整，已停止安装。".into());
    }
    file.sync_all().map_err(|_| "无法保存安装包。")?;
    Ok(())
}

fn download_file(
    url: &str,
    path: &Path,
    size: u64,
    progress: &mut dyn FnMut(u64),
) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .share_mode(FILE_SHARE_READ_ONLY)
        .open(path)
        .map_err(|_| "无法创建安装包临时文件。")?;
    let mut child = curl_command(url, false, size, 240, false)?
        .spawn()
        .map_err(|_| "无法启动系统下载组件。")?;
    let result = match child.stdout.take() {
        Some(mut stdout) => copy_exact(&mut stdout, &mut file, size, progress),
        None => Err("系统下载组件未返回安装包。".into()),
    };
    if result.is_err() {
        let _ = child.kill();
    }
    let status = child.wait().map_err(|_| "系统下载组件异常结束。")?;
    let result = if !status.success() {
        Err(request_failed(status.code()))
    } else {
        result
    };
    result.map_err(|error| public_download_error(url, path).unwrap_or(error))
}

fn public_download_error(url: &str, path: &Path) -> Option<String> {
    if !url.starts_with("https://apix.ilanzou.com/unproved/file/redirect?") {
        return None;
    }
    let file = File::open(path).ok()?;
    if file.metadata().ok()?.len() > 16 * 1024 {
        return None;
    }
    let mut body = Vec::new();
    file.take(16 * 1024 + 1).read_to_end(&mut body).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&body).ok()?;
    let message = value
        .get("msg")
        .or_else(|| value.get("message"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_lowercase();
    for (words, description) in [
        (&["登录", "login", "log in"][..], "登录"),
        (&["验证码", "人机", "captcha"][..], "人工验证"),
        (&["付费", "支付", "payment"][..], "付费授权"),
        (&["提取码", "password"][..], "提取码"),
    ] {
        if words.iter().any(|word| message.contains(word)) {
            return Some(format!(
                "蓝奏服务器要求{description}，请打开分享页完成平台要求后手动下载。"
            ));
        }
    }
    let code = value.get("code").and_then(|v| v.as_i64())?;
    Some(format!(
        "蓝奏公开下载接口未返回文件（代码 {code}），请打开分享页检查发布状态。"
    ))
}

struct VerifiedInstaller {
    path: PathBuf,
    _read_lock: File,
}

fn prepare_verified(
    asset: &InstallerAsset,
    directory: &Path,
    download: &mut dyn FnMut(&Path) -> Result<(), String>,
) -> Result<VerifiedInstaller, String> {
    let part = directory.join("installer.part");
    let final_path = directory.join("installer.exe");
    let result = (|| {
        download(&part)?;
        update_feed::verify_installer(&part, asset)?;
        fs::rename(&part, &final_path).map_err(|_| "无法准备已校验的安装包。")?;
        let lock = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ_ONLY)
            .open(&final_path)
            .map_err(|_| "无法锁定已下载的安装包。")?;
        // Recheck while this read-only handle denies writes and deletion through launch.
        update_feed::verify_installer(&final_path, asset)?;
        Ok(VerifiedInstaller {
            path: final_path,
            _read_lock: lock,
        })
    })();
    if result.is_err() {
        let _ = fs::remove_file(&part);
    }
    result
}

fn installer_parameters(directory: &Path) -> Result<Vec<u16>, String> {
    if !directory.is_absolute() {
        return Err("当前程序目录无效。".into());
    }
    let path: Vec<u16> = directory.as_os_str().encode_wide().collect();
    if path.is_empty() || path.iter().any(|c| *c == 0 || *c == b'"' as u16 || *c < 32) {
        return Err("当前程序目录无法用于更新安装。".into());
    }
    let mut parameters: Vec<u16> = "/ZSCLIPUPDATE=1 /SILENT /SP- /NORESTART /DIR=\""
        .encode_utf16()
        .collect();
    parameters.extend(&path);
    // Escape a root/trailing backslash before the closing Windows command-line quote.
    let trailing = path
        .iter()
        .rev()
        .take_while(|c| **c == b'\\' as u16)
        .count();
    parameters.extend(std::iter::repeat_n(b'\\' as u16, trailing));
    parameters.extend([b'"' as u16, 0]);
    Ok(parameters)
}

struct InstallerProcess {
    handle: windows_sys::Win32::Foundation::HANDLE,
}

impl Drop for InstallerProcess {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(self.handle);
            }
        }
    }
}

impl InstallerProcess {
    fn wait(self) -> Result<u32, String> {
        use windows_sys::Win32::{
            Foundation::WAIT_OBJECT_0,
            System::Threading::{GetExitCodeProcess, WaitForSingleObject, INFINITE},
        };
        unsafe {
            if WaitForSingleObject(self.handle, INFINITE) != WAIT_OBJECT_0 {
                return Err("无法监测安装程序状态，请确认安装窗口已关闭后重试。".into());
            }
            let mut code = 0;
            if GetExitCodeProcess(self.handle, &mut code) == 0 {
                return Err("无法读取安装程序结果，请确认安装窗口已关闭后重试。".into());
            }
            Ok(code)
        }
    }
}

fn launch_installer(
    installer: &VerifiedInstaller,
    directory: &Path,
) -> Result<InstallerProcess, String> {
    use windows_sys::Win32::{
        Foundation::GetLastError,
        System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED},
        UI::{
            Shell::{
                ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS,
                SHELLEXECUTEINFOW,
            },
            WindowsAndMessaging::SW_SHOWNORMAL,
        },
    };
    let file: Vec<u16> = installer
        .path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let work: Vec<u16> = directory.as_os_str().encode_wide().chain(Some(0)).collect();
    let parameters = installer_parameters(directory)?;
    let verb: Vec<u16> = "open\0".encode_utf16().collect();
    unsafe {
        let initialized = CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32);
        let mut execute: SHELLEXECUTEINFOW = std::mem::zeroed();
        execute.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
        execute.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI;
        execute.lpVerb = verb.as_ptr();
        execute.lpFile = file.as_ptr();
        execute.lpParameters = parameters.as_ptr();
        execute.lpDirectory = work.as_ptr();
        execute.nShow = SW_SHOWNORMAL;
        let success = ShellExecuteExW(&mut execute) != 0;
        let code = if success { 0 } else { GetLastError() };
        let process = InstallerProcess {
            handle: execute.hProcess,
        };
        if initialized >= 0 {
            CoUninitialize();
        }
        if success && !process.handle.is_null() {
            Ok(process)
        } else if success {
            Err("安装程序未返回可监测的进程，请确认安装窗口已关闭后重试。".into())
        } else if code == 1223 {
            Err("已取消安装，当前程序保持运行。".into())
        } else {
            Err("无法启动安装程序，当前程序保持运行。".into())
        }
    }
}

fn resolve_installer_url(
    asset: &InstallerAsset,
    fetch: &mut dyn FnMut(&str, bool) -> Result<Vec<u8>, String>,
) -> Result<String, String> {
    if crate::lanzou_update::is_share_url(&asset.url) {
        let name = asset
            .file_name
            .as_deref()
            .filter(|name| !name.is_empty())
            .ok_or("蓝奏安装包信息缺少 file_name 文件名。")?;
        crate::lanzou_update::resolve_file(&asset.url, name, fetch)
    } else {
        Ok(asset.url.clone())
    }
}

fn download_asset(
    asset: &InstallerAsset,
    part: &Path,
    fetch: &mut dyn FnMut(&str, bool) -> Result<Vec<u8>, String>,
    download: &mut dyn FnMut(&str, &Path) -> Result<(), String>,
) -> Result<(), String> {
    let mut mirror_error = None;
    if let Some(mirror) = &asset.mirror_url {
        let name = asset.file_name.as_deref().ok_or("镜像安装包缺少文件名。")?;
        let result = crate::lanzou_update::resolve_file(mirror, name, fetch)
            .and_then(|url| download(&url, part))
            .and_then(|()| update_feed::verify_installer(part, asset));
        match result {
            Ok(()) => return Ok(()),
            Err(error) => mirror_error = Some(error),
        }
        // A mirror can disappear or refuse anonymous access. The canonical HTTPS asset
        // is a separate public source and must pass the identical hash and size checks.
        if part.exists() {
            fs::remove_file(part).map_err(|_| "无法清理未完成的镜像下载。")?;
        }
        set_download_status("蓝奏线路不可用，正在尝试官方发布线路", Some(0));
    }
    let url = resolve_installer_url(asset, fetch)?;
    if !update_feed::https_url(&url) {
        return Err("安装包下载地址不是公开 HTTPS 地址。".into());
    }
    download(&url, part).map_err(|error| match mirror_error {
        Some(mirror_error) => format!("蓝奏线路：{mirror_error}；官方发布线路：{error}"),
        None => error,
    })
}

fn mark_install_started(state: &mut UpdateCheckState) {
    state.downloading = false;
    state.install_started = true;
    state.download_status = "安装程序已启动".into();
}

fn finish_update_attempt(state: &mut UpdateCheckState, result: Result<u32, String>) {
    state.downloading = false;
    state.install_started = false;
    state.download_status.clear();
    state.error = match result {
        Ok(0) => {
            "安装程序已退出，当前程序仍在运行，尚未确认更新完成；请重新启动程序或重试安装。".into()
        }
        Ok(2 | 5) => "已取消安装，当前程序保持运行，可重试安装。".into(),
        Ok(code) => {
            format!("安装程序未完成更新（退出代码 {code}），当前程序保持运行，可重试安装。")
        }
        Err(error) => error,
    };
}

fn set_download_status(status: &str, downloaded: Option<u64>) {
    if let Ok(mut state) = update_check_state().lock() {
        state.download_status = status.into();
        if let Some(downloaded) = downloaded {
            state.downloaded = downloaded;
        }
    }
}

pub(crate) fn start_update_download<F>(notify: F) -> Result<(), String>
where
    F: Fn() + Send + Sync + 'static,
{
    let asset = {
        let mut state = update_check_state()
            .lock()
            .map_err(|_| "更新状态暂不可用。")?;
        if state.checking || state.downloading || state.install_started {
            return Err("更新正在进行。".into());
        }
        if !state.available {
            return Err("请先检查更新。".into());
        }
        let asset = state
            .installer
            .clone()
            .ok_or("该版本未提供可校验的安装包，请查看发布页面。")?;
        state.downloading = true;
        state.downloaded = 0;
        state.total = asset.size;
        state.error.clear();
        state.download_status = "解析下载地址".into();
        asset
    };
    std::thread::spawn(move || {
        let result = (|| {
            let executable = std::env::current_exe().map_err(|_| "无法确定当前程序目录。")?;
            let directory = executable.parent().ok_or("无法确定当前程序目录。")?;
            let _ = installer_parameters(directory)?;
            let job = create_job(&crate::app::runtime::data_dir().join("temp").join("updates"))?;
            set_download_status("正在下载", Some(0));
            notify();
            let verified = prepare_verified(&asset, &job, &mut |part| {
                download_asset(&asset, part, &mut public_fetch, &mut |url, part| {
                    download_file(url, part, asset.size, &mut |size| {
                        set_download_status("正在下载", Some(size));
                        notify();
                    })
                })?;
                set_download_status("校验安装包", Some(asset.size));
                notify();
                Ok(())
            })?;
            set_download_status("启动安装", Some(asset.size));
            notify();
            // `verified` keeps its non-writable/non-deletable read handle through this call.
            launch_installer(&verified, directory)
        })();
        let result = match result {
            Ok(process) => {
                if let Ok(mut state) = update_check_state().lock() {
                    mark_install_started(&mut state);
                }
                notify();
                // Only this worker waits. If installation closes this app, the process exits;
                // otherwise cancellation and failure restore the existing app's retry controls.
                process.wait()
            }
            Err(error) => Err(error),
        };
        if let Ok(mut state) = update_check_state().lock() {
            finish_update_attempt(&mut state, result);
        }
        notify();
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn temp_root() -> PathBuf {
        let base = std::env::var_os("ZSCLIP_TEST_TEMP_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        create_job(&base.join("updater-runtime-tests")).unwrap()
    }
    fn synthetic_pe() -> Vec<u8> {
        let mut bytes = vec![0u8; 4096];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[60..64].copy_from_slice(&128u32.to_le_bytes());
        bytes[128..132].copy_from_slice(b"PE\0\0");
        bytes
    }
    fn asset(bytes: &[u8]) -> InstallerAsset {
        use sha2::{Digest, Sha256};
        InstallerAsset {
            url: "https://downloads.example.com/setup.exe".into(),
            sha256: format!("{:x}", Sha256::digest(bytes)),
            size: bytes.len() as u64,
            file_name: None,
            mirror_url: None,
        }
    }
    #[test]
    fn updater_source_defaults_to_official_release_and_preserves_custom_source() {
        let root = temp_root();
        let path = root.join("update_source.json");
        assert!(read_source(&path).unwrap().manifest_url.is_empty());
        let existing = UpdateSource {
            manifest_url: crate::lanzou_update::DEFAULT_SHARE_URL.into(),
        };
        write_source(&path, &existing).unwrap();
        assert!(read_source(&path).unwrap() == existing);
        write_source(&path, &UpdateSource::default()).unwrap();
        assert!(read_source(&path).unwrap().manifest_url.is_empty());
        fs::remove_file(path).unwrap();
        fs::remove_dir(root).unwrap();
    }
    #[test]
    fn updater_damaged_source_can_be_replaced_with_a_valid_source() {
        let root = temp_root();
        let path = root.join("update_source.json");
        fs::write(&path, b"{").unwrap();
        assert!(read_source(&path).is_err());
        let source = UpdateSource {
            manifest_url: "https://updates.example.com/release.json".into(),
        };
        write_source(&path, &source).unwrap();
        assert!(read_source(&path).unwrap() == source);
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        fs::remove_file(path).unwrap();
        fs::remove_dir(root).unwrap();
    }
    #[test]
    fn updater_direct_asset_filename_does_not_invoke_the_share_reader() {
        let mut asset = asset(&synthetic_pe());
        asset.file_name = Some("setup.exe".into());
        assert_eq!(
            resolve_installer_url(&asset, &mut |_, _| panic!(
                "direct download needs no share lookup"
            ))
            .unwrap(),
            asset.url,
        );
        asset.url = crate::lanzou_update::DEFAULT_SHARE_URL.into();
        for file_name in [None, Some(String::new())] {
            asset.file_name = file_name;
            let error = resolve_installer_url(&asset, &mut |_, _| {
                panic!("missing filename must fail before fetching")
            })
            .unwrap_err();
            assert!(error.contains("file_name"));
        }
    }
    #[test]
    fn updater_custom_source_uses_legacy_manifest_without_official_lookup() {
        let source = UpdateSource {
            manifest_url: "https://updates.example.com/zsclip-update.json".into(),
        };
        let body=serde_json::json!({"format":"zsclip-update-v1","version":"1.0.7","page_url":"https://example.com/releases","windows_x64":{"url":"https://www.ilanzou.com/s/public","sha256":"ab".repeat(32),"size":4096,"file_name":"setup.exe"}}).to_string();
        let release = check_with(&source, false, &mut |url, post| {
            assert_eq!(url, source.manifest_url);
            assert!(!post);
            Ok(body.as_bytes().to_vec())
        })
        .unwrap();
        assert_eq!(
            release.installer.unwrap().url,
            "https://www.ilanzou.com/s/public"
        );
    }
    #[test]
    fn updater_mirror_rejects_changed_bytes_and_falls_back_to_canonical_asset() {
        let root = temp_root();
        let path = root.join("installer.part");
        let bytes = synthetic_pe();
        let mut asset = asset(&bytes);
        asset.mirror_url = Some("https://www.ilanzou.com/s/public".into());
        asset.file_name = Some("setup.exe".into());
        let mut calls = Vec::new();
        download_asset(&asset,&path,&mut |_,_|Ok(serde_json::json!({"code":200,"list":[{"fileList":[{"fileName":"setup.exe","fileId":123,"iconId":13}]}]}).to_string().into_bytes()),&mut |url,path| {
            calls.push(url.to_string());
            let mut result=bytes.clone();
            if url.starts_with("https://apix.ilanzou.com/") {result[2000]=7;}
            fs::write(path,result).map_err(|e|e.to_string())
        }).unwrap();
        assert_eq!(calls.len(), 2);
        assert!(calls[0].contains("enable=0"));
        assert_eq!(calls[1], asset.url);
        update_feed::verify_installer(&path, &asset).unwrap();
        fs::remove_file(path).unwrap();
        fs::remove_dir(root).unwrap();
    }
    #[test]
    fn updater_reports_explicit_server_gates_without_guessing_from_file_type() {
        let root = temp_root();
        let path = root.join("response.json");
        let url = "https://apix.ilanzou.com/unproved/file/redirect?enable=0";
        fs::write(&path, br#"{"code":401,"msg":"Please login"}"#).unwrap();
        assert!(public_download_error(url, &path).unwrap().contains("登录"));
        fs::write(&path, br#"{"code":503,"msg":"temporarily unavailable"}"#).unwrap();
        let error = public_download_error(url, &path).unwrap();
        assert!(error.contains("503") && !error.contains("登录"));
        assert!(public_download_error("https://example.com/setup.exe", &path).is_none());
        fs::remove_file(path).unwrap();
        fs::remove_dir(root).unwrap();
    }
    #[test]
    fn updater_installer_exit_and_monitor_failure_restore_retry_state() {
        for (result, expected) in [
            (Ok(0), "尚未确认更新完成"),
            (Ok(2), "已取消安装"),
            (Ok(5), "已取消安装"),
            (Ok(7), "退出代码 7"),
            (Err("监测失败".to_string()), "监测失败"),
        ] {
            let mut state = UpdateCheckState {
                available: true,
                downloading: true,
                latest_tag: "1.0.99".into(),
                installer: Some(asset(&synthetic_pe())),
                ..Default::default()
            };
            mark_install_started(&mut state);
            assert!(!state.downloading);
            assert!(state.install_started);
            finish_update_attempt(&mut state, result);
            assert!(!state.downloading && !state.install_started);
            assert!(state.available && state.installer.is_some());
            assert_eq!(state.latest_tag, "1.0.99");
            assert!(state.download_status.is_empty());
            assert!(state.error.contains(expected));
        }
        let error = InstallerProcess {
            handle: std::ptr::null_mut(),
        }
        .wait()
        .unwrap_err();
        assert!(error.contains("监测"));
    }
    #[test]
    fn updater_transport_is_https_only_bounded_and_ignores_curlrc() {
        let args = curl_arguments(
            "https://downloads.example.com/a?public=1",
            false,
            4096,
            30,
            true,
        )
        .unwrap();
        let args: Vec<_> = args.iter().map(|arg| arg.to_string_lossy()).collect();
        assert_eq!(args[0], "--disable");
        assert!(args.windows(2).any(|v| v == ["--proto", "=https"]));
        assert!(args.windows(2).any(|v| v == ["--proto-redir", "=https"]));
        assert!(args.windows(2).any(|v| v == ["--max-redirs", "3"]));
        assert!(args.windows(2).any(|v| v == ["--max-filesize", "4096"]));
        assert!(!args
            .iter()
            .any(|v| v.contains("cookie") || v.contains("netrc") || v.contains("insecure")));
        assert!(
            curl_arguments("https://user:secret@example.com/a", false, 4096, 30, false).is_err()
        );
        assert!(curl_arguments("http://example.com/a", false, 4096, 30, false).is_err());
        let args = curl_arguments(
            "https://apix.ilanzou.com/unproved/recommend/list?shareId=public&code=&type=0",
            true,
            4096,
            30,
            true,
        )
        .unwrap();
        let args: Vec<_> = args.iter().map(|arg| arg.to_string_lossy()).collect();
        assert!(args
            .windows(2)
            .any(|v| v == ["--data-raw", "shareId=public&code=&type=0"]));
        assert_eq!(
            args.last().unwrap(),
            "https://apix.ilanzou.com/unproved/recommend/list"
        );
        assert!(args
            .iter()
            .any(|v| v == "Content-Type: application/x-www-form-urlencoded"));
        assert!(!args.iter().any(|v| v == "--request"));
    }
    #[test]
    fn updater_stream_limits_and_hash_lock_prevent_launching_partial_or_changed_files() {
        let root = temp_root();
        let bytes = synthetic_pe();
        let asset = asset(&bytes);
        let unrelated = root.join("keep.txt");
        fs::write(&unrelated, b"keep").unwrap();
        let bad = prepare_verified(&asset, &root, &mut |part| {
            fs::write(part, b"<html>login</html>").map_err(|_| "write".to_string())
        });
        assert!(bad.is_err());
        assert!(!root.join("installer.part").exists());
        assert_eq!(fs::read(&unrelated).unwrap(), b"keep");
        let verified = prepare_verified(&asset, &root, &mut |part| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(part)
                .map_err(|_| "create".to_string())?;
            copy_exact(
                &mut std::io::Cursor::new(&bytes),
                &mut file,
                asset.size,
                &mut |_| {},
            )
        })
        .unwrap();
        assert!(OpenOptions::new().write(true).open(&verified.path).is_err());
        assert!(fs::remove_file(&verified.path).is_err());
        let path = verified.path.clone();
        drop(verified);
        fs::remove_file(path).unwrap();
        fs::remove_file(unrelated).unwrap();
        fs::remove_dir(root).unwrap();
    }
    #[test]
    fn updater_command_is_explicit_and_preserves_unicode_destination() {
        let parameters = installer_parameters(Path::new(r"D:\应用程序\ZS Clip")).unwrap();
        let parameters = String::from_utf16(&parameters[..parameters.len() - 1]).unwrap();
        assert_eq!(
            parameters,
            r#"/ZSCLIPUPDATE=1 /SILENT /SP- /NORESTART /DIR="D:\应用程序\ZS Clip""#
        );
        assert!(installer_parameters(Path::new("relative/path")).is_err());
    }
}
