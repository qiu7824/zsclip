use super::prelude::*;

pub(super) unsafe fn settings_create_hotkey_system_controls(
    st: &mut SettingsWndState,
    b: &SettingsPageBuilder,
    sec1: SettingsFormSectionLayout,
    sec2: SettingsFormSectionLayout,
    line_h: i32,
    note_h: i32,
    small_gap: i32,
) {
    let _ = b.label_auto(st, "说明：通过注册表 DisabledHotkeys 屏蔽或恢复 Win+V。修改后通常需要重启资源管理器或重新登录。", sec1.left(), sec1.row_y(0), sec1.full_w(), note_h);
    st.btn_clip_hist_block = b.button(
        st,
        "屏蔽 Win+V",
        6111,
        sec1.action_x(0, settings_scale(110)),
        sec1.row_y(1),
        settings_scale(110),
    );
    st.btn_clip_hist_restore = b.button(
        st,
        "恢复 Win+V",
        6112,
        sec1.action_x(1, settings_scale(110)),
        sec1.row_y(1),
        settings_scale(110),
    );
    st.btn_restart_explorer = b.button(
        st,
        "重启资源管理器",
        6113,
        sec1.action_x(2, settings_scale(130)),
        sec1.row_y(1),
        settings_scale(130),
    );
    for &hh in &[
        st.btn_clip_hist_block,
        st.btn_clip_hist_restore,
        st.btn_restart_explorer,
    ] {
        if !hh.is_null() {
            st.ownerdraw_ctrls.push(hh);
        }
    }

    let (_desc1, d1h) = b.label_auto(
        st,
        "说明：保存后会立即重新注册主快捷键。",
        sec2.left(),
        sec2.row_y(0),
        sec2.full_w(),
        line_h,
    );
    let _ = b.label_auto(
        st,
        "建议避免使用 Ctrl+C / Ctrl+V 等系统级常用组合。",
        sec2.left(),
        sec2.row_y(0) + d1h + small_gap,
        sec2.full_w(),
        line_h,
    );
}
