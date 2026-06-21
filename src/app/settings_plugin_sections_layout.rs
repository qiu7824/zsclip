use super::prelude::*;

pub(super) fn settings_plugin_layout(
    st: &SettingsWndState,
    index: usize,
    label_w: i32,
) -> SettingsFormSectionLayout {
    crate::settings_model::settings_form_layout_for_section(
        SettingsPage::Plugin.index(),
        index,
        label_w,
        &st.plugin_sections,
    )
}

pub(super) unsafe fn settings_refresh_plugin_cards(st: &mut SettingsWndState) {
    st.plugin_sections = crate::settings_model::settings_plugin_cards_for_state(
        st.draft.quick_search_enabled,
        &st.draft.image_ocr_provider,
        &st.draft.text_translate_provider,
        st.draft.super_mail_merge_enabled,
        st.draft.wps_taskpane_enabled,
    );
}

pub(super) unsafe fn settings_refresh_plugin_host_after_relayout(st: &mut SettingsWndState) {
    let hwnd = [
        st.chk_qs,
        st.cb_ocr_provider,
        st.cb_translate_provider,
        st.chk_ai,
        st.chk_mm,
        st.chk_wps_taskpane,
        st.chk_qr,
    ]
    .iter()
    .find_map(|&child| {
        if child.is_null() {
            None
        } else {
            let parent = platform_window::parent(child);
            if parent.is_null() {
                None
            } else {
                Some(parent)
            }
        }
    })
    .unwrap_or(null_mut());
    if !hwnd.is_null() && st.cur_page == SettingsPage::Plugin.index() {
        settings_repos_controls(hwnd, st, true);
        platform_gdi::invalidate_rect(hwnd, null(), 0);
    }
}
