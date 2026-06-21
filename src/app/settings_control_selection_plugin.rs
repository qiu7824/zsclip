use super::prelude::*;

pub(super) unsafe fn handle_settings_plugin_selection(
    st: &mut SettingsWndState,
    control_id: isize,
    index: usize,
) {
    match control_id {
        IDC_SET_SEARCH_ENGINE => {
            if let Some((_, label, template)) = SEARCH_ENGINE_PRESETS.get(index) {
                let old_engine = settings_host_text(st.cb_engine);
                let old_key = search_engine_key_from_display(&old_engine);
                let old_template = search_engine_template(old_key).to_string();
                let current_template = settings_host_text(st.ed_tpl);
                settings_set_text(st.cb_engine, label);
                if current_template.trim().is_empty() || current_template == old_template {
                    settings_set_text(st.ed_tpl, template);
                }
                repaint_settings_control(st.cb_engine);
            }
        }
        IDC_SET_OCR_PROVIDER => {
            if let Some((key, _)) = IMAGE_OCR_PROVIDER_OPTIONS.get(index) {
                st.draft.image_ocr_provider = (*key).to_string();
                settings_set_text(st.cb_ocr_provider, &image_ocr_provider_display(key));
                settings_sync_page_state(st, SettingsPage::Plugin.index());
                repaint_settings_control(st.cb_ocr_provider);
            }
        }
        IDC_SET_TRANSLATE_PROVIDER => {
            if let Some((key, _)) = TEXT_TRANSLATE_PROVIDER_OPTIONS.get(index) {
                st.draft.text_translate_provider = (*key).to_string();
                settings_set_text(
                    st.cb_translate_provider,
                    &text_translate_provider_display(key),
                );
                settings_sync_page_state(st, SettingsPage::Plugin.index());
                repaint_settings_control(st.cb_translate_provider);
            }
        }
        IDC_SET_TRANSLATE_TARGET => {
            if let Some((key, _)) = TEXT_TRANSLATE_TARGET_OPTIONS.get(index) {
                st.draft.text_translate_target_lang = (*key).to_string();
                settings_set_text(st.cb_translate_target, &text_translate_target_display(key));
                repaint_settings_control(st.cb_translate_target);
            }
        }
        _ => {}
    }
}
