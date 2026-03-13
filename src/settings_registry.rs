use windows_sys::Win32::Foundation::HWND;

pub const SETTINGS_PAGE_COUNT: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsPage {
    General = 0,
    Hotkey = 1,
    Plugin = 2,
    Group = 3,
    Cloud = 4,
    About = 5,
}

#[allow(dead_code)]
impl SettingsPage {
    pub const fn index(self) -> usize { self as usize }
    pub const fn is_scroll_page(self) -> bool { matches!(self, SettingsPage::General) }
    pub const fn title(self) -> &'static str {
        match self {
            SettingsPage::General => "常规",
            SettingsPage::Hotkey => "快捷键",
            SettingsPage::Plugin => "插件",
            SettingsPage::Group => "分组",
            SettingsPage::Cloud => "云同步",
            SettingsPage::About => "关于",
        }
    }
    pub fn from_index(index: usize) -> Self {
        match index {
            1 => SettingsPage::Hotkey,
            2 => SettingsPage::Plugin,
            3 => SettingsPage::Group,
            4 => SettingsPage::Cloud,
            5 => SettingsPage::About,
            _ => SettingsPage::General,
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub struct SettingsCtrlReg {
    pub hwnd: HWND,
    pub page: usize,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub scrollable: bool,
}

impl SettingsCtrlReg {
    pub const fn new(hwnd: HWND, page: usize, x: i32, y: i32, w: i32, h: i32, scrollable: bool) -> Self {
        Self { hwnd, page, x, y, w, h, scrollable }
    }
}

pub struct SettingsUiRegistry {
    built_pages: [bool; SETTINGS_PAGE_COUNT],
    page_ctrls: Vec<Vec<HWND>>,
    regs: Vec<SettingsCtrlReg>,
    scroll_ctrls: Vec<(HWND, i32, i32, i32, i32)>,
}

impl SettingsUiRegistry {
    pub fn new() -> Self {
        Self {
            built_pages: [false; SETTINGS_PAGE_COUNT],
            page_ctrls: vec![Vec::new(); SETTINGS_PAGE_COUNT],
            regs: Vec::new(),
            scroll_ctrls: Vec::new(),
        }
    }

    pub fn is_built(&self, page: usize) -> bool {
        self.built_pages.get(page).copied().unwrap_or(false)
    }

    pub fn mark_built(&mut self, page: usize) {
        if let Some(slot) = self.built_pages.get_mut(page) {
            *slot = true;
        }
    }

    pub fn register(&mut self, reg: SettingsCtrlReg) {
        let page = reg.page.min(SETTINGS_PAGE_COUNT.saturating_sub(1));
        self.regs.push(reg);
        if let Some(list) = self.page_ctrls.get_mut(page) {
            list.push(reg.hwnd);
        }
        if page == SettingsPage::General.index() && reg.scrollable {
            self.scroll_ctrls.push((reg.hwnd, reg.x, reg.y, reg.w, reg.h));
        }
    }

    pub fn regs(&self) -> &[SettingsCtrlReg] { &self.regs }
    #[allow(dead_code)]
    pub fn page_ctrls(&self, page: usize) -> &[HWND] {
        self.page_ctrls.get(page).map(|v| v.as_slice()).unwrap_or(&[])
    }
    pub fn page_regs(&self, page: usize) -> impl Iterator<Item = &SettingsCtrlReg> {
        self.regs.iter().filter(move |reg| reg.page == page)
    }
    pub fn scroll_ctrls(&self) -> &[(HWND, i32, i32, i32, i32)] { &self.scroll_ctrls }
}
