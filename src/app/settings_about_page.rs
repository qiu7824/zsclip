use super::prelude::*;

pub(super) unsafe fn settings_create_about_page(hwnd: HWND, st: &mut SettingsWndState) {
    let page = SettingsPage::About.index();
    let b = SettingsPageBuilder {
        hwnd,
        page,
        font: st.ui_font,
    };
    let sec = crate::settings_model::settings_about_section_layout(0, 88);
    let mut flow = SettingsFlowLayout::new(sec.left(), sec.row_y(0), sec.full_w());

    settings_create_about_metadata_section(st, &b, sec, &mut flow);
    let update = crate::settings_model::settings_about_section_layout(1, 88);
    let mut update_flow = SettingsFlowLayout::new(update.left(), update.row_y(0), update.full_w());
    settings_create_about_update_section(st, &b, &mut update_flow);
    let data = crate::settings_model::settings_about_section_layout(2, 88);
    let mut data_flow = SettingsFlowLayout::new(data.left(), data.row_y(0), data.full_w());
    settings_create_about_data_section(st, &b, &mut data_flow);

    st.ui.mark_built(page);
}

#[cfg(test)]
mod tests {
    use super::*;

    struct HiddenAboutSurface(HWND);

    impl HiddenAboutSurface {
        unsafe fn new(width: i32) -> Self {
            let hwnd = platform_window::create_window_ex(
                0,
                crate::platform::string::to_wide("STATIC").as_ptr(),
                crate::platform::string::to_wide("About layout test").as_ptr(),
                WS_POPUP,
                0,
                0,
                width,
                800,
                null_mut(),
                null_mut(),
                platform_window::module_handle(),
                null(),
            );
            assert!(!hwnd.is_null());
            set_settings_ui_dpi(settings_window_layout_dpi(hwnd));
            platform_window::move_window(
                hwnd,
                0,
                0,
                settings_scale(width),
                settings_scale(800),
                false,
            );
            crate::app_core::set_settings_client_width(settings_scale(width));
            let state = create_settings_window_state(hwnd, null_mut());
            platform_window::set_user_data(hwnd, Box::into_raw(state) as isize);
            Self(hwnd)
        }
    }

    impl Drop for HiddenAboutSurface {
        fn drop(&mut self) {
            unsafe {
                handle_settings_destroy(self.0);
                platform_window::destroy(self.0);
            }
            crate::app_core::set_settings_client_width(0);
        }
    }

    #[test]
    fn compact_about_controls_fit_cards_at_minimum_and_default_width() {
        unsafe {
            for width in [920, 1080] {
                let surface = HiddenAboutSurface::new(width);
                let state = &mut *(platform_window::user_data(surface.0) as *mut SettingsWndState);
                settings_create_about_page(surface.0, state);
                let cards = crate::settings_model::settings_about_cards();
                assert!(cards.last().unwrap().rect.bottom <= settings_scale(600));
                let controls = state
                    .ui
                    .page_regs(SettingsPage::About.index())
                    .collect::<Vec<_>>();
                for (index, first) in controls.iter().enumerate() {
                    assert!(
                        cards.iter().any(|card| {
                            first.bounds.left >= card.rect.left + settings_scale(18)
                                && first.bounds.right <= card.rect.right - settings_scale(18)
                                && first.bounds.top >= card.rect.top + settings_scale(40)
                                && first.bounds.bottom <= card.rect.bottom - settings_scale(12)
                        }),
                        "About control falls outside its card at {width}px: {} {:?}",
                        settings_host_text(first.hwnd),
                        first.bounds
                    );
                    for second in controls.iter().skip(index + 1) {
                        assert!(
                            first.bounds.right <= second.bounds.left
                                || second.bounds.right <= first.bounds.left
                                || first.bounds.bottom <= second.bounds.top
                                || second.bounds.bottom <= first.bounds.top
                        );
                    }
                }
                let directory = controls
                    .iter()
                    .find(|reg| {
                        settings_host_text(reg.hwnd)
                            .starts_with(tr("数据目录：", "Data directory: "))
                    })
                    .expect("data directory label");
                let long_path =
                    format!("Data directory: D:\\{}\\zsclip", "long folder\\".repeat(80));
                settings_set_text(directory.hwnd, &long_path);
                assert_eq!(settings_host_text(directory.hwnd), long_path);
                assert_ne!(platform_window::window_style(directory.hwnd) & 0x4000, 0);
                assert_eq!(directory.bounds.height(), settings_scale(28));
                assert!(!platform_window::is_visible(surface.0));
            }
        }
    }

    #[test]
    fn compact_about_english_text_fits_the_reserved_body_height() {
        unsafe {
            let surface = HiddenAboutSurface::new(920);
            let state = &mut *(platform_window::user_data(surface.0) as *mut SettingsWndState);
            let section = crate::settings_model::settings_about_section_layout(0, 88);
            let summary_h = crate::settings_ui_host::settings_measure_text_height(
                surface.0,
                ABOUT_SUMMARY_EN,
                section.full_w(),
                state.ui_font,
                settings_scale(24),
            );
            assert!(settings_scale(24 + 2 + 4 + 32) + summary_h <= settings_scale(100));
            let note_h = crate::settings_ui_host::settings_measure_text_height(
                surface.0,
                ABOUT_DATA_RETENTION_EN,
                section.full_w(),
                state.ui_font,
                settings_scale(24),
            );
            assert!(settings_scale(28 + 4) + note_h <= settings_scale(108));
            for (label, width) in [
                ("Check for updates", 160),
                ("Release notes", 128),
                ("Update source", 128),
            ] {
                let height = crate::settings_ui_host::settings_measure_text_height(
                    surface.0,
                    label,
                    settings_scale(width - 24),
                    state.ui_font,
                    0,
                );
                assert!(
                    height <= settings_scale(32),
                    "English button wraps: {label}"
                );
            }
        }
    }
}
