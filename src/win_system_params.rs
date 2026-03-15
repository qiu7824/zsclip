use windows_sys::Win32::Foundation::POINT;

pub use crate::settings_layout::{SCROLL_BAR_MARGIN, SCROLL_BAR_W, SCROLL_BAR_W_ACTIVE, SETTINGS_CONTENT_TOTAL_H};
pub use crate::settings_model::{settings_section_body_rect, SETTINGS_FORM_ROW_GAP, SETTINGS_FORM_ROW_H};
pub use crate::settings_render::{
    IDC_SET_AUTOSTART, IDC_SET_BTN_OPENCFG, IDC_SET_BTN_OPENDB, IDC_SET_BTN_OPENDATA, IDC_SET_CLICK_HIDE,
    IDC_SET_CLOSE, IDC_SET_CLOSETRAY, IDC_SET_CLOUD_APPLY_CFG, IDC_SET_CLOUD_DIR, IDC_SET_CLOUD_ENABLE,
    IDC_SET_CLOUD_INTERVAL, IDC_SET_CLOUD_PASS, IDC_SET_CLOUD_RESTORE_BACKUP, IDC_SET_CLOUD_SYNC_NOW,
    IDC_SET_CLOUD_UPLOAD_CFG, IDC_SET_CLOUD_URL, IDC_SET_CLOUD_USER, IDC_SET_DX, IDC_SET_DY,
    IDC_SET_EDGEHIDE, IDC_SET_FX, IDC_SET_FY, IDC_SET_GROUP_ADD, IDC_SET_GROUP_DELETE, IDC_SET_GROUP_DOWN,
    IDC_SET_GROUP_ENABLE, IDC_SET_GROUP_LIST, IDC_SET_GROUP_RENAME, IDC_SET_GROUP_UP, IDC_SET_HOVERPREVIEW,
    IDC_SET_IMAGE_PREVIEW, IDC_SET_MAX, IDC_SET_OPEN_SOURCE, IDC_SET_PLUGIN_MAILMERGE,
    IDC_SET_POSMODE, IDC_SET_QUICK_DELETE, IDC_SET_SAVE, SETTINGS_CLASS,
};

pub(crate) const GMEM_MOVEABLE: u32 = 0x0002;
pub(crate) const GMEM_ZEROINIT: u32 = 0x0040;
pub(crate) const MK_LBUTTON_FLAG: u32 = 0x0001;
pub(crate) const S_OK_HR: i32 = 0;
pub(crate) const E_NOINTERFACE_HR: i32 = 0x80004002u32 as i32;
pub(crate) const E_POINTER_HR: i32 = 0x80004003u32 as i32;
pub(crate) const DRAGDROP_S_DROP_HR: i32 = 0x00040100;
pub(crate) const DRAGDROP_S_CANCEL_HR: i32 = 0x00040101;
pub(crate) const DRAGDROP_S_USEDEFAULTCURSORS_HR: i32 = 0x00040102;
pub(crate) const RPC_E_CHANGED_MODE_HR: i32 = 0x80010106u32 as i32;
pub(crate) const CF_HDROP: u32 = 15;

pub(crate) const IID_IUNKNOWN_RAW: windows_sys::core::GUID =
    windows_sys::core::GUID::from_u128(0x00000000_0000_0000_c000_000000000046);
pub(crate) const IID_IDROPSOURCE_RAW: windows_sys::core::GUID =
    windows_sys::core::GUID::from_u128(0x00000121_0000_0000_c000_000000000046);
pub(crate) const IID_IDATAOBJECT_RAW: windows_sys::core::GUID =
    windows_sys::core::GUID::from_u128(0x0000010e_0000_0000_c000_000000000046);

#[repr(C)]
pub(crate) struct DropFiles {
    pub(crate) p_files: u32,
    pub(crate) pt: POINT,
    pub(crate) f_nc: i32,
    pub(crate) f_wide: i32,
}
