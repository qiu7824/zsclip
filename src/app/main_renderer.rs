use super::prelude::*;

fn main_paint_fill_color(fill: MainPaintFill, th: Theme) -> u32 {
    match fill {
        MainPaintFill::Theme(role) => main_theme_role_color(role, th),
        MainPaintFill::ScrollbarThumb { alpha } => {
            let c = ((alpha as u32 * 100 + 127) / 255) as u8 + 100;
            rgb(c, c, c)
        }
    }
}

pub(super) unsafe fn draw_main_paint_command(hdc: HDC, command: MainPaintCommand, th: Theme) {
    match command {
        MainPaintCommand::FillRect { rect, fill } => {
            let rect: RECT = rect.into();
            let br = platform_gdi::create_solid_brush(main_paint_fill_color(fill, th));
            platform_gdi::fill_rect(hdc, &rect, br);
            platform_gdi::delete_object(br as _);
        }
        MainPaintCommand::RoundRect {
            rect,
            fill,
            stroke,
            radius,
        } => {
            let rect: RECT = rect.into();
            let border = stroke
                .map(|role| main_theme_role_color(role, th))
                .unwrap_or(0);
            draw_round_rect(
                hdc as _,
                &rect,
                main_paint_fill_color(fill, th),
                border,
                radius,
            );
        }
        MainPaintCommand::RoundFill { rect, fill, radius } => {
            let rect: RECT = rect.into();
            draw_round_fill(hdc as _, &rect, main_paint_fill_color(fill, th), radius);
        }
    }
}

fn main_text_command_text(role: MainTextRole) -> &'static str {
    match role {
        MainTextRole::SegmentRecords => "复制记录",
        MainTextRole::SegmentPhrases => "常用短语",
        MainTextRole::EmptyLoading => tr("正在加载...", "Loading..."),
        MainTextRole::EmptyError => tr("加载失败，请稍后重试", "Loading failed. Please try again."),
        MainTextRole::EmptyGroup => tr("当前分组暂无记录", "No records in this group"),
        MainTextRole::EmptyRecords => tr("暂无剪贴板记录", "No clipboard records yet"),
        MainTextRole::EmptyPhrases => tr("暂无短语", "No phrases yet"),
        MainTextRole::LoadingFooter => "继续加载中...",
        MainTextRole::ScrollToTopArrow => "↑",
    }
}

pub(super) fn main_text_command_font(font: MainFontRole) -> &'static str {
    match font {
        MainFontRole::UiText => ui_text_font_family(),
        MainFontRole::Display => ui_display_font_family(),
    }
}

pub(super) fn main_text_command_centered(horizontal_align: HorizontalAlign) -> bool {
    matches!(horizontal_align, HorizontalAlign::Center)
}

unsafe fn draw_main_text_command(hdc: HDC, command: MainTextCommand, th: Theme) {
    let rect: RECT = command.rect.into();
    if command.wrap == TextWrap::Word {
        draw_text_block_ex(
            hdc as _,
            main_text_command_text(command.role),
            &rect,
            main_theme_role_color(command.color, th),
            command.size,
            command.bold,
            main_text_command_font(command.font),
        );
    } else {
        draw_text_ex(
            hdc as _,
            main_text_command_text(command.role),
            &rect,
            main_theme_role_color(command.color, th),
            command.size,
            command.bold,
            main_text_command_centered(command.horizontal_align),
            main_text_command_font(command.font),
        );
    }
}

unsafe fn draw_main_text_commands(
    hdc: HDC,
    commands: &[MainTextCommand],
    layer: MainTextLayer,
    th: Theme,
) {
    for command in commands.iter().filter(|command| command.layer == layer) {
        draw_main_text_command(hdc, *command, th);
    }
}

unsafe fn draw_main_row_text_command(hdc: HDC, command: MainRowTextCommand, text: &str, th: Theme) {
    let rect: RECT = command.rect.into();
    if command.wrap == TextWrap::Word {
        draw_text_block_ex(
            hdc as _,
            text,
            &rect,
            main_theme_role_color(command.color, th),
            command.size,
            command.bold,
            main_text_command_font(command.font),
        );
    } else {
        draw_text_ex(
            hdc as _,
            text,
            &rect,
            main_theme_role_color(command.color, th),
            command.size,
            command.bold,
            main_text_command_centered(command.horizontal_align),
            main_text_command_font(command.font),
        );
    }
}

fn main_icon_asset_kind(kind: MainIconKind) -> IconAssetKind {
    IconAssetKind::from_zsui(kind.zsui_icon())
        .expect("main window icon must have a Windows icon asset")
}

fn main_row_icon_kind_for_clip_presentation(
    kind_icon: crate::app_core::NativeHostClipKindIcon,
) -> MainIconKind {
    match kind_icon {
        crate::app_core::NativeHostClipKindIcon::Text => MainIconKind::Text,
        crate::app_core::NativeHostClipKindIcon::Image => MainIconKind::Image,
        crate::app_core::NativeHostClipKindIcon::Phrase => MainIconKind::Text,
        crate::app_core::NativeHostClipKindIcon::Files => MainIconKind::File,
        crate::app_core::NativeHostClipKindIcon::Folder => MainIconKind::Folder,
    }
}

pub(super) fn windows_native_clip_row_component_specs_for_items(
    items: &[ClipItem],
) -> Vec<crate::app_core::NativeClipRowSpec> {
    let projections = items
        .iter()
        .map(|item| {
            crate::app_core::NativeHostClipListItemProjection::with_metadata(
                item.id,
                item.source_app.clone(),
                item.preview.clone(),
                item.kind,
                item.pinned,
            )
        })
        .collect::<Vec<_>>();
    crate::app_core::native_host_clip_row_specs(
        &projections,
        projections
            .len()
            .min(crate::app_core::NATIVE_HOST_CLIP_ROW_CAPACITY),
    )
}

unsafe fn draw_main_icon_command(hdc: HDC, command: MainIconCommand, dark: bool) {
    let rect: RECT = command.rect.into();
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    let icon = icon_handle_for(main_icon_asset_kind(command.kind), width.max(height));
    if icon != 0 {
        let (tint_for_dark_mode, soften) = match command.color_mode {
            MainIconColorMode::ThemeAware => (dark, 0),
            MainIconColorMode::Original => (false, 0),
        };
        draw_icon_tinted_soft(
            hdc as _,
            rect.left,
            rect.top,
            icon,
            width,
            height,
            tint_for_dark_mode,
            soften,
        );
    }
}

pub(super) unsafe fn paint_main_window(hwnd: HWND) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    state.maybe_request_more_for_active_tab();
    let th = state.theme;
    let dark = platform_appearance::is_dark_mode();

    let mut ps: PAINTSTRUCT = zeroed();
    let hdc = platform_gdi::begin_paint(hwnd, &mut ps);
    if hdc.is_null() {
        return;
    }

    let Some(client_bounds) = main_window_client_bounds(hwnd) else {
        platform_gdi::end_paint(hwnd, &ps);
        return;
    };
    let rc_client: RECT = client_bounds.into();
    let w = rc_client.right - rc_client.left;
    let h = rc_client.bottom - rc_client.top;

    let memdc = platform_gdi::create_compatible_dc(hdc);
    let membmp = platform_gdi::create_compatible_bitmap(hdc, w, h);
    let oldbmp = platform_gdi::select_object(memdc, membmp as _);

    let layout = state.layout();
    let dynamic_row_specs = windows_native_clip_row_component_specs_for_items(state.active_items());
    let row_icon_kinds = state
        .active_items()
        .iter()
        .map(|item| {
            let presentation = crate::app_core::native_host_clip_row_presentation_for_clip_item(
                item,
                is_directory_item(item),
            );
            main_row_icon_kind_for_clip_presentation(presentation.kind_icon)
        })
        .collect();
    let render_plan = layout.render_plan(MainRenderInput {
        client_rect: client_bounds,
        visible_len: state.visible_count(),
        scroll_y: state.scroll_y,
        empty_state: main_empty_state_kind(state),
        hover_idx: state.hover_idx,
        sel_idx: state.sel_idx,
        selected_rows: state.selected_rows.iter().copied().collect(),
        row_icon_kinds,
        tab_index: state.tab_index as i32,
        hover_tab: state.hover_tab,
        hover_title_button: state.hover_btn,
        down_title_button: state.down_btn,
        search_on: state.search_on,
        active_loading: state.active_load_state().loading,
        scroll_fade_alpha: state.scroll_fade_alpha,
        hover_scroll: state.hover_scroll,
        scroll_to_top_visible: scroll_to_top_visible(state),
        hover_scroll_to_top: state.hover_to_top,
        down_scroll_to_top: state.down_to_top,
        title_buttons: main_title_button_visibility(&state.settings),
    });
    for command in &render_plan.chrome_commands {
        draw_main_paint_command(memdc, *command, th);
    }
    for command in &render_plan.icon_commands {
        draw_main_icon_command(memdc, *command, dark);
    }

    for command in &render_plan.segment_commands {
        draw_main_paint_command(memdc, *command, th);
    }
    draw_main_text_commands(
        memdc,
        &render_plan.segment_text_commands,
        MainTextLayer::Content,
        th,
    );

    let saved_clip = platform_gdi::save_dc(memdc);
    let list_clip_rc: RECT = render_plan.list_clip_rect.into();
    platform_gdi::intersect_clip_rect(
        memdc,
        list_clip_rc.left,
        list_clip_rc.top,
        list_clip_rc.right,
        list_clip_rc.bottom,
    );
    if state.visible_count() == 0 {
        draw_main_text_commands(
            memdc,
            &render_plan.text_commands,
            MainTextLayer::Content,
            th,
        );
    } else {
        for command in &render_plan.row_background_commands {
            draw_main_paint_command(memdc, *command, th);
        }
        for row_plan in &render_plan.visible_rows {
            let i = row_plan.index;
            let item = state.active_items()[i as usize].clone();
            let dynamic_row_item_id = dynamic_row_specs
                .get(i as usize)
                .and_then(|spec| spec.action.has_item().then_some(spec.action.item_id))
                .unwrap_or(item.id);
            let row_presentation = crate::app_core::native_host_clip_row_presentation_for_clip_item(
                &item,
                is_directory_item(&item),
            );
            debug_assert_eq!(dynamic_row_item_id, row_presentation.item_id);

            if let Some(command) = row_plan.item_icon_command {
                draw_main_icon_command(memdc, command, dark);
            }

            let row_content = layout.row_content_plan(
                row_plan,
                MainRowContentInput {
                    pinned: row_presentation.pin_badge.is_some(),
                    show_delete: row_shows_delete_button(state, i),
                    show_preview: row_supports_image_preview(&item, &state.settings),
                },
            );

            for command in &row_content.paint_commands {
                draw_main_paint_command(memdc, *command, th);
            }
            for command in &row_content.icon_commands {
                draw_main_icon_command(memdc, *command, dark);
            }

            if let Some(preview_rc) = row_content.preview_rect {
                let preview_rc: RECT = preview_rc.into();
                let thumb_px = ((preview_rc.right - preview_rc.left)
                    .max(preview_rc.bottom - preview_rc.top)
                    + 8)
                .clamp(32, 96) as usize;
                if let Some((bytes, width, height)) =
                    ensure_item_thumbnail_bytes(state, &item, thumb_px)
                {
                    draw_rgba_image_fit(
                        memdc as _,
                        &bytes,
                        width,
                        height,
                        &preview_rc,
                        th.surface2,
                    );
                }
            }
            let display_preview: String;
            let preview_str =
                if row_presentation.kind_icon == crate::app_core::NativeHostClipKindIcon::Image {
                    display_preview =
                        format_created_at_local(&item.created_at, &row_presentation.preview);
                    &display_preview
                } else {
                    &row_presentation.preview
                };
            draw_main_row_text_command(memdc, row_content.text_command, preview_str, th);
        }

        draw_main_text_commands(
            memdc,
            &render_plan.text_commands,
            MainTextLayer::Content,
            th,
        );
    }

    for command in &render_plan.overlay_commands {
        draw_main_paint_command(memdc, *command, th);
    }

    draw_main_text_commands(
        memdc,
        &render_plan.text_commands,
        MainTextLayer::Overlay,
        th,
    );
    platform_gdi::restore_dc(memdc, saved_clip);

    platform_gdi::copy_bits(hdc, 0, 0, w, h, memdc, 0, 0);
    platform_gdi::select_object(memdc, oldbmp);
    platform_gdi::delete_object(membmp as _);
    platform_gdi::delete_dc(memdc);
    platform_gdi::end_paint(hwnd, &ps);
}

unsafe fn draw_rgba_image_fit(
    hdc: *mut core::ffi::c_void,
    bytes: &[u8],
    width: usize,
    height: usize,
    dest: &RECT,
    bg: u32,
) {
    if bytes.is_empty() || width == 0 || height == 0 {
        return;
    }
    let avail_w = (dest.right - dest.left).max(1);
    let avail_h = (dest.bottom - dest.top).max(1);
    let scale = (avail_w as f32 / width as f32)
        .min(avail_h as f32 / height as f32)
        .max(0.01);
    let draw_w = ((width as f32) * scale).round().max(1.0) as i32;
    let draw_h = ((height as f32) * scale).round().max(1.0) as i32;
    let draw_x = dest.left + (avail_w - draw_w) / 2;
    let draw_y = dest.top + (avail_h - draw_h) / 2;

    let bgra = crate::ui::rgba_to_opaque_bgra_on_bg(bytes, bg);

    platform_gdi::stretch_top_down_32bpp(
        hdc as _,
        draw_x,
        draw_y,
        draw_w,
        draw_h,
        width as i32,
        height as i32,
        &bgra,
    );
}
