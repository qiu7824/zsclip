use super::prelude::*;

pub(super) unsafe fn settings_create_about_data_section(
    st: &mut SettingsWndState,
    b: &SettingsPageBuilder,
    flow: &mut SettingsFlowLayout,
) {
    let info_text = format!(
        "{}{}",
        tr("数据目录：", "Data directory: "),
        data_dir().to_string_lossy()
    );
    let info_rect = flow.full_rect(settings_scale(72));
    let _ = b.label_auto(
        st,
        &info_text,
        info_rect.left,
        info_rect.top,
        info_rect.right - info_rect.left,
        settings_scale(72),
    );
}
