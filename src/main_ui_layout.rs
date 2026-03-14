use crate::ui_core::UiRect;

#[derive(Clone, Copy, Debug)]
pub(crate) struct MainUiLayout {
    pub(crate) win_w: i32,
    pub(crate) title_h: i32,
    pub(crate) seg_x: i32,
    pub(crate) seg_y: i32,
    pub(crate) seg_w: i32,
    pub(crate) seg_h: i32,
    pub(crate) list_x: i32,
    pub(crate) list_y: i32,
    pub(crate) list_w: i32,
    pub(crate) list_h: i32,
    pub(crate) list_pad: i32,
    pub(crate) row_h: i32,
    pub(crate) btn_w: i32,
    pub(crate) btn_gap: i32,
    pub(crate) search_left: i32,
    pub(crate) search_top: i32,
    pub(crate) search_w: i32,
    pub(crate) search_h: i32,
}

impl MainUiLayout {
    pub(crate) const fn zsclip() -> Self {
        Self {
            win_w: 300,
            title_h: 35,
            seg_x: 6,
            seg_y: 36,
            seg_w: 288,
            seg_h: 30,
            list_x: 6,
            list_y: 70,
            list_w: 288,
            list_h: 538,
            list_pad: 4,
            row_h: 44,
            btn_w: 32,
            btn_gap: 2,
            search_left: 58,
            search_top: 4,
            search_w: 112,
            search_h: 30,
        }
    }

    pub(crate) fn list_view_height(self) -> i32 {
        self.list_h - 2 * self.list_pad
    }

    pub(crate) fn total_content_height(self, filtered_len: usize) -> i32 {
        filtered_len as i32 * self.row_h
    }

    pub(crate) fn max_scroll(self, filtered_len: usize) -> i32 {
        (self.total_content_height(filtered_len) - self.list_view_height()).max(0)
    }

    pub(crate) fn clamp_scroll(self, scroll_y: i32, filtered_len: usize) -> i32 {
        scroll_y.clamp(0, self.max_scroll(filtered_len))
    }

    pub(crate) fn ensure_visible(self, scroll_y: i32, idx: i32, filtered_len: usize) -> i32 {
        if idx < 0 {
            return self.clamp_scroll(scroll_y, filtered_len);
        }
        let top = idx * self.row_h;
        let bottom = top + self.row_h;
        let view_top = scroll_y;
        let view_bottom = scroll_y + self.list_view_height();
        let next = if top < view_top {
            top
        } else if bottom > view_bottom {
            bottom - self.list_view_height()
        } else {
            scroll_y
        };
        self.clamp_scroll(next, filtered_len)
    }

    pub(crate) fn row_rect(self, visible_idx: i32, filtered_len: usize, scroll_y: i32) -> Option<UiRect> {
        if visible_idx < 0 || visible_idx >= filtered_len as i32 {
            return None;
        }
        let inner_l = self.list_x + self.list_pad;
        let inner_t = self.list_y + self.list_pad;
        let y0 = inner_t + visible_idx * self.row_h - scroll_y;
        Some(UiRect::new(
            inner_l,
            y0,
            inner_l + self.list_w - 2 * self.list_pad,
            y0 + self.row_h,
        ))
    }

    pub(crate) fn quick_delete_rect(self, visible_idx: i32, filtered_len: usize, scroll_y: i32) -> Option<UiRect> {
        let row = self.row_rect(visible_idx, filtered_len, scroll_y)?;
        let size = 16;
        let left = row.right - 10 - size - 12;
        let top = row.top + (self.row_h - size) / 2;
        Some(UiRect::new(left, top, left + size, top + size))
    }

    pub(crate) fn search_rect(self) -> UiRect {
        UiRect::new(
            self.search_left,
            self.search_top,
            self.search_left + self.search_w,
            self.search_top + self.search_h,
        )
    }

    pub(crate) fn title_button_rect(self, key: &str) -> UiRect {
        let x_close = self.win_w - 4 - self.btn_w;
        let x_min = x_close - self.btn_gap - self.btn_w;
        let x_set = x_min - self.btn_gap - self.btn_w;
        let x_search = x_set - self.btn_gap - self.btn_w;
        let x = match key {
            "search" => x_search,
            "setting" => x_set,
            "min" => x_min,
            _ => x_close,
        };
        let top = (self.title_h - self.btn_w) / 2;
        UiRect::new(x, top, x + self.btn_w, top + self.btn_w)
    }

    pub(crate) fn segment_rects(self) -> (UiRect, UiRect) {
        let inner_l = self.seg_x + 1;
        let inner_t = self.seg_y + 1;
        let inner_w = self.seg_w - 2;
        let inner_h = self.seg_h - 2;
        let gap = 1;
        let btn_w = (inner_w - gap) / 2;
        (
            UiRect::new(inner_l, inner_t, inner_l + btn_w, inner_t + inner_h),
            UiRect::new(inner_l + btn_w + gap, inner_t, inner_l + inner_w, inner_t + inner_h),
        )
    }

    pub(crate) fn scrollbar_track_rect(self, filtered_len: usize) -> Option<UiRect> {
        if self.total_content_height(filtered_len) <= self.list_view_height() {
            return None;
        }
        Some(UiRect::new(
            self.list_x + self.list_w - self.list_pad - 8 - 2,
            self.list_y + self.list_pad + 2,
            self.list_x + self.list_w - self.list_pad - 2,
            self.list_y + self.list_h - self.list_pad - 2,
        ))
    }

    pub(crate) fn scrollbar_thumb_rect(self, filtered_len: usize, scroll_y: i32) -> Option<UiRect> {
        let track = self.scrollbar_track_rect(filtered_len)?;
        let track_h = track.bottom - track.top;
        let total_h = self.total_content_height(filtered_len);
        let view_h = self.list_view_height();
        let thumb_h = (track_h as f32 * (view_h as f32 / total_h as f32)) as i32;
        let thumb_h = thumb_h.max(28);
        let max_scroll = self.max_scroll(filtered_len).max(1);
        let thumb_y = track.top + ((track_h - thumb_h) as f32 * (scroll_y as f32 / max_scroll as f32)) as i32;
        Some(UiRect::new(track.left + 1, thumb_y, track.right - 1, thumb_y + thumb_h))
    }
}
