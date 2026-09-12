use super::prelude::*;

pub(super) unsafe fn settings_create_about_data_section(
    st: &mut SettingsWndState,
    b: &SettingsPageBuilder,
    flow: &mut SettingsFlowLayout,
) {
    let info_text = format!(
        "{}{}",
        tr("数据目录：", "Data directory: "),
        data_dir().to_string_lossy()
    );
    let info_text = format!("{}\n{}", info_text, tr(
        "普通历史按保存条数及 512 MB 内容预算清理（至少保留最新一条）；不限条数时关闭自动清理。置顶和常用短语保留。数据库空闲页会复用，未引用图片延迟回收。",
        "History uses the record limit and a 512 MB payload budget (keeping the newest item). Unlimited disables pruning. Pinned items and phrases are retained. Database free pages are reused; unreferenced images are reclaimed later."));
    let info_rect = flow.full_rect(settings_scale(96));
    let _ = b.label_auto(
        st,
        &info_text,
        info_rect.left,
        info_rect.top,
        info_rect.right - info_rect.left,
        settings_scale(96),
    );
}
