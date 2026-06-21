use super::prelude::*;

pub(super) unsafe fn settings_relayout_plugin_page(st: &mut SettingsWndState) {
    settings_refresh_plugin_cards(st);
    let line_h = settings_scale(24);
    settings_relayout_plugin_quick_search_section(st, line_h);
    settings_relayout_plugin_ocr_section(st, line_h);
    settings_relayout_plugin_translate_section(st, line_h);
    settings_relayout_plugin_tool_sections(st);
    settings_refresh_plugin_host_after_relayout(st);
}
