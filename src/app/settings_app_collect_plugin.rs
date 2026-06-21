use super::prelude::*;
use crate::win_system_ui::settings_host_text;

pub(super) unsafe fn settings_collect_plugin_to_draft(st: &mut SettingsWndState) {
    if st.ui.is_built(SettingsPage::Plugin.index()) && !st.cb_engine.is_null() {
        st.draft.search_engine =
            search_engine_key_from_display(&settings_host_text(st.cb_engine)).to_string();
        st.draft.search_template = {
            let tpl = settings_host_text(st.ed_tpl);
            if tpl.trim().is_empty() {
                search_engine_template(&st.draft.search_engine).to_string()
            } else {
                tpl
            }
        };
        st.draft.image_ocr_provider =
            image_ocr_provider_key_from_display(&settings_host_text(st.cb_ocr_provider))
                .to_string();
        if st.draft.image_ocr_provider == "winocr" {
            st.draft.image_ocr_wechat_dir = settings_host_text(st.ed_ocr_cloud_url);
        } else {
            st.draft.image_ocr_cloud_url = settings_host_text(st.ed_ocr_cloud_url);
            st.draft.image_ocr_cloud_token = settings_host_text(st.ed_ocr_cloud_token);
        }
        st.draft.text_translate_provider =
            text_translate_provider_key_from_display(&settings_host_text(st.cb_translate_provider))
                .to_string();
        if st.draft.text_translate_provider == "baidu" {
            st.draft.text_translate_app_id = settings_host_text(st.ed_translate_app_id);
            st.draft.text_translate_secret = settings_host_text(st.ed_translate_secret);
            st.draft.text_translate_target_lang =
                text_translate_target_key_from_display(&settings_host_text(st.cb_translate_target))
                    .to_string();
        }
    }
}
