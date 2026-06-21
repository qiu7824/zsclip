use super::prelude::*;

pub(super) fn settings_open_plugin_dropdown_for_control(
    hwnd: HWND,
    st: &mut SettingsWndState,
    control_id: isize,
) -> bool {
    match control_id {
        7201 => {
            let rc = settings_control_screen_rect_or_empty(st.cb_engine);
            let current = SEARCH_ENGINE_PRESETS
                .iter()
                .position(|(_, name, _)| *name == settings_host_text(st.cb_engine))
                .unwrap_or(0);
            let labels: Vec<&str> = SEARCH_ENGINE_PRESETS
                .iter()
                .map(|(_, name, _)| *name)
                .collect();
            st.dropdown_popup =
                present_settings_dropdown_popup(hwnd, 7201, &rc, &labels, current, 260);
            true
        }
        IDC_SET_OCR_PROVIDER => {
            let rc = settings_control_screen_rect_or_empty(st.cb_ocr_provider);
            let current = IMAGE_OCR_PROVIDER_OPTIONS
                .iter()
                .position(|(key, _)| {
                    image_ocr_provider_display(key) == settings_host_text(st.cb_ocr_provider)
                })
                .unwrap_or(0);
            let labels_owned: Vec<String> = IMAGE_OCR_PROVIDER_OPTIONS
                .iter()
                .map(|(key, _)| image_ocr_provider_display(key))
                .collect();
            let labels: Vec<&str> = labels_owned.iter().map(|s| s.as_str()).collect();
            st.dropdown_popup = present_settings_dropdown_popup(
                hwnd,
                IDC_SET_OCR_PROVIDER,
                &rc,
                &labels,
                current,
                240,
            );
            true
        }
        IDC_SET_TRANSLATE_PROVIDER => {
            let rc = settings_control_screen_rect_or_empty(st.cb_translate_provider);
            let current = TEXT_TRANSLATE_PROVIDER_OPTIONS
                .iter()
                .position(|(key, _)| {
                    text_translate_provider_display(key)
                        == settings_host_text(st.cb_translate_provider)
                })
                .unwrap_or(0);
            let labels_owned: Vec<String> = TEXT_TRANSLATE_PROVIDER_OPTIONS
                .iter()
                .map(|(key, _)| text_translate_provider_display(key))
                .collect();
            let labels: Vec<&str> = labels_owned.iter().map(|s| s.as_str()).collect();
            st.dropdown_popup = present_settings_dropdown_popup(
                hwnd,
                IDC_SET_TRANSLATE_PROVIDER,
                &rc,
                &labels,
                current,
                240,
            );
            true
        }
        IDC_SET_TRANSLATE_TARGET => {
            let rc = settings_control_screen_rect_or_empty(st.cb_translate_target);
            let current = TEXT_TRANSLATE_TARGET_OPTIONS
                .iter()
                .position(|(key, _)| {
                    text_translate_target_display(key) == settings_host_text(st.cb_translate_target)
                })
                .unwrap_or(0);
            let labels_owned: Vec<String> = TEXT_TRANSLATE_TARGET_OPTIONS
                .iter()
                .map(|(key, _)| text_translate_target_display(key))
                .collect();
            let labels: Vec<&str> = labels_owned.iter().map(|s| s.as_str()).collect();
            st.dropdown_popup = present_settings_dropdown_popup(
                hwnd,
                IDC_SET_TRANSLATE_TARGET,
                &rc,
                &labels,
                current,
                200,
            );
            true
        }
        _ => false,
    }
}
