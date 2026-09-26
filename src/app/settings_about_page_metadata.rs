use super::prelude::*;

pub(super) const ABOUT_SUMMARY_EN: &str =
    "Local clipboard history for text, images, files, and reusable phrases.";

pub(super) unsafe fn settings_create_about_metadata_section(
    st: &mut SettingsWndState,
    b: &SettingsPageBuilder,
    sec: SettingsFormSectionLayout,
    flow: &mut SettingsFlowLayout,
) {
    let version_text = format!(
        "{}{}",
        tr("版本：", "Version: "),
        crate::app_version::APP_VERSION
    );
    let version_rect = flow.full_rect(settings_scale(24));
    b.label(
        st,
        &version_text,
        version_rect.left,
        version_rect.top,
        version_rect.right - version_rect.left,
        version_rect.height(),
    );
    flow.consume_full(version_rect.height(), settings_scale(2));

    let summary_text = tr(
        "本地剪贴板管理工具，支持文本、图片、文件和常用短语。",
        ABOUT_SUMMARY_EN,
    );
    let summary_rect = flow.full_rect(settings_scale(24));
    let (_, summary_h) = b.label_auto(
        st,
        summary_text,
        summary_rect.left,
        summary_rect.top,
        summary_rect.width(),
        summary_rect.height(),
    );
    flow.consume_full(summary_h, settings_scale(4));

    let source_label_w = sec.label_w();
    let source_row_h = settings_scale(32);
    let source_label = flow.row_label_rect(source_label_w, source_row_h, 0);
    b.label(
        st,
        tr("开源地址：", "Source: "),
        source_label.left,
        source_label.top,
        source_label.right - source_label.left,
        source_label.bottom - source_label.top,
    );
    let source_field = flow.row_field_rect(source_label_w, source_row_h);
    let link = b.button(
        st,
        open_source_url_display(),
        IDC_SET_OPEN_SOURCE,
        source_field.left,
        source_field.top,
        source_field.right - source_field.left,
    );
    if !link.is_null() {
        st.ownerdraw_ctrls.push(link);
    }
    flow.consume_row(source_row_h, 0);
}
