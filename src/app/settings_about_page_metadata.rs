use super::prelude::*;

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
    let version_rect = flow.full_rect(settings_scale(28));
    let (_, version_h) = b.label_auto(
        st,
        &version_text,
        version_rect.left,
        version_rect.top,
        version_rect.right - version_rect.left,
        settings_scale(28),
    );
    flow.consume_full(version_h, settings_scale(8));

    let summary_text = format!(
        "{}\r\n{}",
        tr(
            "设置界面现在统一使用同一套 section/form 布局。",
            "The settings window now uses a unified section/form layout.",
        ),
        tr(
            "新增设置项时可以直接复用卡片、字段列、按钮行和统一间距。",
            "New settings can reuse the same cards, field columns, action rows, and spacing.",
        )
    );
    let summary_rect = flow.full_rect(settings_scale(72));
    let (_, summary_h) = b.label_auto(
        st,
        &summary_text,
        summary_rect.left,
        summary_rect.top,
        summary_rect.right - summary_rect.left,
        settings_scale(72),
    );
    flow.consume_full(summary_h, settings_scale(10));

    let source_label_w = sec.label_w();
    let source_row_h = settings_scale(34);
    let source_label = flow.row_label_rect(source_label_w, settings_scale(24), settings_scale(2));
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
    flow.consume_row(source_row_h, settings_scale(10));
}
