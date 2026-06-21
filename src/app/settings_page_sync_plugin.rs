use super::prelude::*;

pub(super) unsafe fn settings_sync_plugin_page_state(st: &mut SettingsWndState) {
    let s = st.draft.clone();
    settings_relayout_plugin_page(st);
    settings_set_text(st.cb_engine, &search_engine_display(&s.search_engine));
    settings_set_text(st.ed_tpl, &s.search_template);
    let search_enabled = s.quick_search_enabled;
    settings_plugin_show_enable(st, st.lb_qs_status, false, false);
    settings_plugin_show_enable(st, st.lb_qs_engine, search_enabled, false);
    settings_plugin_show_enable(st, st.cb_engine, search_enabled, true);
    settings_plugin_show_enable(st, st.lb_qs_template, search_enabled, false);
    settings_plugin_show_enable(st, st.ed_tpl, search_enabled, true);
    settings_plugin_show_enable(st, st.btn_qs_restore, search_enabled, true);
    settings_plugin_show_enable(st, st.lb_qs_hint, false, false);
    settings_set_text(
        st.cb_ocr_provider,
        &image_ocr_provider_display(&s.image_ocr_provider),
    );
    let baidu_enabled = s.image_ocr_provider == "baidu";
    let winocr_enabled = s.image_ocr_provider == "winocr";
    let ocr_fields_visible = baidu_enabled || winocr_enabled;
    settings_set_text(
        st.lb_ocr_primary,
        if winocr_enabled {
            tr("微信目录", "WeChat directory")
        } else {
            tr("API Key", "API Key")
        },
    );
    settings_set_text(st.lb_ocr_secondary, tr("Secret Key", "Secret Key"));
    settings_set_text(
        st.ed_ocr_cloud_url,
        if winocr_enabled {
            &s.image_ocr_wechat_dir
        } else {
            &s.image_ocr_cloud_url
        },
    );
    settings_set_text(st.ed_ocr_cloud_token, &s.image_ocr_cloud_token);
    settings_plugin_show_enable(st, st.lb_ocr_primary, ocr_fields_visible, false);
    settings_plugin_show_enable(st, st.ed_ocr_cloud_url, ocr_fields_visible, true);
    settings_plugin_show_enable(st, st.lb_ocr_secondary, baidu_enabled, false);
    settings_plugin_show_enable(st, st.ed_ocr_cloud_token, baidu_enabled, true);
    settings_plugin_show_enable(st, st.btn_ocr_detect, winocr_enabled, true);
    settings_plugin_show_enable(st, st.lb_ocr_status, false, false);
    settings_set_text(
        st.cb_translate_provider,
        &text_translate_provider_display(&s.text_translate_provider),
    );
    settings_set_text(st.ed_translate_app_id, &s.text_translate_app_id);
    settings_set_text(st.ed_translate_secret, &s.text_translate_secret);
    settings_set_text(
        st.cb_translate_target,
        &text_translate_target_display(&s.text_translate_target_lang),
    );
    let translate_enabled = s.text_translate_provider == "baidu";
    settings_plugin_show_enable(st, st.lb_translate_primary, translate_enabled, false);
    settings_plugin_show_enable(st, st.ed_translate_app_id, translate_enabled, true);
    settings_plugin_show_enable(st, st.lb_translate_secondary, translate_enabled, false);
    settings_plugin_show_enable(st, st.ed_translate_secret, translate_enabled, true);
    settings_plugin_show_enable(st, st.lb_translate_target, translate_enabled, false);
    settings_plugin_show_enable(st, st.cb_translate_target, translate_enabled, true);
    settings_plugin_show_enable(st, st.lb_translate_status, false, false);
    settings_plugin_show_enable(st, st.lb_ai_status, false, false);
    settings_plugin_show_enable(st, st.lb_mail_merge_status, false, false);
    settings_plugin_show_enable(st, st.btn_mail_merge, s.super_mail_merge_enabled, true);
    settings_plugin_show_enable(st, st.btn_wps_taskpane_docs, s.wps_taskpane_enabled, true);
    settings_plugin_show_enable(st, st.lb_qr_status, false, false);
}
