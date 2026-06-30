pub use crate::win_ui_render::{
    draw_icon_tinted_soft, draw_round_fill, draw_round_rect, draw_text, draw_text_block,
    draw_text_block_ex, draw_text_ex,
};

pub fn rgba_to_opaque_bgra_on_bg(bytes: &[u8], bg: u32) -> Vec<u8> {
    if bytes.len() < 4 {
        return bytes.to_vec();
    }
    let br = (bg & 0xFF) as u32;
    let bg_g = ((bg >> 8) & 0xFF) as u32;
    let bb = ((bg >> 16) & 0xFF) as u32;
    let mut out = Vec::with_capacity(bytes.len());
    let mut chunks = bytes.chunks_exact(4);
    for px in &mut chunks {
        let a = px[3] as u32;
        let inv = 255 - a;
        let r = ((px[0] as u32 * a + br * inv + 127) / 255) as u8;
        let g = ((px[1] as u32 * a + bg_g * inv + 127) / 255) as u8;
        let b = ((px[2] as u32 * a + bb * inv + 127) / 255) as u8;
        out.extend_from_slice(&[b, g, r, 255]);
    }
    out.extend_from_slice(chunks.remainder());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_core::{
        clamp_window_pos_to_rect, dpi_compensated_size, ClipKindFilter, ClipListState,
        DpiCompensationPlan, DpiCompensationState, ItemsCursor, ItemsQuery, MainFrameHitTarget,
        MainPointerDownTarget, MainUiLayout, SharedTabViewState, TabLoadState,
        TitleButtonVisibility, UiRect,
    };
    use crate::win_native_style::rgb;

    fn test_main_layout() -> MainUiLayout {
        MainUiLayout {
            win_w: 400,
            title_h: 40,
            seg_x: 0,
            seg_y: 0,
            seg_w: 200,
            seg_h: 32,
            list_x: 0,
            list_y: 0,
            list_w: 240,
            list_h: 220,
            list_pad: 10,
            row_h: 20,
            btn_w: 32,
            btn_gap: 4,
            search_left: 0,
            search_top: 0,
            search_w: 100,
            search_h: 28,
        }
    }

    fn hover_test_layout() -> MainUiLayout {
        MainUiLayout {
            win_w: 400,
            title_h: 40,
            seg_x: 20,
            seg_y: 50,
            seg_w: 200,
            seg_h: 32,
            list_x: 20,
            list_y: 100,
            list_w: 260,
            list_h: 220,
            list_pad: 10,
            row_h: 20,
            btn_w: 32,
            btn_gap: 4,
            search_left: 70,
            search_top: 6,
            search_w: 120,
            search_h: 28,
        }
    }

    #[test]
    fn rgba_alpha_composites_to_opaque_bgra_without_channel_swap_error() {
        let out = rgba_to_opaque_bgra_on_bg(&[255, 0, 0, 128], rgb(255, 255, 255));
        assert_eq!(out, vec![127, 127, 255, 255]);
    }

    #[test]
    fn ui_facade_does_not_export_win32_drawtext_flags() {
        let source = include_str!("ui.rs");
        let forbidden = [
            format!("{}{}", "pub const ", "DT_"),
            format!("{}{}", "TRANS", "PARENT"),
        ];
        for token in forbidden {
            assert!(!source.contains(&token), "{token}");
        }
    }

    #[test]
    fn window_position_clamps_inside_bounds_even_when_window_is_larger() {
        let bounds = UiRect::new(100, 200, 500, 600);
        assert_eq!(
            clamp_window_pos_to_rect(50, 700, bounds, 200, 100),
            (100, 500)
        );
        assert_eq!(
            clamp_window_pos_to_rect(450, 100, bounds, 700, 900),
            (100, 200)
        );
    }

    #[test]
    fn dpi_compensated_size_preserves_existing_monitor_ratio_rule() {
        assert_eq!(dpi_compensated_size(800, 600, 96, 144), (533, 400));
        assert_eq!(dpi_compensated_size(800, 600, 144, 96), (1200, 900));
        assert_eq!(dpi_compensated_size(0, -5, 0, 0), (1, 1));
    }

    #[test]
    fn dpi_compensation_state_tracks_base_target_and_resize_completion() {
        let mut state = DpiCompensationState::default();
        assert_eq!(state.target_size(144), None);
        assert!(state.ensure_base(800, 600, 96));
        assert!(!state.ensure_base(1024, 768, 144));
        assert_eq!(state.target_size(144), Some((533, 400)));
        assert!(!state.already_at_target(144, 530, 400, 533, 400, 2));
        assert!(state.already_at_target(96, 800, 601, 800, 600, 2));

        state.set_applying(true);
        assert!(state.is_applying());
        state.finish_resize(144);
        assert!(!state.is_applying());
        assert!(state.already_at_target(144, 533, 400, 533, 400, 2));

        state.reset();
        assert_eq!(state.target_size(96), None);
    }

    #[test]
    fn dpi_compensation_resize_plan_clamps_to_bounds_and_skips_repeats() {
        let mut state = DpiCompensationState::default();
        let current = UiRect::new(100, 100, 900, 700);
        let bounds = UiRect::new(0, 0, 700, 500);

        assert_eq!(state.resize_plan(current, bounds, 96, 2), None);
        let plan = state.resize_plan(current, bounds, 144, 2).unwrap();
        assert_eq!(
            plan,
            DpiCompensationPlan {
                x: 167,
                y: 100,
                width: 533,
                height: 400,
                monitor_dpi: 144,
            }
        );
        state.finish_resize(plan.monitor_dpi);

        assert_eq!(
            state.resize_plan(UiRect::new(100, 100, 633, 500), bounds, 144, 2),
            None
        );

        let clamped = state
            .resize_plan(UiRect::new(500, 420, 1033, 820), bounds, 96, 2)
            .unwrap();
        assert_eq!(clamped.x, 0);
        assert_eq!(clamped.y, 0);
        assert_eq!(clamped.width, 700);
        assert_eq!(clamped.height, 500);
    }

    #[test]
    fn shared_tab_view_state_defaults_to_records_without_group_filters() {
        let state = SharedTabViewState::default();
        assert_eq!(state.tab_index, 0);
        assert_eq!(state.tab_group_filters, [0, 0]);
    }

    #[test]
    fn items_query_for_tab_applies_group_filter_and_trims_search() {
        assert_eq!(
            ItemsQuery::for_tab(1, true, [3, 8], [ClipKindFilter::All; 2], "  bili  "),
            ItemsQuery {
                category: 1,
                group_id: 8,
                search_text: "bili".to_string(),
                kind_filter: ClipKindFilter::All,
                near_query: None,
            }
        );
        assert_eq!(
            ItemsQuery::for_tab(1, false, [3, 8], [ClipKindFilter::All; 2], "  catsxp  "),
            ItemsQuery {
                category: 1,
                group_id: 0,
                search_text: "catsxp".to_string(),
                kind_filter: ClipKindFilter::All,
                near_query: None,
            }
        );
        assert_eq!(
            ItemsQuery::for_tab(9, true, [3, 8], [ClipKindFilter::All; 2], "  all  "),
            ItemsQuery {
                category: 9,
                group_id: 0,
                search_text: "all".to_string(),
                kind_filter: ClipKindFilter::All,
                near_query: None,
            }
        );
    }

    #[test]
    fn tab_load_state_defaults_to_ready_for_first_page() {
        let load = TabLoadState::default();
        assert_eq!(load.query, None);
        assert_eq!(load.next_cursor, None);
        assert!(load.has_more);
        assert!(!load.loading);
        assert_eq!(load.request_seq, 0);
        assert_eq!(load.error, None);
    }

    #[test]
    fn tab_load_state_tracks_request_identity_and_result_acceptance() {
        let mut load = TabLoadState::default();
        load.next_cursor = Some(ItemsCursor {
            pinned: true,
            id: 42,
        });
        load.has_more = false;

        let query = ItemsQuery {
            category: 0,
            group_id: 7,
            search_text: "hello".to_string(),
            kind_filter: ClipKindFilter::All,
            near_query: None,
        };
        let seq = load.begin_request(query.clone(), true);

        assert_eq!(seq, 1);
        assert_eq!(load.query, Some(query.clone()));
        assert_eq!(load.next_cursor, None);
        assert!(load.has_more);
        assert!(load.loading);
        assert!(load.accepts_result(seq, &query));
        assert!(!load.accepts_result(seq + 1, &query));
        assert!(!load.accepts_result(
            seq,
            &ItemsQuery {
                category: 1,
                group_id: 7,
                search_text: "hello".to_string(),
                kind_filter: ClipKindFilter::All,
                near_query: None,
            },
        ));
    }

    #[test]
    fn tab_load_state_finish_and_invalidate_reset_transient_state() {
        let mut load = TabLoadState::default();
        let query = ItemsQuery {
            category: 1,
            group_id: 0,
            search_text: String::new(),
            kind_filter: ClipKindFilter::All,
            near_query: None,
        };
        load.begin_request(query, false);
        load.finish_request(
            Some("failed".to_string()),
            Some(ItemsCursor {
                pinned: false,
                id: 12,
            }),
            false,
        );

        assert!(!load.loading);
        assert_eq!(load.error.as_deref(), Some("failed"));
        assert_eq!(
            load.next_cursor,
            Some(ItemsCursor {
                pinned: false,
                id: 12,
            })
        );
        assert!(!load.has_more);

        load.invalidate();
        assert_eq!(load.request_seq, 2);
        assert_eq!(load.query, None);
        assert_eq!(load.next_cursor, None);
        assert!(load.has_more);
        assert!(!load.loading);
        assert_eq!(load.error, None);
    }

    #[test]
    fn primary_pointer_selection_handles_ctrl_and_shift() {
        let mut list = ClipListState::default();
        list.apply_visible_len(8);

        list.apply_primary_pointer_selection(2, true, false);
        assert_eq!(list.sel_idx, 2);
        assert_eq!(list.selection_anchor, 2);
        assert_eq!(list.selected_visible_rows(), vec![2]);

        list.apply_primary_pointer_selection(4, true, false);
        assert_eq!(list.sel_idx, 4);
        assert_eq!(list.selection_anchor, 4);
        assert_eq!(list.selected_visible_rows(), vec![2, 4]);

        list.apply_primary_pointer_selection(6, false, true);
        assert_eq!(list.sel_idx, 6);
        assert_eq!(list.selection_anchor, 4);
        assert_eq!(list.selected_visible_rows(), vec![4, 5, 6]);
    }

    #[test]
    fn context_pointer_selection_preserves_existing_multi_selection() {
        let mut list = ClipListState::default();
        list.apply_visible_len(8);
        list.apply_primary_pointer_selection(2, true, false);
        list.apply_primary_pointer_selection(4, true, false);

        list.apply_context_pointer_selection(2, false, false);
        assert_eq!(list.sel_idx, 2);
        assert_eq!(list.selected_visible_rows(), vec![2, 4]);

        list.apply_context_pointer_selection(6, false, false);
        assert_eq!(list.sel_idx, 6);
        assert_eq!(list.selection_anchor, 6);
        assert_eq!(list.selected_visible_rows(), vec![6]);
    }

    #[test]
    fn context_pointer_selection_extends_from_anchor_with_shift() {
        let mut list = ClipListState::default();
        list.apply_visible_len(8);
        list.apply_context_pointer_selection(2, false, false);

        list.apply_context_pointer_selection(5, false, true);
        assert_eq!(list.sel_idx, 5);
        assert_eq!(list.selection_anchor, 2);
        assert_eq!(list.selected_visible_rows(), vec![2, 3, 4, 5]);

        list.apply_context_pointer_selection(4, true, false);
        assert_eq!(list.sel_idx, 4);
        assert_eq!(list.selection_anchor, 2);
        assert_eq!(
            list.selected_rows.iter().copied().collect::<Vec<_>>(),
            vec![2, 3, 5]
        );
        assert_eq!(list.selected_visible_rows(), vec![2, 3, 4, 5]);
    }

    #[test]
    fn main_layout_wheel_scroll_target_clamps_and_preserves_direction() {
        let layout = test_main_layout();
        assert_eq!(layout.max_scroll(30), 400);
        assert_eq!(layout.wheel_scroll_target(100, 30, 120), 60);
        assert_eq!(layout.wheel_scroll_target(100, 30, -120), 140);
        assert_eq!(layout.wheel_scroll_target(10, 30, 120), 0);
        assert_eq!(layout.wheel_scroll_target(390, 30, -120), 400);
        assert_eq!(layout.wheel_scroll_target(100, 5, -120), 0);
    }

    #[test]
    fn main_layout_scrollbar_track_click_maps_pointer_to_scroll_range() {
        let layout = test_main_layout();
        let track = layout.scrollbar_track_rect(30).unwrap();

        assert_eq!(
            layout.scrollbar_track_click_scroll_target(30, track.top),
            Some(0)
        );
        assert_eq!(
            layout.scrollbar_track_click_scroll_target(30, track.bottom),
            Some(layout.max_scroll(30))
        );
        assert_eq!(
            layout.scrollbar_track_click_scroll_target(5, track.top),
            None
        );
    }

    #[test]
    fn main_layout_scrollbar_drag_target_uses_drag_delta_and_clamps() {
        let layout = test_main_layout();
        let track = layout.scrollbar_track_rect(30).unwrap();
        let thumb = layout.scrollbar_thumb_rect(30, 100).unwrap();
        let drag_range = (track.bottom - track.top) - (thumb.bottom - thumb.top);

        assert_eq!(
            layout.scrollbar_drag_scroll_target(30, 50, 100, 50),
            Some(100)
        );
        assert_eq!(
            layout.scrollbar_drag_scroll_target(30, 50, 100, 50 + drag_range),
            Some(layout.max_scroll(30))
        );
        assert_eq!(
            layout.scrollbar_drag_scroll_target(30, 50, 100, 50 - drag_range),
            Some(0)
        );
        assert_eq!(layout.scrollbar_drag_scroll_target(5, 50, 100, 60), None);
    }

    #[test]
    fn main_layout_hover_target_finds_title_buttons_and_respects_visibility() {
        let layout = hover_test_layout();
        let close = layout.title_button_rect("close");
        let target = layout.hover_target(
            close.left + 1,
            close.top + 1,
            20,
            0,
            TitleButtonVisibility::default(),
            false,
        );
        assert_eq!(target.title_button, "close");

        let target = layout.hover_target(
            close.left + 1,
            close.top + 1,
            20,
            0,
            TitleButtonVisibility {
                close: false,
                ..TitleButtonVisibility::default()
            },
            false,
        );
        assert_eq!(target.title_button, "");
    }

    #[test]
    fn main_layout_hover_target_finds_tabs_scrollbar_and_rows() {
        let layout = hover_test_layout();
        let (records, phrases) = layout.segment_rects();
        assert_eq!(
            layout
                .hover_target(
                    records.left + 1,
                    records.top + 1,
                    20,
                    0,
                    TitleButtonVisibility::default(),
                    false
                )
                .tab,
            0
        );
        assert_eq!(
            layout
                .hover_target(
                    phrases.left + 1,
                    phrases.top + 1,
                    20,
                    0,
                    TitleButtonVisibility::default(),
                    false
                )
                .tab,
            1
        );

        let row = layout
            .hover_target(
                layout.list_x + layout.list_pad + 1,
                layout.list_y + layout.list_pad + layout.row_h + 1,
                20,
                0,
                TitleButtonVisibility::default(),
                false,
            )
            .row;
        assert_eq!(row, 1);

        let track = layout.scrollbar_track_rect(30).unwrap();
        let target = layout.hover_target(
            track.left - 7,
            track.top + 1,
            30,
            0,
            TitleButtonVisibility::default(),
            false,
        );
        assert!(target.scrollbar);
    }

    #[test]
    fn main_layout_hover_target_prioritizes_scroll_to_top_over_row_hover() {
        let layout = hover_test_layout();
        let button = layout.scroll_to_top_button_rect();
        let target = layout.hover_target(
            button.left + 1,
            button.top + 1,
            30,
            100,
            TitleButtonVisibility::default(),
            true,
        );
        assert!(target.scroll_to_top);
        assert_eq!(target.row, -1);
    }

    #[test]
    fn main_layout_pointer_down_target_finds_title_drag_and_buttons() {
        let layout = hover_test_layout();
        assert_eq!(
            layout.pointer_down_target(
                10,
                10,
                20,
                0,
                TitleButtonVisibility::default(),
                false,
                false
            ),
            MainPointerDownTarget::TitleDrag
        );
        assert_eq!(
            layout.pointer_down_target(
                layout.search_left + 1,
                layout.search_top + 1,
                20,
                0,
                TitleButtonVisibility::default(),
                true,
                false
            ),
            MainPointerDownTarget::None
        );
        let close = layout.title_button_rect("close");
        assert_eq!(
            layout.pointer_down_target(
                close.left + 1,
                close.top + 1,
                20,
                0,
                TitleButtonVisibility::default(),
                false,
                false
            ),
            MainPointerDownTarget::TitleButton("close")
        );
    }

    #[test]
    fn main_layout_pointer_down_target_finds_scrollbar_and_tabs() {
        let layout = hover_test_layout();
        let thumb = layout.scrollbar_thumb_rect(30, 0).unwrap();
        assert_eq!(
            layout.pointer_down_target(
                thumb.left - 7,
                thumb.top + 1,
                30,
                0,
                TitleButtonVisibility::default(),
                false,
                false
            ),
            MainPointerDownTarget::ScrollbarThumb
        );
        let track = layout.scrollbar_track_rect(30).unwrap();
        assert_eq!(
            layout.pointer_down_target(
                track.left - 7,
                thumb.bottom + 8,
                30,
                0,
                TitleButtonVisibility::default(),
                false,
                false
            ),
            MainPointerDownTarget::ScrollbarTrack
        );

        let (records, phrases) = layout.segment_rects();
        assert_eq!(
            layout.pointer_down_target(
                records.left + 1,
                records.top + 1,
                30,
                0,
                TitleButtonVisibility::default(),
                false,
                false
            ),
            MainPointerDownTarget::Tab(0)
        );
        assert_eq!(
            layout.pointer_down_target(
                phrases.left + 1,
                phrases.top + 1,
                30,
                0,
                TitleButtonVisibility::default(),
                false,
                false
            ),
            MainPointerDownTarget::Tab(1)
        );
    }

    #[test]
    fn main_layout_pointer_down_target_finds_scroll_to_top_and_rows() {
        let layout = hover_test_layout();
        let button = layout.scroll_to_top_button_rect();
        assert_eq!(
            layout.pointer_down_target(
                button.left + 1,
                button.top + 1,
                30,
                100,
                TitleButtonVisibility::default(),
                false,
                true
            ),
            MainPointerDownTarget::ScrollToTop
        );

        assert_eq!(
            layout.pointer_down_target(
                layout.list_x + layout.list_pad + 1,
                layout.list_y + layout.list_pad + layout.row_h + 1,
                30,
                0,
                TitleButtonVisibility::default(),
                false,
                false
            ),
            MainPointerDownTarget::Row(1)
        );
    }

    #[test]
    fn main_layout_frame_hit_target_maps_title_bar_to_caption_only_when_draggable() {
        let layout = hover_test_layout();
        assert_eq!(
            layout.frame_hit_target(10, 10, TitleButtonVisibility::default(), false, true),
            MainFrameHitTarget::Caption
        );
        assert_eq!(
            layout.frame_hit_target(10, 10, TitleButtonVisibility::default(), false, false),
            MainFrameHitTarget::Client
        );
        assert_eq!(
            layout.frame_hit_target(
                10,
                layout.title_h + 1,
                TitleButtonVisibility::default(),
                false,
                true
            ),
            MainFrameHitTarget::Client
        );
    }

    #[test]
    fn main_layout_frame_hit_target_keeps_title_controls_client() {
        let layout = hover_test_layout();
        assert_eq!(
            layout.frame_hit_target(
                layout.search_left + 1,
                layout.search_top + 1,
                TitleButtonVisibility::default(),
                true,
                true
            ),
            MainFrameHitTarget::Client
        );

        let close = layout.title_button_rect("close");
        assert_eq!(
            layout.frame_hit_target(
                close.left + 1,
                close.top + 1,
                TitleButtonVisibility::default(),
                false,
                true
            ),
            MainFrameHitTarget::Client
        );
        assert_eq!(
            layout.frame_hit_target(
                close.left + 1,
                close.top + 1,
                TitleButtonVisibility {
                    close: false,
                    ..TitleButtonVisibility::default()
                },
                false,
                true
            ),
            MainFrameHitTarget::Caption
        );
    }
}
