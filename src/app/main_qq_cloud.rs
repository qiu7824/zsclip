use super::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};

const UPLOAD: usize = 53001;
const CONNECT: usize = 53002;
const STATUS: usize = 53003;
const DISCONNECT: usize = 53004;
static BUSY: AtomicBool = AtomicBool::new(false);

pub(super) fn extend_menu(mut entries: Vec<NativePopupMenuEntry>, kind: ClipKind, selected: usize, visible: bool) -> Vec<NativePopupMenuEntry> {
    if !visible { return entries; }
    entries.push(NativePopupMenuEntry::Separator);
    entries.push(NativePopupMenuEntry::Command { id:UPLOAD, label:"上传 QQ 云剪贴板".into(),
        enabled:!BUSY.load(Ordering::Acquire) && selected<=1 && matches!(kind,ClipKind::Text|ClipKind::Phrase), checked:false });
    entries
}

pub(super) fn is_settings_command(command: usize) -> bool { matches!(command,CONNECT|STATUS|DISCONNECT) }

pub(super) unsafe fn handle_settings_command(hwnd: HWND,parent: HWND,command: usize) -> bool {
    if !is_settings_command(command) { return false; }
    let ptr=get_state_ptr(parent);if ptr.is_null() { return true; }
    let settings=(*ptr).settings.clone();
    handle_request(hwnd,&settings,parent,command,None,0)
}

pub(super) unsafe fn create_settings_section(st: &mut SettingsWndState,b: &SettingsPageBuilder) {
    let index=st.multi_sync_sections.len().saturating_sub(1);
    let section=settings_multi_sync_layout(st,index,110);
    b.form_label(st,&section,0,"账号授权：");
    st.lb_qq_cloud_status=b.form_value_label_auto(st,&section,0,"正在读取授权状态…",settings_scale(70)).0;
    let buttons=b.form_action_row(st,&section,2,&[("导入手机授权",CONNECT as isize,settings_scale(150)),("查看授权状态",STATUS as isize,settings_scale(150)),("撤销电脑授权",DISCONNECT as isize,settings_scale(150))]);
    st.btn_qq_cloud_connect=buttons[0];st.btn_qq_cloud_status=buttons[1];st.btn_qq_cloud_disconnect=buttons[2];
    let (_,toggle)=b.own_toggle_row(st,"右键显示“上传 QQ 云剪贴板”",crate::win_system_params::IDC_SET_QQ_CLOUD_MENU,section.left(),section.row_y(3),section.full_w());
    st.chk_qq_cloud_menu=toggle;
    b.label_auto(st,"QQ 云仅手动上传，与局域网方向独立。手机需已有有效登录态；授权保存后电脑可独立上传。",section.left(),section.row_y(4),section.full_w(),settings_scale(70));
}

pub(super) unsafe fn sync_settings_section(st:&mut SettingsWndState) -> bool {
    let busy=BUSY.load(Ordering::Acquire);
    let text=if busy { "正在处理授权或上传…" } else if crate::qq_cloud_auth::load_account().is_ok() {
        "此电脑已保存授权；有效性会在上传时核验。"
    } else { "此电脑尚无可用授权。局域网文字同步不需要 QQ 登录。" };
    let changed=crate::win_system_ui::settings_host_text(st.lb_qq_cloud_status)!=text;
    settings_set_text(st.lb_qq_cloud_status,text);
    for hwnd in [st.btn_qq_cloud_connect,st.btn_qq_cloud_status,st.btn_qq_cloud_disconnect] {
        if !hwnd.is_null() { crate::win_system_ui::settings_host_set_enabled(hwnd,!busy); }
    }
    if !st.chk_qq_cloud_menu.is_null() { repaint_settings_control(st.chk_qq_cloud_menu); }
    changed
}

struct BusyGuard { main_owner: usize }
impl Drop for BusyGuard {
    fn drop(&mut self) {
        BUSY.store(false, Ordering::Release);
        crate::platform::window::post_message(self.main_owner as isize,crate::app::WM_LAN_SYNC_READY,0,0);
    }
}

fn message(owner: usize, body: &str, error: bool) {
    platform_dialog::WindowsDialogHost::new().show_message(
        owner as HWND,
        "QQ 云剪贴板",
        body,
        if error {
            NativeDialogLevel::Error
        } else {
            NativeDialogLevel::Info
        },
    );
}

pub(super) unsafe fn handle_command(
    hwnd: HWND,
    state: &AppState,
    command: usize,
    item: Option<&ClipItem>,
    selected: usize,
) -> bool {
    let settings=state.settings.clone();
    handle_request(hwnd,&settings,state.hwnd,command,item,selected)
}

unsafe fn handle_request(hwnd: HWND,settings:&AppSettings,main_owner:HWND,command:usize,item:Option<&ClipItem>,selected:usize) -> bool {
    if ![UPLOAD, CONNECT, STATUS, DISCONNECT].contains(&command) {
        return false;
    }
    if BUSY.load(Ordering::Acquire) {
        message(hwnd as usize, "QQ 云剪贴板正在处理请求，请稍后。", false);
        return true;
    }
    if command==UPLOAD && !settings.qq_cloud_menu_enabled { return true; }
    let text = if command == UPLOAD {
        let Some(item) = item.filter(|i| matches!(i.kind, ClipKind::Text | ClipKind::Phrase))
        else {
            return true;
        };
        if selected > 1 {
            message(hwnd as usize, "每次请选择一条文本上传。", false);
            return true;
        }
        let text = item.text.clone().unwrap_or_default();
        if crate::db_runtime::text_is_protected(&text) {
            message(hwnd as usize, "密码与密钥不会上传到 QQ 云剪贴板。", false);
            return true;
        }
        if let Err(error) = crate::qq_cloud::validate_text(&text) {
            message(hwnd as usize, &error, true);
            return true;
        }
        Some(text)
    } else {
        None
    };
    if command == CONNECT
        && (!settings.lan_sync_enabled || lan_sync::trusted_devices().is_empty())
    {
        message(hwnd as usize, "请先在“多端同步”中开启局域网并与输入法配对。局域网同步无需 QQ 登录。\n\nQQ 客户端会拒绝该重签名包的授权登录；手机需先取得有效登录态，才能导入云授权。", false);
        return true;
    }
    if command == DISCONNECT
        && platform_dialog::WindowsDialogHost::new().confirm(
            hwnd,
            "清除 QQ 云授权",
            "清除这台电脑保存的 QQ 云剪贴板授权？手机中的账号和云端内容将保留。",
            NativeDialogLevel::Question,
            NativeDialogButtons::YesNo,
        ) != NativeDialogResponse::Yes
    {
        return true;
    }
    if BUSY
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return true;
    }
    let owner = hwnd as usize;
    let main_owner=main_owner as usize;
    std::thread::spawn(move || {
        let _busy = BusyGuard { main_owner: main_owner as usize };
        match command {
            CONNECT => match crate::qq_cloud_auth::begin_authorization() {
                Ok(window) => {
                    message(owner, &format!("手机需已有合法有效登录态。QQ 客户端登录可能拒绝重签名应用；已有有效登录态可继续授权。局域网文字同步不需要 QQ 登录。\n\n电脑校验码：{}\n\n在手机输入法剪贴板中打开 ZSClip，选择导入现有云授权，核对两端校验码后确认。\n\n授权窗口有效 5 分钟。手机显示“待电脑确认”和确认码后点击这里的“确定”，再核对手机提供的确认码。", window.code), false);
                    match crate::qq_cloud_auth::pending_confirmation(&window.request_id) {
                        Ok(code) => {
                            let display = code.as_bytes().chunks(4).map(|part| String::from_utf8_lossy(part).to_uppercase()).collect::<Vec<_>>().join(" ");
                            let approved = platform_dialog::WindowsDialogHost::new().confirm(
                                owner as HWND, "确认 QQ 云账号授权",
                                &format!("授权确认码：{display}\n\n手机显示的确认码是否完全相同？\n只有一致时才点击“是”保存账号。"),
                                NativeDialogLevel::Question, NativeDialogButtons::YesNo) == NativeDialogResponse::Yes;
                            if approved {
                                match crate::qq_cloud_auth::confirm_authorization(&window.request_id, &code) {
                                    Ok(()) => message(owner, "QQ 云授权已安全保存。此后可直接上传所选文本，手机无需在线。账号授权失效时需要重新连接。", false),
                                    Err(error) => message(owner, &error, true),
                                }
                            }
                        }
                        Err(error) => message(owner, &error, false),
                    }
                    crate::qq_cloud_auth::cancel_authorization();
                }
                Err(error) => message(owner, &error, true),
            },
            UPLOAD => {
                if crate::db_runtime::text_is_protected(text.as_deref().unwrap_or_default()) {
                    message(owner, "密码与密钥不会上传到 QQ 云剪贴板。", false);
                    return;
                }
                let result = crate::qq_cloud_auth::load_account()
                    .and_then(|account| crate::qq_cloud::upload_text(&account, text.as_deref().unwrap_or_default()));
                match result { Ok(outcome) => message(owner, outcome.status_text(), false), Err(error) => message(owner, &error, true) }
            }
            STATUS => match crate::qq_cloud_auth::load_account() {
                Ok(_) => message(owner, "此电脑已保存 QQ 云剪贴板授权。上传由电脑直接完成，无需手机在线；授权有效性将在上传时核验。", false),
                Err(error) => message(owner, &format!("{error}\n\nQQ 客户端会拒绝该重签名包的授权登录；手机取得有效登录态后才能导入云授权。局域网同步不受此限制。"), false),
            },
            DISCONNECT => match crate::qq_cloud_auth::disconnect() {
                Ok(()) => message(owner, "已清除此电脑保存的 QQ 云授权。", false),
                Err(error) => message(owner, &error, true),
            },
            _ => {}
        }
    });
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn optional_row_menu_contains_only_one_upload_action() {
        assert!(extend_menu(Vec::new(),ClipKind::Text,1,false).is_empty());
        for (kind,count,expected) in [(ClipKind::Text,1,true),(ClipKind::Phrase,1,true),(ClipKind::Text,2,false),(ClipKind::Image,1,false)] {
            let items=extend_menu(Vec::new(),kind,count,true);
            assert_eq!(items.len(),2);
            assert!(matches!(&items[1],NativePopupMenuEntry::Command{id,enabled,..} if *id==UPLOAD && *enabled==expected));
            assert!(!items.iter().any(|item|matches!(item,NativePopupMenuEntry::Submenu{..})));
        }
    }
}
