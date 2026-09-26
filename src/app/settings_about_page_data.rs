use super::prelude::*;

pub(super) const ABOUT_DATA_RETENTION_EN: &str = "History uses the record limit and a 512 MB payload budget, keeping the newest item. Unlimited disables pruning. Pinned items and phrases are retained. Database free pages are reused; unused images are reclaimed later.";

pub(super) unsafe fn settings_create_about_data_section(
    st: &mut SettingsWndState,
    b: &SettingsPageBuilder,
    flow: &mut SettingsFlowLayout,
) {
    let directory_text = format!(
        "{}{}",
        tr("数据目录：", "Data directory: "),
        data_dir().to_string_lossy()
    );
    let directory_rect = flow.full_rect(settings_scale(28));
    b.label(
        st,
        &directory_text,
        directory_rect.left,
        directory_rect.top,
        directory_rect.width(),
        directory_rect.height(),
    );
    flow.consume_full(directory_rect.height(), settings_scale(4));

    let info_text = tr(
        "普通历史按条数和 512 MB 内容预算清理，至少保留最新一条；条数不限时不自动清理。置顶和常用短语保留。数据库空闲页复用，未引用图片延迟回收。",
        ABOUT_DATA_RETENTION_EN,
    );
    let info_rect = flow.full_rect(settings_scale(24));
    b.label_auto(
        st,
        info_text,
        info_rect.left,
        info_rect.top,
        info_rect.right - info_rect.left,
        info_rect.height(),
    );
}
