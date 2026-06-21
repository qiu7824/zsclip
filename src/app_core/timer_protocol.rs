#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MainTimerTask {
    StartupRecovery,
    VvWatch,
    VvShow,
    Paste,
    SearchDebounce,
    HiddenReclaim,
    ClipboardRetry,
    DpiFit,
    ScrollFade,
    EdgeAutoHide,
    OutsideHide,
    CloudSync,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MainTimerIds {
    pub(crate) startup_recovery: usize,
    pub(crate) vv_watch: usize,
    pub(crate) vv_show: usize,
    pub(crate) paste: usize,
    pub(crate) search_debounce: usize,
    pub(crate) hidden_reclaim: usize,
    pub(crate) clipboard_retry: usize,
    pub(crate) dpi_fit: usize,
    pub(crate) scroll_fade: usize,
    pub(crate) edge_auto_hide: usize,
    pub(crate) outside_hide: usize,
    pub(crate) cloud_sync: usize,
}

pub(crate) fn main_timer_task_for_id(timer_id: usize, ids: MainTimerIds) -> Option<MainTimerTask> {
    if timer_id == ids.startup_recovery {
        Some(MainTimerTask::StartupRecovery)
    } else if timer_id == ids.vv_watch {
        Some(MainTimerTask::VvWatch)
    } else if timer_id == ids.vv_show {
        Some(MainTimerTask::VvShow)
    } else if timer_id == ids.paste {
        Some(MainTimerTask::Paste)
    } else if timer_id == ids.search_debounce {
        Some(MainTimerTask::SearchDebounce)
    } else if timer_id == ids.hidden_reclaim {
        Some(MainTimerTask::HiddenReclaim)
    } else if timer_id == ids.clipboard_retry {
        Some(MainTimerTask::ClipboardRetry)
    } else if timer_id == ids.dpi_fit {
        Some(MainTimerTask::DpiFit)
    } else if timer_id == ids.scroll_fade {
        Some(MainTimerTask::ScrollFade)
    } else if timer_id == ids.edge_auto_hide {
        Some(MainTimerTask::EdgeAutoHide)
    } else if timer_id == ids.outside_hide {
        Some(MainTimerTask::OutsideHide)
    } else if timer_id == ids.cloud_sync {
        Some(MainTimerTask::CloudSync)
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SettingsTimerTask {
    HideScrollbar,
    ClearSaveHint,
    DpiFit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SettingsTimerIds {
    pub(crate) hide_scrollbar: usize,
    pub(crate) clear_save_hint: usize,
    pub(crate) dpi_fit: usize,
}

pub(crate) fn settings_timer_task_for_id(
    timer_id: usize,
    ids: SettingsTimerIds,
) -> Option<SettingsTimerTask> {
    if timer_id == ids.hide_scrollbar {
        Some(SettingsTimerTask::HideScrollbar)
    } else if timer_id == ids.clear_save_hint {
        Some(SettingsTimerTask::ClearSaveHint)
    } else if timer_id == ids.dpi_fit {
        Some(SettingsTimerTask::DpiFit)
    } else {
        None
    }
}
