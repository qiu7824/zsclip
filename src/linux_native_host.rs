use crate::app_core::{
    NativeHostDialogAction, NativeHostRowAction, NativeHostSearchTextAction,
    NativeHostSettingsAction, NativeHostSettingsPlatformAction, NativeHostStatusMenuAction,
    NativeHostUiAction, NativeHostVvPasteExecution, NativeHostVvTriggerInput,
    NativeHostVvTriggerTransition, ProductAdapterAsyncBridgeResult, ProductAdapterCommandResult,
};
use crate::linux_app::LinuxHostContractSummary;

#[cfg(target_os = "linux")]
mod gtk_host {
    use std::cell::{Cell, RefCell};
    use std::process::Command;
    use std::rc::Rc;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    use gtk4 as gtk;

    use crate::app_core::{
        main_group_filter_selection_for_id, main_row_group_selection_for_id, menu_ids,
        native_host_clip_row_presentation_for_projection, native_host_clip_row_specs,
        native_host_dialog_button_specs, native_host_edit_text_button_specs,
        native_host_edit_text_close_plan, native_host_edit_text_plan_for_item,
        native_host_filtered_projected_clip_item_ids,
        native_host_full_row_popup_menu_entries_for_groups,
        native_host_group_filter_label_for_groups,
        native_host_group_filter_popup_menu_entries_for_groups,
        native_host_main_action_button_specs, native_host_main_tool_button_specs,
        native_host_reconciled_selected_item_id, native_host_row_action_button_specs,
        native_host_row_popup_menu_input_for_projection, native_host_search_input_specs,
        native_host_settings_action_button_specs, native_host_settings_control_button_specs,
        native_host_settings_dropdown_specs, native_host_settings_group_button_specs,
        native_host_settings_page_tab_specs, native_host_settings_platform_button_specs,
        native_host_settings_section_label, native_host_settings_toggle_specs,
        native_host_status_menu_item_specs, native_host_vv_popup_render_plan_for_projection,
        native_popup_menu_command_accelerator_label, native_popup_menu_command_icon_name,
        MainGroupFilterSelection, MainRowGroupSelection, NativeButtonStyleRole,
        NativeComponentAction, NativeDialogResponse, NativeHostClipKindIcon,
        NativeHostClipListItemProjection, NativeHostClipRowPresentation, NativeHostDialogAction,
        NativeHostEditTextAction, NativeHostEditTextPlan, NativeHostMainToolAction,
        NativeHostRowAction, NativeHostSearchTextAction, NativeHostSettingsAction,
        NativeHostSettingsControlAction, NativeHostSettingsGroupAction,
        NativeHostSettingsPlatformAction, NativeHostVvPasteExecution, NativeHostVvTriggerAction,
        NativeHostVvTriggerInput, NativeHostVvTriggerKey, NativeHostVvTriggerTransition,
        NativePopupMenuEntry, NativeSettingsPageTabKind, ProductAdapterCommandResult,
        SettingsControlRole, NATIVE_HOST_CLIP_ROW_CAPACITY,
    };
    use gtk::prelude::*;
    use gtk::{gdk, gio, glib};
    use gtk::{
        Application, ApplicationWindow, Box as GtkBox, Button, ButtonsType, DropDown, Entry,
        EventControllerKey, GestureClick, HeaderBar, Image, Label, ListBox, ListBoxRow, MenuButton,
        MessageDialog, MessageType, Notebook, Orientation, PolicyType, PopoverMenu, Revealer,
        RevealerTransitionType, ScrolledWindow, SearchEntry, SelectionMode, Switch, TextView,
        ToggleButton,
    };
    use ksni::blocking::TrayMethods;

    use crate::linux_app::LinuxHostContractSummary;

    const ZSCLIP_GTK_CSS: &str = r#"
.clip-row {
    padding: 8px 12px;
}

.clip-row-content {
    padding: 6px 8px;
}

.clip-list {
    margin-bottom: 8px;
}

.clip-row:hover {
    background: alpha(@theme_fg_color, 0.05);
}

.clip-list row:selected,
.clip-list row:selected .clip-row-content {
    background: alpha(@accent_color, 0.22);
}

.clip-row-pin {
    opacity: 0.75;
}

.settings-row {
    padding: 10px 12px;
    border-bottom: 1px solid alpha(@theme_fg_color, 0.10);
}

.vv-popup {
    background: @theme_bg_color;
    border: 1px solid @borders;
    border-radius: 12px;
}

.vv-index {
    color: @accent_color;
    font-size: 22px;
    font-weight: 700;
    min-width: 32px;
}

.vv-preview {
    font-family: monospace;
}

searchentry {
    margin-bottom: 8px;
}
"#;

    struct ZsclipGtkStatusNotifier {
        sender: mpsc::Sender<crate::app_core::NativeHostStatusMenuAction>,
        tooltip: String,
    }

    #[derive(Default)]
    struct GtkVvKeytapModifierState {
        control_left: bool,
        control_right: bool,
        alt_left: bool,
        alt_right: bool,
        meta_left: bool,
        meta_right: bool,
    }

    impl GtkVvKeytapModifierState {
        fn apply_event(&mut self, event: keytap::EventKind) {
            let (key, pressed) = match event {
                keytap::EventKind::KeyDown(key) => (key, true),
                keytap::EventKind::KeyUp(key) => (key, false),
                keytap::EventKind::KeyRepeat(_) => return,
            };
            match key {
                keytap::Key::ControlLeft => self.control_left = pressed,
                keytap::Key::ControlRight => self.control_right = pressed,
                keytap::Key::AltLeft => self.alt_left = pressed,
                keytap::Key::AltRight => self.alt_right = pressed,
                keytap::Key::MetaLeft => self.meta_left = pressed,
                keytap::Key::MetaRight => self.meta_right = pressed,
                _ => {}
            }
        }

        fn command_modifier(&self) -> bool {
            self.control_left
                || self.control_right
                || self.alt_left
                || self.alt_right
                || self.meta_left
                || self.meta_right
        }
    }

    impl ksni::Tray for ZsclipGtkStatusNotifier {
        const MENU_ON_ACTIVATE: bool = true;

        fn id(&self) -> String {
            "zsclip".to_string()
        }

        fn title(&self) -> String {
            "ZSClip".to_string()
        }

        fn icon_name(&self) -> String {
            "edit-paste".to_string()
        }

        fn tool_tip(&self) -> ksni::ToolTip {
            ksni::ToolTip {
                icon_name: self.icon_name(),
                title: self.title(),
                description: self.tooltip.clone(),
                ..Default::default()
            }
        }

        fn activate(&mut self, _x: i32, _y: i32) {
            let _ = self
                .sender
                .send(crate::app_core::NativeHostStatusMenuAction::ToggleWindow);
        }

        fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
            let mut items = Vec::new();
            for spec in native_host_status_menu_item_specs() {
                if spec.starts_section && !items.is_empty() {
                    items.push(ksni::MenuItem::Separator);
                }
                items.push({
                    let action = spec.action;
                    let sender = self.sender.clone();
                    ksni::menu::StandardItem {
                        label: spec.label.to_string(),
                        icon_name: spec.icon_name.to_string(),
                        activate: Box::new(move |_| {
                            let _ = sender.send(action);
                        }),
                        ..Default::default()
                    }
                    .into()
                });
            }
            items
        }
    }

    fn install_zsclip_gtk_css() {
        let Some(display) = gdk::Display::default() else {
            return;
        };
        let provider = gtk::CssProvider::new();
        provider.load_from_data(ZSCLIP_GTK_CSS);
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    fn apply_gtk_button_style_role(button: &Button, style_role: NativeButtonStyleRole) {
        match style_role {
            NativeButtonStyleRole::Plain => {}
            NativeButtonStyleRole::Suggested => button.add_css_class("suggested-action"),
            NativeButtonStyleRole::Destructive => button.add_css_class("destructive-action"),
        }
    }

    struct GtkWindowSystemCapabilityResult {
        supported: bool,
        result_name: &'static str,
    }

    struct GtkWindowSystemSnapshot {
        backend_name: &'static str,
        prefer_dark: bool,
        scale_factor: i32,
        always_on_top_supported: bool,
        cursor_follow_supported: bool,
    }

    trait GtkWindowSystemBackend {
        fn backend_name(&self) -> &'static str;
        fn prefer_dark(&self) -> bool;
        fn scale_factor(&self, window: &ApplicationWindow) -> i32;
        fn apply_always_on_top(
            &self,
            window: &ApplicationWindow,
            enabled: bool,
        ) -> GtkWindowSystemCapabilityResult;
        fn position_near_cursor(
            &self,
            window: &ApplicationWindow,
        ) -> GtkWindowSystemCapabilityResult;
    }

    struct Gtk4WindowSystemBackend;

    impl GtkWindowSystemBackend for Gtk4WindowSystemBackend {
        fn backend_name(&self) -> &'static str {
            "gtk4"
        }

        fn prefer_dark(&self) -> bool {
            gtk::Settings::default()
                .map(|settings| settings.is_gtk_application_prefer_dark_theme())
                .unwrap_or(false)
        }

        fn scale_factor(&self, window: &ApplicationWindow) -> i32 {
            window
                .surface()
                .and_then(|surface| {
                    gdk::Display::default().and_then(|display| {
                        display
                            .monitor_at_surface(&surface)
                            .map(|monitor| monitor.scale_factor())
                    })
                })
                .unwrap_or_else(|| window.scale_factor())
        }

        fn apply_always_on_top(
            &self,
            _window: &ApplicationWindow,
            _enabled: bool,
        ) -> GtkWindowSystemCapabilityResult {
            GtkWindowSystemCapabilityResult {
                supported: false,
                result_name: "zsclip.gtk.window.always_on_top.requires_backend_adapter",
            }
        }

        fn position_near_cursor(
            &self,
            _window: &ApplicationWindow,
        ) -> GtkWindowSystemCapabilityResult {
            GtkWindowSystemCapabilityResult {
                supported: false,
                result_name: "zsclip.gtk.window.cursor_follow.requires_backend_adapter",
            }
        }
    }

    struct X11CommandWindowSystemBackend {
        gtk4: Gtk4WindowSystemBackend,
    }

    impl X11CommandWindowSystemBackend {
        fn new() -> Self {
            Self {
                gtk4: Gtk4WindowSystemBackend,
            }
        }
    }

    impl GtkWindowSystemBackend for X11CommandWindowSystemBackend {
        fn backend_name(&self) -> &'static str {
            "gtk4_x11_command"
        }

        fn prefer_dark(&self) -> bool {
            self.gtk4.prefer_dark()
        }

        fn scale_factor(&self, window: &ApplicationWindow) -> i32 {
            self.gtk4.scale_factor(window)
        }

        fn apply_always_on_top(
            &self,
            _window: &ApplicationWindow,
            enabled: bool,
        ) -> GtkWindowSystemCapabilityResult {
            let state = if enabled { "add,above" } else { "remove,above" };
            if gtk_window_command_success("wmctrl", &["-r", ":ACTIVE:", "-b", state]) {
                GtkWindowSystemCapabilityResult {
                    supported: true,
                    result_name: "zsclip.gtk.window.always_on_top.x11_command",
                }
            } else {
                self.gtk4.apply_always_on_top(_window, enabled)
            }
        }

        fn position_near_cursor(
            &self,
            window: &ApplicationWindow,
        ) -> GtkWindowSystemCapabilityResult {
            let Some((cursor_x, cursor_y)) = gtk_xdotool_mouse_location() else {
                return self.gtk4.position_near_cursor(window);
            };
            let Some(window_id) = gtk_window_command_output("xdotool", &["getactivewindow"]) else {
                return self.gtk4.position_near_cursor(window);
            };
            let next_x = (cursor_x + 12).to_string();
            let next_y = (cursor_y + 12).to_string();
            if Command::new("xdotool")
                .args([
                    "windowmove",
                    window_id.as_str(),
                    next_x.as_str(),
                    next_y.as_str(),
                ])
                .output()
                .ok()
                .is_some_and(|output| output.status.success())
            {
                GtkWindowSystemCapabilityResult {
                    supported: true,
                    result_name: "zsclip.gtk.window.cursor_follow.x11_command",
                }
            } else {
                self.gtk4.position_near_cursor(window)
            }
        }
    }

    fn gtk_window_command_success(program: &str, args: &[&str]) -> bool {
        Command::new(program)
            .args(args)
            .output()
            .ok()
            .is_some_and(|output| output.status.success())
    }

    fn gtk_window_command_output(program: &str, args: &[&str]) -> Option<String> {
        let output = Command::new(program).args(args).output().ok()?;
        if !output.status.success() {
            return None;
        }
        let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
        (!value.is_empty()).then_some(value)
    }

    fn gtk_xdotool_mouse_location() -> Option<(i32, i32)> {
        let output = gtk_window_command_output("xdotool", &["getmouselocation", "--shell"])?;
        let mut x = None;
        let mut y = None;
        for line in output.lines() {
            if let Some(value) = line.strip_prefix("X=") {
                x = value.parse::<i32>().ok();
            } else if let Some(value) = line.strip_prefix("Y=") {
                y = value.parse::<i32>().ok();
            }
        }
        Some((x?, y?))
    }

    fn gtk_select_window_system_backend() -> X11CommandWindowSystemBackend {
        X11CommandWindowSystemBackend::new()
    }

    fn gtk_native_window_traits(
        window: &ApplicationWindow,
        backend: &dyn GtkWindowSystemBackend,
    ) -> GtkWindowSystemSnapshot {
        let always_on_top = backend.apply_always_on_top(window, true);
        let cursor_follow = backend.position_near_cursor(window);
        if !always_on_top.supported {
            eprintln!("{}", always_on_top.result_name);
        }
        if !cursor_follow.supported {
            eprintln!("{}", cursor_follow.result_name);
        }
        GtkWindowSystemSnapshot {
            backend_name: backend.backend_name(),
            prefer_dark: backend.prefer_dark(),
            scale_factor: backend.scale_factor(window),
            always_on_top_supported: always_on_top.supported,
            cursor_follow_supported: cursor_follow.supported,
        }
    }

    pub(super) fn run(summary: LinuxHostContractSummary) -> Result<(), String> {
        let app = Application::builder()
            .application_id("io.github.qiu7824.zsclip")
            .build();
        app.connect_activate(move |app| {
            install_zsclip_gtk_css();
            let window = ApplicationWindow::builder()
                .application(app)
                .default_width(960)
                .default_height(640)
                .title("ZSClip")
                .build();
            window.set_icon_name(Some("edit-paste"));

            let root = GtkBox::new(Orientation::Vertical, 12);
            root.set_margin_top(24);
            root.set_margin_bottom(24);
            root.set_margin_start(24);
            root.set_margin_end(24);

            let title = Label::new(Some("ZSClip"));
            title.add_css_class("title-1");
            title.set_xalign(0.0);

            let status = Label::new(Some(&format!(
                "GTK native host running app_core {}.{} with {} shared surfaces",
                summary.api_major, summary.api_minor, summary.surfaces
            )));
            status.set_xalign(0.0);
            status.add_css_class("dim-label");

            root.append(&title);
            root.append(&status);
            let search_spec = native_host_search_input_specs()[0];
            let search_entry = SearchEntry::new();
            search_entry.set_placeholder_text(Some(search_spec.placeholder));
            search_entry.set_widget_name(search_spec.id);
            search_entry.set_max_width_chars(60);
            let search_revealer = Revealer::new();
            search_revealer.set_transition_type(RevealerTransitionType::SlideDown);
            search_revealer.set_transition_duration(160);
            search_revealer.set_reveal_child(false);
            search_revealer.set_child(Some(&search_entry));
            let search_escape_controller = EventControllerKey::new();
            let search_entry_for_escape = search_entry.clone();
            let search_revealer_for_escape = search_revealer.clone();
            search_escape_controller.connect_key_pressed(move |_, key, _keycode, _state| {
                if key == gdk::Key::Escape {
                    search_entry_for_escape.set_text("");
                    search_revealer_for_escape.set_reveal_child(false);
                    return glib::Propagation::Stop;
                }
                glib::Propagation::Proceed
            });
            search_entry.add_controller(search_escape_controller);
            let status_menu = install_status_menu(app, &status, &window);
            if let Some(status_notifier) = install_status_notifier(app, &status, &window) {
                let _ = Box::leak(Box::new(status_notifier));
            }
            let header = HeaderBar::new();
            header.set_show_title_buttons(true);
            let header_title = Label::new(Some("ZSClip"));
            header_title.add_css_class("heading");
            header.set_title_widget(Some(&header_title));
            let search_button = ToggleButton::builder()
                .icon_name("edit-find-symbolic")
                .tooltip_text("Search")
                .build();
            let search_entry_for_toggle = search_entry.clone();
            let search_revealer_for_toggle = search_revealer.clone();
            search_button.connect_toggled(move |button| {
                let active = button.is_active();
                search_revealer_for_toggle.set_reveal_child(active);
                if active {
                    search_entry_for_toggle.grab_focus();
                } else {
                    search_entry_for_toggle.set_text("");
                }
            });
            header.pack_start(&search_button);
            let status_button = MenuButton::builder()
                .icon_name("open-menu-symbolic")
                .tooltip_text("Status")
                .build();
            let status_popover = PopoverMenu::from_model(Some(&status_menu));
            status_button.set_popover(Some(&status_popover));
            header.pack_end(&status_button);
            window.set_titlebar(Some(&header));
            root.append(&search_revealer);
            let clip_items = Rc::new(RefCell::new(
                crate::linux_app::linux_native_host_projected_clip_items(),
            ));
            let selected_item_id = Rc::new(Cell::new(
                clip_items
                    .borrow()
                    .first()
                    .map(|item| item.id)
                    .unwrap_or_default(),
            ));
            let current_group_filter = Rc::new(Cell::new(0_i64));
            let clip_rows: Vec<_> = native_host_clip_row_specs(
                &clip_items.borrow(),
                NATIVE_HOST_CLIP_ROW_CAPACITY,
            )
            .into_iter()
            .enumerate()
            .map(|(index, spec)| {
                let action = spec.action;
                let row = ListBoxRow::new();
                row.set_child(Some(&gtk_clip_row_content(
                    clip_items
                        .borrow()
                        .get(index)
                        .map(native_host_clip_row_presentation_for_projection)
                        .as_ref(),
                    &spec.label,
                )));
                row.set_widget_name(&action.item_id.to_string());
                row.set_visible(action.has_item());
                row.set_selectable(action.has_item());
                row.set_activatable(action.has_item());
                row.add_css_class("clip-row");
                row
            })
            .collect();
            register_dynamic_group_popup_actions(
                app,
                &status,
                selected_item_id.clone(),
                current_group_filter.clone(),
                clip_rows.clone(),
                clip_items.clone(),
            );
            refresh_clip_rows(&clip_rows, &clip_items.borrow());
            reconcile_selected_item_id(&selected_item_id, &clip_items.borrow());
            let clip_scroller = ScrolledWindow::builder()
                .hexpand(true)
                .vexpand(true)
                .hscrollbar_policy(PolicyType::Never)
                .vscrollbar_policy(PolicyType::Automatic)
                .build();
            let clip_list = ListBox::new();
            clip_list.set_selection_mode(SelectionMode::Single);
            clip_list.set_show_separators(true);
            clip_list.set_focusable(true);
            clip_list.add_css_class("boxed-list");
            clip_list.add_css_class("clip-list");
            for row in &clip_rows {
                clip_list.append(row);
            }
            let selected_item_id_for_selection = selected_item_id.clone();
            clip_list.connect_row_selected(move |_, row| {
                let Some(row) = row else {
                    return;
                };
                let item_id = row.widget_name().parse::<i64>().unwrap_or_default();
                if item_id > 0 {
                    selected_item_id_for_selection.set(item_id);
                    eprintln!("ZSClip GTK row selected item_id={}", item_id);
                }
            });
            let selected_item_id_for_activation = selected_item_id.clone();
            let status_for_activation = status.clone();
            let current_group_filter_for_activation = current_group_filter.clone();
            let clip_rows_for_activation = clip_rows.clone();
            let clip_items_for_activation = clip_items.clone();
            clip_list.connect_row_activated(move |_, row| {
                let item_id = row.widget_name().parse::<i64>().unwrap_or_default();
                if item_id <= 0 {
                    return;
                }
                selected_item_id_for_activation.set(item_id);
                let _ = perform_gtk_item_row_action(
                    NativeHostRowAction::Paste,
                    &selected_item_id_for_activation,
                    &status_for_activation,
                    &current_group_filter_for_activation,
                    &clip_rows_for_activation,
                    clip_items_for_activation.clone(),
                );
            });
            sync_clip_list_selection(&clip_list, &clip_rows, selected_item_id.get());
            clip_list.grab_focus();
            clip_scroller.set_child(Some(&clip_list));
            root.append(&clip_scroller);
            let row_actions = GtkBox::new(Orientation::Horizontal, 8);
            for spec in native_host_row_action_button_specs() {
                let action = spec.action;
                let button = Button::with_label(spec.label);
                button.set_widget_name(spec.id);
                let status = status.clone();
                let app = app.clone();
                let clip_items = clip_items.clone();
                let clip_rows = clip_rows.clone();
                let selected_item_id = selected_item_id.clone();
                let current_group_filter = current_group_filter.clone();
                button.connect_clicked(move |_| {
                    let _ = perform_gtk_item_row_action(
                        action,
                        &selected_item_id,
                        &status,
                        &current_group_filter,
                        &clip_rows,
                        clip_items.clone(),
                    );
                    if matches!(action, NativeHostRowAction::Edit) {
                        if let Some(plan) =
                            native_edit_plan(&clip_items.borrow(), selected_item_id.get())
                        {
                            present_edit_text_window(
                                &app,
                                plan,
                                false,
                                Some(EditRefreshTarget {
                                    rows: clip_rows.clone(),
                                    items: clip_items.clone(),
                                    current_group_filter: current_group_filter.clone(),
                                }),
                            );
                        }
                    }
                });
                row_actions.append(&button);
            }
            let row_menu = install_row_popup_menu(
                app,
                &status,
                selected_item_id.clone(),
                current_group_filter.clone(),
                clip_rows.clone(),
                clip_items.clone(),
            );
            for row in &clip_rows {
                install_row_context_menu(
                    row,
                    &clip_list,
                    &row_menu,
                    selected_item_id.clone(),
                    clip_items.clone(),
                );
            }
            let group_filter_menu = install_group_filter_popup_menu(
                app,
                &status,
                selected_item_id.clone(),
                current_group_filter.clone(),
                clip_rows.clone(),
                clip_items.clone(),
            );
            let group_popup_menus = GroupPopupMenus {
                row_menu: row_menu.clone(),
                group_filter_menu: group_filter_menu.clone(),
                current_group_filter: current_group_filter.clone(),
                selected_item_id: selected_item_id.clone(),
                clip_rows: clip_rows.clone(),
                clip_items: clip_items.clone(),
            };
            for spec in native_host_main_tool_button_specs() {
                let action = spec.action;
                match action {
                    NativeHostMainToolAction::RowMenu => {
                        let button = MenuButton::builder().label(spec.label).build();
                        button.set_widget_name(spec.id);
                        let popover = PopoverMenu::from_model(Some(&row_menu));
                        button.set_popover(Some(&popover));
                        row_actions.append(&button);
                    }
                    NativeHostMainToolAction::GroupFilter => {
                        let button = MenuButton::builder().label(spec.label).build();
                        button.set_widget_name(spec.id);
                        let popover = PopoverMenu::from_model(Some(&group_filter_menu));
                        button.set_popover(Some(&popover));
                        row_actions.append(&button);
                    }
                    #[cfg(feature = "vv-paste")]
                    NativeHostMainToolAction::VvPopup => {
                        let button = Button::with_label(spec.label);
                        button.set_widget_name(spec.id);
                        let app_for_vv = app.clone();
                        let current_group_filter_for_vv = current_group_filter.clone();
                        button.connect_clicked(move |_| {
                            eprintln!("ZSClip GTK VV popup requested");
                            present_vv_popup_window(&app_for_vv, current_group_filter_for_vv.get());
                        });
                        row_actions.append(&button);
                    }
                    #[cfg(feature = "vv-paste")]
                    NativeHostMainToolAction::VvTrigger => {
                        let button = Button::with_label(spec.label);
                        button.set_widget_name(spec.id);
                        let app_for_vv_trigger = app.clone();
                        let current_group_filter_for_vv_trigger = current_group_filter.clone();
                        button.connect_clicked(move |_| {
                            eprintln!("ZSClip GTK VV trigger requested");
                            perform_vv_trigger_demo(
                                &app_for_vv_trigger,
                                current_group_filter_for_vv_trigger.get(),
                            );
                        });
                        row_actions.append(&button);
                    }
                }
            }
            root.append(&row_actions);
            let search_rows = clip_rows.clone();
            let search_items = clip_items.clone();
            let search_selected_item_id = selected_item_id.clone();
            let search_clip_list = clip_list.clone();
            search_entry.connect_search_changed(move |entry| {
                let action = NativeHostSearchTextAction::new(entry.text().to_string());
                let _ = crate::linux_app::dispatch_linux_native_search_text_action(action);
                update_clip_list_visibility(
                    &search_rows,
                    &search_items.borrow(),
                    &search_selected_item_id,
                    entry.text().as_str(),
                );
                sync_clip_list_selection(
                    &search_clip_list,
                    &search_rows,
                    search_selected_item_id.get(),
                );
            });
            let actions = GtkBox::new(Orientation::Horizontal, 8);
            for spec in native_host_main_action_button_specs() {
                let action = spec.action;
                let button = Button::with_label(spec.label);
                button.set_widget_name(spec.id);
                let app = app.clone();
                let window = window.clone();
                let search_entry = search_entry.clone();
                let search_revealer = search_revealer.clone();
                let search_button = search_button.clone();
                let search_rows_for_toggle = clip_rows.clone();
                let search_items_for_toggle = clip_items.clone();
                let search_selected_item_id_for_toggle = selected_item_id.clone();
                let search_clip_list_for_toggle = clip_list.clone();
                let status = status.clone();
                let group_popup_menus = group_popup_menus.clone();
                button.connect_clicked(move |_| {
                    let result = crate::linux_app::dispatch_linux_native_host_action(action);
                    status.set_text(&format!(
                        "{} -> {}",
                        action.action_name(),
                        result.result_name
                    ));
                    if action.opens_settings_surface() {
                        present_settings_window(
                            &app,
                            &result.result_name,
                            Some(group_popup_menus.clone()),
                        );
                    }
                    if action.toggles_search_surface() {
                        let next_visible = !search_revealer.reveals_child();
                        search_revealer.set_reveal_child(next_visible);
                        search_button.set_active(next_visible);
                        if next_visible {
                            search_entry.grab_focus();
                        } else {
                            search_entry.set_text("");
                            update_clip_list_visibility(
                                &search_rows_for_toggle,
                                &search_items_for_toggle.borrow(),
                                &search_selected_item_id_for_toggle,
                                "",
                            );
                            sync_clip_list_selection(
                                &search_clip_list_for_toggle,
                                &search_rows_for_toggle,
                                search_selected_item_id_for_toggle.get(),
                            );
                            search_clip_list_for_toggle.grab_focus();
                        }
                    }
                    if action.hides_main_window_surface() {
                        window.set_visible(false);
                    }
                    if action.should_close_host() {
                        app.quit();
                    }
                });
                actions.append(&button);
            }
            root.append(&actions);
            install_main_window_keyboard_controller(
                &window,
                &search_entry,
                &search_revealer,
                &search_button,
                &clip_list,
                selected_item_id.clone(),
                &status,
                current_group_filter.clone(),
                clip_rows.clone(),
                clip_items.clone(),
            );
            install_vv_key_controller(&window, app, current_group_filter.clone());
            install_vv_global_key_tap(app);
            window.set_child(Some(&root));
            window.present();
            let window_backend = gtk_select_window_system_backend();
            let window_traits = gtk_native_window_traits(&window, &window_backend);
            eprintln!(
                "ZSClip GTK native window traits backend={} always_on_top_supported={} cursor_follow_supported={} scale_factor={} dark_mode={}",
                window_traits.backend_name,
                window_traits.always_on_top_supported,
                window_traits.cursor_follow_supported,
                window_traits.scale_factor,
                window_traits.prefer_dark
            );
            run_auto_smoke_if_requested(app, &status);
        });
        let _exit_code = app.run();
        Ok(())
    }

    fn run_auto_smoke_if_requested(app: &Application, status: &Label) {
        if !matches!(
            std::env::var("ZSCLIP_NATIVE_HOST_AUTO_SMOKE").as_deref(),
            Ok("1")
        ) {
            return;
        }

        eprintln!("ZSClip GTK auto smoke started");

        let clipboard_text = "zsclip gtk auto smoke clipboard";
        let clipboard_written =
            <crate::linux_app::LinuxClipboardHost as crate::app_core::ClipboardHost>::write_text(
                clipboard_text,
            );
        let clipboard_read =
            <crate::linux_app::LinuxClipboardHost as crate::app_core::ClipboardHost>::read_text()
                .unwrap_or_default();
        eprintln!(
            "ZSClip GTK clipboard text smoke write={} read={}",
            clipboard_written,
            clipboard_read == clipboard_text
        );
        let file_sequence_before =
            <crate::linux_app::LinuxClipboardHost as crate::app_core::ClipboardHost>::sequence_number();
        let smoke_file = std::env::temp_dir().join("zsclip-gtk-auto-smoke-file.txt");
        let _ = std::fs::write(&smoke_file, "zsclip gtk auto smoke file");
        let smoke_path = smoke_file.to_string_lossy().to_string();
        let file_written =
            <crate::linux_app::LinuxClipboardHost as crate::app_core::ClipboardHost>::write_file_paths(
                &[smoke_path.clone()],
            );
        let file_read =
            <crate::linux_app::LinuxClipboardHost as crate::app_core::ClipboardHost>::read_file_paths()
                .unwrap_or_default();
        let file_sequence_after =
            <crate::linux_app::LinuxClipboardHost as crate::app_core::ClipboardHost>::sequence_number();
        eprintln!(
            "ZSClip GTK clipboard file smoke write={} read={}",
            file_written,
            file_read.iter().any(|path| path == &smoke_path)
        );
        eprintln!(
            "ZSClip GTK clipboard sequence smoke before={} after={} changed={}",
            file_sequence_before,
            file_sequence_after,
            file_sequence_after != file_sequence_before
        );
        let mut monitor_model = crate::linux_app::LinuxApplicationModel::default();
        let _ = monitor_model.poll_clipboard_capture_event();
        let monitor_sequence_before =
            <crate::linux_app::LinuxClipboardHost as crate::app_core::ClipboardHost>::sequence_number(
            );
        let _ =
            <crate::linux_app::LinuxClipboardHost as crate::app_core::ClipboardHost>::write_text(
                "zsclip gtk monitor smoke clipboard",
            );
        let monitor_event = monitor_model.poll_clipboard_capture_event();
        let monitor_changed = matches!(
            monitor_event,
            Some(crate::app_core::ApplicationEvent::ClipboardChanged { sequence })
                if sequence != monitor_sequence_before
        );
        eprintln!(
            "ZSClip GTK clipboard monitor smoke changed={}",
            monitor_changed
        );
        let shell_open_host = crate::linux_app::LinuxShellOpenHost::default();
        <crate::linux_app::LinuxShellOpenHost as crate::app_core::NativeShellOpenHost>::open_path(
            &shell_open_host,
            &smoke_path,
        );
        let shell_open_recorded = shell_open_host
            .opened_paths()
            .iter()
            .any(|path| path == &smoke_path);
        eprintln!(
            "ZSClip GTK shell open smoke dry_run={} recorded={}",
            matches!(
                std::env::var("ZSCLIP_NATIVE_HOST_SHELL_OPEN_DRY_RUN").as_deref(),
                Ok("1")
            ),
            shell_open_recorded
        );
        let previous_file_picker_smoke =
            std::env::var_os("ZSCLIP_NATIVE_HOST_FILE_PICKER_SMOKE_PATH");
        std::env::set_var("ZSCLIP_NATIVE_HOST_FILE_PICKER_SMOKE_PATH", &smoke_path);
        let file_dialog_host = crate::linux_app::LinuxFileDialogHost::default();
        let file_picker_result =
            <crate::linux_app::LinuxFileDialogHost as crate::app_core::NativeFileDialogHost>::pick_file(
                &file_dialog_host,
                crate::app_core::NativeFileDialogRequest {
                    title: "ZSClip Smoke File",
                    filter_name: "Text",
                    filter_pattern: "*.txt",
                    current_path: &smoke_path,
                },
            );
        match previous_file_picker_smoke {
            Some(value) => std::env::set_var("ZSCLIP_NATIVE_HOST_FILE_PICKER_SMOKE_PATH", value),
            None => std::env::remove_var("ZSCLIP_NATIVE_HOST_FILE_PICKER_SMOKE_PATH"),
        }
        let file_picker_recorded = !file_dialog_host.requests().is_empty();
        let file_picker_selected = matches!(file_picker_result.as_ref().map(|path| path.as_deref()), Ok(Some(path)) if path == smoke_path);
        eprintln!(
            "ZSClip GTK file picker smoke injected=true recorded={} selected={}",
            file_picker_recorded, file_picker_selected
        );
        let identity = crate::linux_app::linux_native_identity_smoke();
        eprintln!(
            "ZSClip GTK identity smoke queried=true foreground_seen={} process_name_seen={} class_name_seen={} exists={} foreground={} current_process_window={} foreground_requested={} focus_status={:?}",
            identity.foreground_seen,
            identity.process_name_seen,
            identity.class_name_seen,
            identity.foreground_exists,
            identity.foreground_matches,
            identity.current_process_window,
            identity.foreground_requested,
            identity.focus_status
        );

        let settings_result = crate::linux_app::dispatch_linux_native_host_action(
            crate::app_core::NativeHostUiAction::OpenSettings,
        );
        eprintln!(
            "ZSClip GTK action open_settings -> {}",
            settings_result.result_name
        );
        present_settings_window(app, &settings_result.result_name, None);

        let mut settings_control_actions =
            vec![crate::app_core::NativeHostSettingsControlAction::ToggleClipboardCapture];
        #[cfg(feature = "lan-sync")]
        settings_control_actions
            .push(crate::app_core::NativeHostSettingsControlAction::ToggleLanSync);
        #[cfg(any(feature = "cloud-sync", feature = "lan-sync"))]
        settings_control_actions
            .push(crate::app_core::NativeHostSettingsControlAction::OpenSyncModeDropdown);
        for action in settings_control_actions {
            let result = crate::linux_app::dispatch_linux_native_settings_control_action(action);
            eprintln!(
                "ZSClip GTK settings control action {} -> {}",
                action.action_name(),
                result.result_name
            );
        }

        let mut row_actions = vec![
            crate::app_core::NativeHostRowAction::Copy,
            crate::app_core::NativeHostRowAction::Edit,
        ];
        #[cfg(feature = "ai-actions")]
        row_actions.push(crate::app_core::NativeHostRowAction::TextTranslate);

        for action in row_actions {
            let result = crate::linux_app::dispatch_linux_native_row_action(action);
            eprintln!(
                "ZSClip GTK row action {} -> {}",
                action.action_name(),
                result.result_name
            );
            if matches!(action, crate::app_core::NativeHostRowAction::Edit) {
                let items = crate::linux_app::linux_native_host_projected_clip_items();
                let selected_item_id = items.first().map(|item| item.id).unwrap_or_default();
                if let Some(plan) = native_edit_plan(&items, selected_item_id) {
                    present_edit_text_window(app, plan, true, None);
                }
            }
        }

        eprintln!("ZSClip GTK VV trigger requested");
        perform_vv_trigger_demo(app, 0);
        let vv_result = crate::linux_app::dispatch_linux_native_vv_select_event(0);
        let vv_paste = perform_gtk_vv_paste(0, 0);
        eprintln!("ZSClip GTK VV select 0 -> {}", vv_result.event_name);
        eprintln!(
            "ZSClip GTK VV paste 0 -> {} accepted={} kind={}",
            vv_paste.result_name,
            vv_paste.accepted,
            vv_paste.clipboard_kind.unwrap_or("none")
        );

        #[cfg(feature = "lan-sync")]
        {
            let status_action = crate::app_core::NativeHostStatusMenuAction::ToggleLanSync;
            let status_result =
                crate::linux_app::dispatch_linux_native_status_menu_action(status_action);
            eprintln!(
                "ZSClip GTK status menu action {} -> {}",
                status_action.action_name(),
                status_result.result_name
            );
        }

        status.set_text("GTK auto smoke complete");
        eprintln!("ZSClip GTK auto smoke finished");
    }

    fn toggle_gtk_main_window(window: &ApplicationWindow) {
        if window.is_visible() {
            window.set_visible(false);
        } else {
            window.present();
        }
    }

    fn install_status_menu(
        app: &Application,
        status: &Label,
        window: &ApplicationWindow,
    ) -> gio::Menu {
        let menu = gio::Menu::new();
        let mut section = gio::Menu::new();
        for spec in native_host_status_menu_item_specs() {
            let action = spec.action;
            let detailed_action = format!("app.{}", action.action_name());
            if spec.starts_section && section.n_items() > 0 {
                menu.append_section(None, &section);
                section = gio::Menu::new();
            }
            let item = gio::MenuItem::new(Some(spec.label), Some(&detailed_action));
            if !spec.icon_name.is_empty() {
                let icon = gio::ThemedIcon::new(spec.icon_name);
                item.set_icon(&icon);
            }
            section.append_item(&item);
            let simple_action = gio::SimpleAction::new(action.action_name(), None);
            let action_app = app.clone();
            let status = status.clone();
            let window = window.clone();
            simple_action.connect_activate(move |_, _| {
                let result = crate::linux_app::dispatch_linux_native_status_menu_action(action);
                eprintln!(
                    "ZSClip GTK status menu action {} -> {}",
                    action.action_name(),
                    result.result_name
                );
                status.set_text(&format!(
                    "{} -> {}",
                    action.action_name(),
                    result.result_name
                ));
                if action.toggles_main_window_surface() {
                    toggle_gtk_main_window(&window);
                }
                if action.should_exit_host() {
                    action_app.quit();
                }
            });
            app.add_action(&simple_action);
        }
        if section.n_items() > 0 {
            menu.append_section(None, &section);
        }

        let menubar = gio::Menu::new();
        menubar.append_submenu(Some("ZSClip"), &menu);
        app.set_menubar(Some(&menubar));
        menu
    }

    fn install_status_notifier(
        app: &Application,
        status: &Label,
        window: &ApplicationWindow,
    ) -> Option<ksni::blocking::Handle<ZsclipGtkStatusNotifier>> {
        let (sender, receiver) = mpsc::channel();
        let app_for_tray = app.clone();
        let status_for_tray = status.clone();
        let window_for_tray = window.clone();
        glib::timeout_add_local(Duration::from_millis(100), move || {
            for action in receiver.try_iter() {
                let result = crate::linux_app::dispatch_linux_native_status_menu_action(action);
                eprintln!(
                    "ZSClip GTK StatusNotifier action {} -> {}",
                    action.action_name(),
                    result.result_name
                );
                status_for_tray.set_text(&format!(
                    "{} -> {}",
                    action.action_name(),
                    result.result_name
                ));
                if action.toggles_main_window_surface() {
                    toggle_gtk_main_window(&window_for_tray);
                }
                if action.should_exit_host() {
                    app_for_tray.quit();
                    return glib::ControlFlow::Break;
                }
            }
            glib::ControlFlow::Continue
        });

        match (ZsclipGtkStatusNotifier {
            sender,
            tooltip: "ZSClip clipboard manager".to_string(),
        })
        .assume_sni_available(true)
        .spawn()
        {
            Ok(handle) => {
                eprintln!("ZSClip GTK StatusNotifierItem installed");
                Some(handle)
            }
            Err(error) => {
                eprintln!("ZSClip GTK StatusNotifierItem unavailable: {}", error);
                None
            }
        }
    }

    fn install_row_context_menu(
        row: &ListBoxRow,
        clip_list: &ListBox,
        row_menu: &gio::Menu,
        selected_item_id: Rc<Cell<i64>>,
        clip_items: Rc<RefCell<Vec<NativeHostClipListItemProjection>>>,
    ) {
        let popover = PopoverMenu::from_model(Some(row_menu));
        popover.set_parent(row);
        let gesture = GestureClick::new();
        gesture.set_button(3);
        let row = row.clone();
        let gesture_row = row.clone();
        let clip_list = clip_list.clone();
        let row_menu = row_menu.clone();
        gesture.connect_pressed(move |_, _, x, y| {
            let item_id = gesture_row.widget_name().parse::<i64>().unwrap_or_default();
            if item_id <= 0 {
                return;
            }
            selected_item_id.set(item_id);
            clip_list.select_row(Some(&gesture_row));
            let groups = crate::db_runtime::native_clip_groups(0).unwrap_or_default();
            replace_popup_menu_entries(
                &row_menu,
                &native_host_full_row_popup_menu_entries_for_groups(
                    &groups,
                    native_host_row_popup_menu_input_for_projection(
                        &clip_items.borrow(),
                        item_id,
                        true,
                    ),
                    |label| label.to_string(),
                ),
            );
            popover.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
            popover.popup();
            eprintln!("ZSClip GTK row context menu item_id={}", item_id);
        });
        row.add_controller(gesture);
    }

    fn install_main_window_keyboard_controller(
        window: &ApplicationWindow,
        search_entry: &SearchEntry,
        search_revealer: &Revealer,
        search_button: &ToggleButton,
        clip_list: &ListBox,
        selected_item_id: Rc<Cell<i64>>,
        status: &Label,
        current_group_filter: Rc<Cell<i64>>,
        clip_rows: Vec<ListBoxRow>,
        clip_items: Rc<RefCell<Vec<NativeHostClipListItemProjection>>>,
    ) {
        let controller = EventControllerKey::new();
        let search_entry = search_entry.clone();
        let search_revealer = search_revealer.clone();
        let search_button = search_button.clone();
        let clip_list = clip_list.clone();
        let status = status.clone();
        controller.connect_key_pressed(move |_, key, _keycode, state| {
            if state.contains(gdk::ModifierType::CONTROL_MASK)
                && matches!(key.to_unicode(), Some('f' | 'F'))
            {
                search_revealer.set_reveal_child(true);
                search_button.set_active(true);
                search_entry.grab_focus();
                return glib::Propagation::Stop;
            }

            if search_entry.has_focus() {
                return glib::Propagation::Proceed;
            }

            if key == gdk::Key::Return || key == gdk::Key::KP_Enter {
                if let Some(row) = clip_list.selected_row() {
                    let item_id = row.widget_name().parse::<i64>().unwrap_or_default();
                    if item_id > 0 {
                        selected_item_id.set(item_id);
                    }
                } else {
                    sync_clip_list_selection(&clip_list, &clip_rows, selected_item_id.get());
                }
                let _ = perform_gtk_item_row_action(
                    NativeHostRowAction::Paste,
                    &selected_item_id,
                    &status,
                    &current_group_filter,
                    &clip_rows,
                    clip_items.clone(),
                );
                return glib::Propagation::Stop;
            }

            if key == gdk::Key::Delete || key == gdk::Key::BackSpace {
                if let Some(row) = clip_list.selected_row() {
                    let item_id = row.widget_name().parse::<i64>().unwrap_or_default();
                    if item_id > 0 {
                        selected_item_id.set(item_id);
                    }
                } else {
                    sync_clip_list_selection(&clip_list, &clip_rows, selected_item_id.get());
                }
                let _ = perform_gtk_item_row_action(
                    NativeHostRowAction::Delete,
                    &selected_item_id,
                    &status,
                    &current_group_filter,
                    &clip_rows,
                    clip_items.clone(),
                );
                return glib::Propagation::Stop;
            }

            glib::Propagation::Proceed
        });
        window.add_controller(controller);
    }

    fn sync_clip_list_selection(clip_list: &ListBox, rows: &[ListBoxRow], selected_item_id: i64) {
        let selected_row = rows
            .iter()
            .find(|row| {
                row.is_visible()
                    && row.widget_name().parse::<i64>().unwrap_or_default() == selected_item_id
            })
            .or_else(|| {
                rows.iter().find(|row| {
                    row.is_visible() && row.widget_name().parse::<i64>().unwrap_or_default() > 0
                })
            });
        if let Some(row) = selected_row {
            clip_list.select_row(Some(row));
        } else {
            clip_list.unselect_all();
        }
    }

    fn install_row_popup_menu(
        app: &Application,
        status: &Label,
        selected_item_id: Rc<Cell<i64>>,
        current_group_filter: Rc<Cell<i64>>,
        clip_rows: Vec<ListBoxRow>,
        clip_items: Rc<RefCell<Vec<NativeHostClipListItemProjection>>>,
    ) -> gio::Menu {
        let groups = crate::db_runtime::native_clip_groups(0).unwrap_or_default();
        let entries = native_host_full_row_popup_menu_entries_for_groups(
            &groups,
            native_host_row_popup_menu_input_for_projection(
                &clip_items.borrow(),
                selected_item_id.get(),
                true,
            ),
            |label| label.to_string(),
        );
        install_popup_menu(
            app,
            status,
            entries,
            selected_item_id,
            current_group_filter,
            clip_rows,
            clip_items,
        )
    }

    fn install_group_filter_popup_menu(
        app: &Application,
        status: &Label,
        selected_item_id: Rc<Cell<i64>>,
        current_group_filter: Rc<Cell<i64>>,
        clip_rows: Vec<ListBoxRow>,
        clip_items: Rc<RefCell<Vec<NativeHostClipListItemProjection>>>,
    ) -> gio::Menu {
        let groups = crate::db_runtime::native_clip_groups(0).unwrap_or_default();
        install_popup_menu(
            app,
            status,
            native_host_group_filter_popup_menu_entries_for_groups(
                &groups,
                current_group_filter.get(),
            ),
            selected_item_id,
            current_group_filter,
            clip_rows,
            clip_items,
        )
    }

    fn install_popup_menu(
        app: &Application,
        status: &Label,
        entries: Vec<NativePopupMenuEntry>,
        selected_item_id: Rc<Cell<i64>>,
        current_group_filter: Rc<Cell<i64>>,
        clip_rows: Vec<ListBoxRow>,
        clip_items: Rc<RefCell<Vec<NativeHostClipListItemProjection>>>,
    ) -> gio::Menu {
        for menu_id in popup_command_ids(&entries) {
            register_popup_command_action(
                app,
                status,
                menu_id,
                selected_item_id.clone(),
                current_group_filter.clone(),
                clip_rows.clone(),
                clip_items.clone(),
            );
        }

        build_row_popup_menu(&entries)
    }

    fn register_dynamic_group_popup_actions(
        app: &Application,
        status: &Label,
        selected_item_id: Rc<Cell<i64>>,
        current_group_filter: Rc<Cell<i64>>,
        clip_rows: Vec<ListBoxRow>,
        clip_items: Rc<RefCell<Vec<NativeHostClipListItemProjection>>>,
    ) {
        register_popup_command_action(
            app,
            status,
            menu_ids::GROUP_FILTER_ALL,
            selected_item_id.clone(),
            current_group_filter.clone(),
            clip_rows.clone(),
            clip_items.clone(),
        );
        for index in 0..menu_ids::DYNAMIC_GROUP_LIMIT {
            register_popup_command_action(
                app,
                status,
                menu_ids::ROW_GROUP_BASE + index,
                selected_item_id.clone(),
                current_group_filter.clone(),
                clip_rows.clone(),
                clip_items.clone(),
            );
            register_popup_command_action(
                app,
                status,
                menu_ids::GROUP_FILTER_BASE + index,
                selected_item_id.clone(),
                current_group_filter.clone(),
                clip_rows.clone(),
                clip_items.clone(),
            );
        }
    }

    fn register_popup_command_action(
        app: &Application,
        status: &Label,
        menu_id: usize,
        selected_item_id: Rc<Cell<i64>>,
        current_group_filter: Rc<Cell<i64>>,
        clip_rows: Vec<ListBoxRow>,
        clip_items: Rc<RefCell<Vec<NativeHostClipListItemProjection>>>,
    ) {
        let action_name = popup_command_action_name(menu_id);
        if app.lookup_action(&action_name).is_some() {
            return;
        }
        let simple_action = gio::SimpleAction::new(&action_name, None);
        let status = status.clone();
        let action_app = app.clone();
        simple_action.connect_activate(move |_, _| {
            let result = crate::linux_app::dispatch_linux_native_menu_command_id(menu_id);
            eprintln!(
                "ZSClip GTK popup menu command {} -> {}",
                menu_id, result.result_name
            );
            status.set_text(&format!(
                "popup_command_{} -> {}",
                menu_id, result.result_name
            ));
            if perform_group_menu_command(
                menu_id,
                selected_item_id.clone(),
                current_group_filter.clone(),
                &clip_rows,
                clip_items.clone(),
            ) {
                return;
            }
            if let Some(action) = NativeHostRowAction::from_menu_id(menu_id) {
                let _ = perform_gtk_item_row_action(
                    action,
                    &selected_item_id,
                    &status,
                    &current_group_filter,
                    &clip_rows,
                    clip_items.clone(),
                );
                if !matches!(action, NativeHostRowAction::Edit) {
                    return;
                }
                if let Some(plan) = native_edit_plan(&clip_items.borrow(), selected_item_id.get()) {
                    present_edit_text_window(
                        &action_app,
                        plan,
                        false,
                        Some(EditRefreshTarget {
                            rows: clip_rows.clone(),
                            items: clip_items.clone(),
                            current_group_filter: current_group_filter.clone(),
                        }),
                    );
                }
            }
        });
        app.add_action(&simple_action);
    }

    fn perform_gtk_item_row_action(
        action: NativeHostRowAction,
        selected_item_id: &Cell<i64>,
        status: &Label,
        current_group_filter: &Cell<i64>,
        clip_rows: &[ListBoxRow],
        clip_items: Rc<RefCell<Vec<NativeHostClipListItemProjection>>>,
    ) -> ProductAdapterCommandResult {
        let item_id = selected_item_id.get();
        let result = crate::linux_app::dispatch_linux_native_row_action_for_item(action, item_id);
        eprintln!(
            "ZSClip GTK row action {} item_id={} -> {}",
            action.action_name(),
            item_id,
            result.result_name
        );
        status.set_text(&format!(
            "{} item_id={} -> {}",
            action.action_name(),
            item_id,
            result.result_name
        ));
        if result.accepted && matches!(action, NativeHostRowAction::Paste) {
            let posted = gtk_post_native_paste_shortcut();
            eprintln!("ZSClip GTK row paste shortcut posted={}", posted);
        }
        if result.accepted
            && matches!(
                action,
                NativeHostRowAction::Pin | NativeHostRowAction::Delete
            )
        {
            reload_clip_items_for_group_with_selection(
                current_group_filter,
                clip_rows,
                clip_items,
                selected_item_id,
            );
        }
        result
    }

    fn perform_group_menu_command(
        menu_id: usize,
        selected_item_id: Rc<Cell<i64>>,
        current_group_filter: Rc<Cell<i64>>,
        clip_rows: &[ListBoxRow],
        clip_items: Rc<RefCell<Vec<NativeHostClipListItemProjection>>>,
    ) -> bool {
        let groups = crate::db_runtime::native_clip_groups(0).unwrap_or_default();
        if menu_id == menu_ids::ROW_GROUP_REMOVE {
            let item_id = selected_item_id.get();
            let result = crate::linux_app::dispatch_linux_native_remove_group(item_id);
            eprintln!(
                "ZSClip GTK remove group item_id={} -> {}",
                item_id, result.result_name
            );
            reload_clip_items_for_group_with_selection(
                &current_group_filter,
                clip_rows,
                clip_items,
                &selected_item_id,
            );
            return true;
        }
        if let Some(MainRowGroupSelection::Group { index }) =
            main_row_group_selection_for_id(menu_id)
        {
            let Some(group) = groups.get(index) else {
                return true;
            };
            let item_id = selected_item_id.get();
            let result = crate::linux_app::dispatch_linux_native_assign_group(item_id, group.id);
            eprintln!(
                "ZSClip GTK assign group item_id={} group_id={} -> {}",
                item_id, group.id, result.result_name
            );
            reload_clip_items_for_group_with_selection(
                &current_group_filter,
                clip_rows,
                clip_items,
                &selected_item_id,
            );
            return true;
        }

        match main_group_filter_selection_for_id(menu_id) {
            Some(MainGroupFilterSelection::All) => {
                current_group_filter.set(0);
                let result = crate::linux_app::dispatch_linux_native_group_filter(0);
                eprintln!("ZSClip GTK group filter all -> {}", result.result_name);
                reload_clip_items_for_group_with_selection(
                    &current_group_filter,
                    clip_rows,
                    clip_items,
                    &selected_item_id,
                );
                true
            }
            Some(MainGroupFilterSelection::Group { index }) => {
                let Some(group) = groups.get(index) else {
                    return true;
                };
                current_group_filter.set(group.id);
                let result = crate::linux_app::dispatch_linux_native_group_filter(group.id);
                eprintln!(
                    "ZSClip GTK group filter group_id={} -> {}",
                    group.id, result.result_name
                );
                reload_clip_items_for_group_with_selection(
                    &current_group_filter,
                    clip_rows,
                    clip_items,
                    &selected_item_id,
                );
                true
            }
            None => false,
        }
    }

    fn reload_clip_items_for_group(
        current_group_filter: &Cell<i64>,
        rows: &[ListBoxRow],
        clip_items: Rc<RefCell<Vec<NativeHostClipListItemProjection>>>,
    ) {
        *clip_items.borrow_mut() =
            crate::linux_app::linux_native_host_projected_clip_items_for_group(
                current_group_filter.get(),
            );
        refresh_clip_rows(rows, &clip_items.borrow());
    }

    fn reload_clip_items_for_group_with_selection(
        current_group_filter: &Cell<i64>,
        rows: &[ListBoxRow],
        clip_items: Rc<RefCell<Vec<NativeHostClipListItemProjection>>>,
        selected_item_id: &Cell<i64>,
    ) {
        reload_clip_items_for_group(current_group_filter, rows, clip_items.clone());
        reconcile_selected_item_id(selected_item_id, &clip_items.borrow());
    }

    fn reconcile_selected_item_id(
        selected_item_id: &Cell<i64>,
        items: &[NativeHostClipListItemProjection],
    ) {
        selected_item_id.set(native_host_reconciled_selected_item_id(
            selected_item_id.get(),
            items,
        ));
    }

    fn popup_command_ids(entries: &[NativePopupMenuEntry]) -> Vec<usize> {
        let mut ids = Vec::new();
        collect_popup_command_ids(entries, &mut ids);
        ids
    }

    fn collect_popup_command_ids(entries: &[NativePopupMenuEntry], ids: &mut Vec<usize>) {
        for entry in entries {
            match entry {
                NativePopupMenuEntry::Command {
                    id, enabled: true, ..
                } => {
                    if !ids.contains(id) {
                        ids.push(*id);
                    }
                }
                NativePopupMenuEntry::Submenu {
                    enabled: true,
                    entries,
                    ..
                } => collect_popup_command_ids(entries, ids),
                _ => {}
            }
        }
    }

    fn popup_command_action_name(menu_id: usize) -> String {
        format!("popup_command_{}", menu_id)
    }

    #[derive(Clone)]
    struct GroupPopupMenus {
        row_menu: gio::Menu,
        group_filter_menu: gio::Menu,
        current_group_filter: Rc<Cell<i64>>,
        selected_item_id: Rc<Cell<i64>>,
        clip_rows: Vec<ListBoxRow>,
        clip_items: Rc<RefCell<Vec<NativeHostClipListItemProjection>>>,
    }

    fn refresh_group_popup_menus(menus: &GroupPopupMenus) {
        let groups = crate::db_runtime::native_clip_groups(0).unwrap_or_default();
        if menus.current_group_filter.get() > 0
            && !groups
                .iter()
                .any(|group| group.id == menus.current_group_filter.get())
        {
            menus.current_group_filter.set(0);
        }
        replace_popup_menu_entries(
            &menus.row_menu,
            &native_host_full_row_popup_menu_entries_for_groups(
                &groups,
                native_host_row_popup_menu_input_for_projection(
                    &menus.clip_items.borrow(),
                    menus.selected_item_id.get(),
                    true,
                ),
                |label| label.to_string(),
            ),
        );
        replace_popup_menu_entries(
            &menus.group_filter_menu,
            &native_host_group_filter_popup_menu_entries_for_groups(
                &groups,
                menus.current_group_filter.get(),
            ),
        );
        reload_clip_items_for_group_with_selection(
            &menus.current_group_filter,
            &menus.clip_rows,
            menus.clip_items.clone(),
            &menus.selected_item_id,
        );
    }

    fn refresh_group_popup_menus_for_category(category: i64, menus: Option<&GroupPopupMenus>) {
        if category == 0 {
            if let Some(menus) = menus {
                refresh_group_popup_menus(menus);
            }
        }
    }

    fn build_row_popup_menu(entries: &[NativePopupMenuEntry]) -> gio::Menu {
        let menu = gio::Menu::new();
        append_popup_menu_entries(&menu, entries);
        menu
    }

    fn replace_popup_menu_entries(menu: &gio::Menu, entries: &[NativePopupMenuEntry]) {
        menu.remove_all();
        append_popup_menu_entries(menu, entries);
    }

    fn append_popup_menu_entries(menu: &gio::Menu, entries: &[NativePopupMenuEntry]) {
        for entry in entries {
            match entry {
                NativePopupMenuEntry::Command {
                    id, label, enabled, ..
                } => {
                    let detailed_action = format!("app.{}", popup_command_action_name(*id));
                    let display_label = native_popup_menu_command_accelerator_label(*id)
                        .map(|accelerator| format!("{label}\t{accelerator}"))
                        .unwrap_or_else(|| label.to_string());
                    let item = gio::MenuItem::new(
                        Some(&display_label),
                        enabled.then_some(detailed_action.as_str()),
                    );
                    if let Some(icon_name) = native_popup_menu_command_icon_name(*id) {
                        let icon = gio::ThemedIcon::new(icon_name);
                        item.set_icon(&icon);
                    }
                    menu.append_item(&item);
                }
                NativePopupMenuEntry::Submenu {
                    label,
                    enabled,
                    entries,
                } => {
                    if *enabled {
                        let submenu = build_row_popup_menu(entries);
                        menu.append_submenu(Some(label), &submenu);
                    }
                }
                NativePopupMenuEntry::Separator => {
                    let section = gio::Menu::new();
                    menu.append_section(None, &section);
                }
            }
        }
    }

    fn present_vv_popup_window(app: &Application, current_group_id: i64) {
        let groups = crate::db_runtime::native_clip_groups(0).unwrap_or_default();
        let group_label = native_host_group_filter_label_for_groups(&groups, current_group_id);
        let items =
            crate::linux_app::linux_native_host_projected_clip_items_for_group(current_group_id);
        let plan = native_host_vv_popup_render_plan_for_projection(&items, &group_label);
        let width = plan
            .text_commands
            .iter()
            .map(|command| command.rect.right)
            .max()
            .unwrap_or(360)
            .max(360);
        let height = plan
            .text_commands
            .iter()
            .map(|command| command.rect.bottom)
            .max()
            .unwrap_or(168)
            .max(168)
            + 12;
        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(width)
            .default_height(height)
            .decorated(false)
            .title("ZSClip VV Popup")
            .build();
        if let Some(parent) = gtk_transient_parent_for(app, &window) {
            window.set_transient_for(Some(&parent));
        }
        window.add_css_class("vv-popup");
        window.connect_notify_local(Some("is-active"), |window, _| {
            if !window.is_active() {
                window.close();
            }
        });

        let root = GtkBox::new(Orientation::Vertical, 4);
        root.set_margin_top(12);
        root.set_margin_bottom(12);
        root.set_margin_start(14);
        root.set_margin_end(14);
        for (index, command) in plan.text_commands.iter().enumerate() {
            let row = GtkBox::new(Orientation::Horizontal, 8);
            let number = Label::new(Some(&(index + 1).to_string()));
            number.set_width_chars(2);
            number.add_css_class("heading");
            number.add_css_class("vv-index");
            let label = Label::new(Some(&command.text));
            label.set_xalign(0.0);
            label.set_hexpand(true);
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            label.add_css_class("monospace");
            label.add_css_class("vv-preview");
            row.append(&number);
            row.append(&label);
            root.append(&row);
        }
        let status = Label::new(Some("vv_select_requested"));
        status.set_xalign(0.0);
        status.add_css_class("dim-label");
        let popup_key_controller = EventControllerKey::new();
        let window_for_keys = window.clone();
        let status_for_keys = status.clone();
        popup_key_controller.connect_key_pressed(move |_, key, _keycode, _state| {
            let Some(trigger_key) = gtk_vv_trigger_key_from_gdk(key) else {
                return glib::Propagation::Proceed;
            };
            match trigger_key {
                NativeHostVvTriggerKey::Escape => {
                    window_for_keys.close();
                    glib::Propagation::Stop
                }
                NativeHostVvTriggerKey::Digit1To9(selected_index) => {
                    let result =
                        crate::linux_app::dispatch_linux_native_vv_select_event(selected_index);
                    let paste = perform_gtk_vv_paste(selected_index, current_group_id);
                    eprintln!(
                        "ZSClip GTK VV popup key select {} -> {}",
                        selected_index, result.event_name
                    );
                    eprintln!(
                        "ZSClip GTK VV popup key paste {} -> {} accepted={} kind={}",
                        selected_index,
                        paste.result_name,
                        paste.accepted,
                        paste.clipboard_kind.unwrap_or("none")
                    );
                    status_for_keys.set_text(&format!(
                        "{}: {} / {}",
                        selected_index, result.event_name, paste.result_name
                    ));
                    window_for_keys.close();
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        });
        window.add_controller(popup_key_controller);
        #[cfg(feature = "vv-paste")]
        {
            for spec in crate::app_core::native_host_vv_select_specs(&plan, width, height) {
                let action = spec.action;
                let selected_index = action.index;
                let button = Button::with_label(&spec.label);
                button.set_widget_name(&spec.id);
                let status = status.clone();
                button.connect_clicked(move |_| {
                    let result =
                        crate::linux_app::dispatch_linux_native_vv_select_event(selected_index);
                    let paste = perform_gtk_vv_paste(selected_index, current_group_id);
                    eprintln!(
                        "ZSClip GTK VV select {} -> {}",
                        selected_index, result.event_name
                    );
                    eprintln!(
                        "ZSClip GTK VV paste {} -> {} accepted={} kind={}",
                        selected_index,
                        paste.result_name,
                        paste.accepted,
                        paste.clipboard_kind.unwrap_or("none")
                    );
                    status.set_text(&format!(
                        "{}: {} / {}",
                        selected_index, result.event_name, paste.result_name
                    ));
                });
                root.append(&button);
            }
        }
        root.append(&status);
        window.set_child(Some(&root));
        window.present();
    }

    fn perform_vv_trigger_demo(app: &Application, current_group_id: i64) {
        let target_token = app.as_ptr() as usize as u64;
        let first = NativeHostVvTriggerInput {
            key: NativeHostVvTriggerKey::TriggerV,
            target_token,
            target_ready: true,
            command_modifier: false,
            popup_menu_active: false,
            now_ms: 1,
        };
        let _ = perform_vv_trigger_input(app, first, current_group_id);
        let second = NativeHostVvTriggerInput { now_ms: 2, ..first };
        let transition = perform_vv_trigger_input(app, second, current_group_id);
        eprintln!(
            "ZSClip GTK VV trigger demo -> {:?} consume={}",
            transition.action, transition.consume_key
        );
    }

    fn install_vv_key_controller(
        window: &ApplicationWindow,
        app: &Application,
        current_group_filter: Rc<Cell<i64>>,
    ) {
        let controller = EventControllerKey::new();
        let app = app.clone();
        controller.connect_key_pressed(move |_, key, _keycode, state| {
            let Some(trigger_key) = gtk_vv_trigger_key_from_gdk(key) else {
                return glib::Propagation::Proceed;
            };
            let target_token = app.as_ptr() as usize as u64;
            let transition = perform_vv_trigger_input(
                &app,
                NativeHostVvTriggerInput {
                    key: trigger_key,
                    target_token,
                    target_ready: true,
                    command_modifier: gtk_vv_has_command_modifier(state),
                    popup_menu_active: false,
                    now_ms: gtk_vv_now_ms(),
                },
                current_group_filter.get(),
            );
            if transition.consume_key {
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        window.add_controller(controller);
    }

    fn install_vv_global_key_tap(app: &Application) {
        let (sender, receiver) = mpsc::channel::<NativeHostVvTriggerInput>();
        thread::spawn(move || {
            let tap = match keytap::Tap::new() {
                Ok(tap) => tap,
                Err(error) => {
                    eprintln!("ZSClip GTK VV global keytap unavailable: {}", error);
                    return;
                }
            };
            eprintln!("ZSClip GTK VV global keytap installed (observe-only)");
            let mut modifiers = GtkVvKeytapModifierState::default();
            for event in tap.iter() {
                modifiers.apply_event(event.kind);
                let Some(input) =
                    gtk_vv_trigger_input_from_keytap_event(event, modifiers.command_modifier())
                else {
                    continue;
                };
                if sender.send(input).is_err() {
                    break;
                }
            }
        });

        let app_for_tap = app.clone();
        glib::timeout_add_local(Duration::from_millis(40), move || {
            for input in receiver.try_iter() {
                let transition = perform_vv_trigger_input(&app_for_tap, input, 0);
                if transition.consume_key {
                    eprintln!("ZSClip GTK VV global keytap cannot consume external key");
                }
            }
            glib::ControlFlow::Continue
        });
    }

    fn gtk_vv_trigger_input_from_keytap_event(
        event: keytap::Event,
        command_modifier: bool,
    ) -> Option<NativeHostVvTriggerInput> {
        let key = match event.kind {
            keytap::EventKind::KeyDown(key) => gtk_vv_trigger_key_from_keytap(key),
            keytap::EventKind::KeyRepeat(_) | keytap::EventKind::KeyUp(_) => return None,
        };
        Some(NativeHostVvTriggerInput {
            key,
            target_token: gtk_vv_global_keytap_target_token(),
            target_ready: true,
            command_modifier,
            popup_menu_active: false,
            now_ms: gtk_vv_now_ms(),
        })
    }

    fn gtk_vv_trigger_key_from_keytap(key: keytap::Key) -> NativeHostVvTriggerKey {
        match key {
            keytap::Key::V => NativeHostVvTriggerKey::TriggerV,
            keytap::Key::Escape => NativeHostVvTriggerKey::Escape,
            keytap::Key::Backspace => NativeHostVvTriggerKey::Backspace,
            keytap::Key::Digit1 => NativeHostVvTriggerKey::Digit1To9(0),
            keytap::Key::Digit2 => NativeHostVvTriggerKey::Digit1To9(1),
            keytap::Key::Digit3 => NativeHostVvTriggerKey::Digit1To9(2),
            keytap::Key::Digit4 => NativeHostVvTriggerKey::Digit1To9(3),
            keytap::Key::Digit5 => NativeHostVvTriggerKey::Digit1To9(4),
            keytap::Key::Digit6 => NativeHostVvTriggerKey::Digit1To9(5),
            keytap::Key::Digit7 => NativeHostVvTriggerKey::Digit1To9(6),
            keytap::Key::Digit8 => NativeHostVvTriggerKey::Digit1To9(7),
            keytap::Key::Digit9 => NativeHostVvTriggerKey::Digit1To9(8),
            _ => NativeHostVvTriggerKey::Other,
        }
    }

    fn gtk_vv_global_keytap_target_token() -> u64 {
        1_u64 << 61
    }

    fn gtk_vv_trigger_key_from_gdk(key: gdk::Key) -> Option<NativeHostVvTriggerKey> {
        if key == gdk::Key::Escape {
            return Some(NativeHostVvTriggerKey::Escape);
        }
        if key == gdk::Key::BackSpace {
            return Some(NativeHostVvTriggerKey::Backspace);
        }
        match key.to_unicode()? {
            'v' | 'V' => Some(NativeHostVvTriggerKey::TriggerV),
            '1'..='9' => Some(NativeHostVvTriggerKey::Digit1To9(
                key.to_unicode()? as usize - '1' as usize,
            )),
            _ => Some(NativeHostVvTriggerKey::Other),
        }
    }

    fn gtk_vv_has_command_modifier(state: gdk::ModifierType) -> bool {
        state.intersects(
            gdk::ModifierType::CONTROL_MASK
                | gdk::ModifierType::ALT_MASK
                | gdk::ModifierType::SUPER_MASK
                | gdk::ModifierType::META_MASK,
        )
    }

    fn gtk_vv_now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or_default()
    }

    fn perform_vv_trigger_input(
        app: &Application,
        input: NativeHostVvTriggerInput,
        current_group_id: i64,
    ) -> NativeHostVvTriggerTransition {
        let transition = crate::linux_native_host::dispatch_gtk_vv_trigger_key(input);
        handle_vv_trigger_transition(app, transition, current_group_id);
        transition
    }

    fn handle_vv_trigger_transition(
        app: &Application,
        transition: NativeHostVvTriggerTransition,
        current_group_id: i64,
    ) {
        match transition.action {
            NativeHostVvTriggerAction::Show { .. } => {
                present_vv_popup_window(app, current_group_id)
            }
            NativeHostVvTriggerAction::Select { index } => {
                let result = crate::linux_app::dispatch_linux_native_vv_select_event(index);
                let paste = perform_gtk_vv_paste(index, current_group_id);
                eprintln!(
                    "ZSClip GTK VV trigger select {} -> {}",
                    index, result.event_name
                );
                eprintln!(
                    "ZSClip GTK VV trigger paste {} -> {} accepted={} kind={}",
                    index,
                    paste.result_name,
                    paste.accepted,
                    paste.clipboard_kind.unwrap_or("none")
                );
            }
            NativeHostVvTriggerAction::Hide => {
                for window in app.windows() {
                    if window.title().as_deref() == Some("ZSClip VV Popup") {
                        window.close();
                    }
                }
            }
            NativeHostVvTriggerAction::Ignore => {}
        }
    }

    fn perform_gtk_vv_paste(index: usize, current_group_id: i64) -> NativeHostVvPasteExecution {
        let paste =
            crate::linux_app::dispatch_linux_native_vv_paste_for_group(index, current_group_id);
        if paste.accepted && paste.backspaces > 0 {
            let deleted = gtk_post_native_delete_backspaces(paste.backspaces);
            eprintln!(
                "ZSClip GTK VV delete backspaces requested={} posted={}",
                paste.backspaces, deleted
            );
        }
        if paste.accepted && paste.paste_shortcut_sent {
            let posted = gtk_post_native_paste_shortcut();
            eprintln!("ZSClip GTK VV native paste shortcut posted={}", posted);
        }
        paste
    }

    fn gtk_post_native_paste_shortcut() -> bool {
        gtk_try_ydotool_paste_shortcut().unwrap_or_else(gtk_try_xdotool_paste_shortcut)
    }

    fn gtk_post_native_delete_backspaces(backspaces: u8) -> u8 {
        let mut posted = 0;
        for _ in 0..backspaces {
            if gtk_try_ydotool_backspace().unwrap_or_else(gtk_try_xdotool_backspace) {
                posted += 1;
            }
        }
        posted
    }

    fn gtk_try_ydotool_paste_shortcut() -> Option<bool> {
        gtk_run_command("ydotool", &["key", "29:1", "47:1", "47:0", "29:0"])
    }

    fn gtk_try_ydotool_backspace() -> Option<bool> {
        gtk_run_command("ydotool", &["key", "14:1", "14:0"])
    }

    fn gtk_try_xdotool_paste_shortcut() -> bool {
        if std::env::var_os("DISPLAY").is_none() {
            return false;
        }
        gtk_run_command("xdotool", &["key", "--clearmodifiers", "ctrl+v"]).unwrap_or(false)
    }

    fn gtk_try_xdotool_backspace() -> bool {
        if std::env::var_os("DISPLAY").is_none() {
            return false;
        }
        gtk_run_command("xdotool", &["key", "--clearmodifiers", "BackSpace"]).unwrap_or(false)
    }

    fn gtk_run_command(program: &str, args: &[&str]) -> Option<bool> {
        match Command::new(program).args(args).status() {
            Ok(status) => Some(status.success()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                eprintln!(
                    "ZSClip GTK native input command {} failed: {}",
                    program, error
                );
                Some(false)
            }
        }
    }

    fn native_edit_plan(
        items: &[NativeHostClipListItemProjection],
        selected_item_id: i64,
    ) -> Option<NativeHostEditTextPlan> {
        let selected_item_id = (selected_item_id > 0).then_some(selected_item_id);
        let mut plan = native_host_edit_text_plan_for_item(items, selected_item_id)?;
        if let Ok(Some(text)) = crate::db_runtime::item_text(plan.item_id) {
            plan.initial_text = text;
        }
        Some(plan)
    }

    fn gtk_edit_text_view_text(text_view: &TextView) -> String {
        let buffer = text_view.buffer();
        buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), true)
            .to_string()
    }

    fn refresh_edit_target(refresh_target: &Option<EditRefreshTarget>) {
        if let Some(target) = refresh_target.as_ref() {
            let refreshed = crate::linux_app::linux_native_host_projected_clip_items_for_group(
                target.current_group_filter.get(),
            );
            *target.items.borrow_mut() = refreshed;
            refresh_clip_rows(&target.rows, &target.items.borrow());
        }
    }

    fn save_gtk_edit_text(
        item_id: i64,
        text_view: &TextView,
        status: &Label,
        refresh_target: &Option<EditRefreshTarget>,
    ) -> bool {
        let text = gtk_edit_text_view_text(text_view);
        let result = crate::linux_app::dispatch_linux_native_edit_text_save(item_id, &text);
        eprintln!(
            "ZSClip GTK edit save item_id={} text_len={} -> {}",
            item_id,
            text.chars().count(),
            result.result_name
        );
        status.set_text(&format!("saved -> {}", result.result_name));
        if result.accepted {
            refresh_edit_target(refresh_target);
        }
        result.accepted
    }

    fn present_gtk_edit_unsaved_changes_dialog(
        window: &ApplicationWindow,
        item_id: i64,
        text_view: &TextView,
        status: &Label,
        refresh_target: &Option<EditRefreshTarget>,
        close_without_prompt: &Rc<Cell<bool>>,
    ) {
        let dialog = MessageDialog::builder()
            .title("Save edited clipboard text?")
            .text("Save edited clipboard text?")
            .secondary_text("The edited clipboard text has unsaved changes.")
            .message_type(MessageType::Warning)
            .buttons(ButtonsType::None)
            .modal(true)
            .transient_for(window)
            .build();
        dialog.add_button("Save", gtk::ResponseType::Yes);
        dialog.add_button("Discard", gtk::ResponseType::No);
        dialog.add_button("Cancel", gtk::ResponseType::Cancel);

        let window_for_response = window.clone();
        let text_view_for_response = text_view.clone();
        let status_for_response = status.clone();
        let refresh_target_for_response = refresh_target.clone();
        let close_without_prompt_for_response = Rc::clone(close_without_prompt);
        dialog.connect_response(move |dialog, response| {
            match response {
                gtk::ResponseType::Yes => {
                    if save_gtk_edit_text(
                        item_id,
                        &text_view_for_response,
                        &status_for_response,
                        &refresh_target_for_response,
                    ) {
                        close_without_prompt_for_response.set(true);
                        window_for_response.close();
                    }
                }
                gtk::ResponseType::No => {
                    close_without_prompt_for_response.set(true);
                    window_for_response.close();
                }
                _ => {}
            }
            dialog.close();
        });
        dialog.present();
    }

    fn request_gtk_edit_close(
        window: &ApplicationWindow,
        initial_text: &str,
        item_id: i64,
        text_view: &TextView,
        status: &Label,
        refresh_target: &Option<EditRefreshTarget>,
        close_without_prompt: &Rc<Cell<bool>>,
    ) -> bool {
        if close_without_prompt.get() {
            return true;
        }
        let current_text = gtk_edit_text_view_text(text_view);
        let close_plan = native_host_edit_text_close_plan(initial_text, &current_text);
        if !close_plan.requires_unsaved_confirmation {
            close_without_prompt.set(true);
            return true;
        }
        present_gtk_edit_unsaved_changes_dialog(
            window,
            item_id,
            text_view,
            status,
            refresh_target,
            close_without_prompt,
        );
        false
    }

    fn present_edit_text_window(
        app: &Application,
        plan: NativeHostEditTextPlan,
        auto_save: bool,
        refresh_target: Option<EditRefreshTarget>,
    ) {
        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(560)
            .default_height(340)
            .title(&format!("ZSClip Edit - {}", plan.title))
            .build();
        window.set_modal(true);
        if let Some(parent) = gtk_transient_parent_for(app, &window) {
            window.set_transient_for(Some(&parent));
        }
        let root = GtkBox::new(Orientation::Vertical, 8);
        root.set_margin_top(16);
        root.set_margin_bottom(16);
        root.set_margin_start(16);
        root.set_margin_end(16);

        let title = Label::new(Some("Edit clipboard text"));
        title.set_xalign(0.0);
        let text_view = TextView::new();
        text_view.set_wrap_mode(gtk::WrapMode::WordChar);
        text_view.set_vexpand(true);
        text_view.buffer().set_text(&plan.initial_text);
        let initial_text = Rc::new(plan.initial_text.clone());
        let close_without_prompt = Rc::new(Cell::new(false));
        let editor_scroller = ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .hscrollbar_policy(PolicyType::Never)
            .vscrollbar_policy(PolicyType::Automatic)
            .build();
        editor_scroller.set_child(Some(&text_view));
        let status = Label::new(Some("Ready"));
        status.set_xalign(0.0);
        status.add_css_class("dim-label");
        let save_buttons: Vec<_> = native_host_edit_text_button_specs()
            .into_iter()
            .map(|spec| {
                let action = spec.action;
                let button = Button::with_label(spec.label);
                button.set_widget_name(spec.id);
                apply_gtk_button_style_role(&button, spec.style_role);
                let text_view_for_save = text_view.clone();
                let status_for_save = status.clone();
                let refresh_target = refresh_target.clone();
                let item_id = plan.item_id;
                let window_for_cancel = window.clone();
                let initial_text_for_cancel = Rc::clone(&initial_text);
                let close_without_prompt_for_cancel = Rc::clone(&close_without_prompt);
                button.connect_clicked(move |_| match action {
                    NativeHostEditTextAction::Save => {
                        if save_gtk_edit_text(
                            item_id,
                            &text_view_for_save,
                            &status_for_save,
                            &refresh_target,
                        ) {
                            close_without_prompt_for_cancel.set(true);
                            window_for_cancel.close();
                        }
                    }
                    NativeHostEditTextAction::Cancel => {
                        if request_gtk_edit_close(
                            &window_for_cancel,
                            &initial_text_for_cancel,
                            item_id,
                            &text_view_for_save,
                            &status_for_save,
                            &refresh_target,
                            &close_without_prompt_for_cancel,
                        ) {
                            window_for_cancel.close();
                        }
                    }
                });
                button
            })
            .collect();

        let window_for_close = window.clone();
        let text_view_for_close = text_view.clone();
        let status_for_close = status.clone();
        let refresh_target_for_close = refresh_target.clone();
        let initial_text_for_close = Rc::clone(&initial_text);
        let close_without_prompt_for_close = Rc::clone(&close_without_prompt);
        let item_id_for_close = plan.item_id;
        window.connect_close_request(move |_| {
            if request_gtk_edit_close(
                &window_for_close,
                &initial_text_for_close,
                item_id_for_close,
                &text_view_for_close,
                &status_for_close,
                &refresh_target_for_close,
                &close_without_prompt_for_close,
            ) {
                glib::Propagation::Proceed
            } else {
                glib::Propagation::Stop
            }
        });

        root.append(&title);
        root.append(&editor_scroller);
        for button in &save_buttons {
            root.append(button);
        }
        root.append(&status);
        window.set_child(Some(&root));
        window.present();
        eprintln!("ZSClip GTK edit window shown");

        if auto_save {
            if let Some(save) = save_buttons.first() {
                save.emit_clicked();
            }
        }
    }

    fn gtk_transient_parent_for(
        app: &Application,
        child: &ApplicationWindow,
    ) -> Option<gtk::Window> {
        app.active_window()
            .filter(|parent| parent.as_ptr() != child.as_ptr().cast())
            .or_else(|| {
                app.windows()
                    .into_iter()
                    .find(|parent| parent.as_ptr() != child.as_ptr().cast())
            })
    }

    #[derive(Clone)]
    struct EditRefreshTarget {
        rows: Vec<ListBoxRow>,
        items: Rc<RefCell<Vec<NativeHostClipListItemProjection>>>,
        current_group_filter: Rc<Cell<i64>>,
    }

    fn gtk_clip_row_content(
        presentation: Option<&NativeHostClipRowPresentation>,
        fallback_label: &str,
    ) -> GtkBox {
        let row_box = GtkBox::new(Orientation::Horizontal, 8);
        row_box.add_css_class("clip-row-content");
        row_box.set_hexpand(true);
        if let Some(presentation) = presentation {
            row_box.set_tooltip_text(Some(&presentation.accessibility_label));
        }

        let icon = Image::from_icon_name(gtk_clip_row_icon_name(
            presentation.map(|presentation| presentation.kind_icon),
        ));
        icon.set_pixel_size(24);
        row_box.append(&icon);

        let text_box = GtkBox::new(Orientation::Vertical, 2);
        text_box.set_hexpand(true);

        let title = presentation
            .map(|presentation| presentation.title.as_str())
            .filter(|title| !title.trim().is_empty())
            .unwrap_or(fallback_label);
        let preview = presentation
            .map(|presentation| presentation.preview.as_str())
            .filter(|preview| !preview.trim().is_empty())
            .unwrap_or("");

        let title_label = Label::new(Some(title));
        title_label.set_xalign(0.0);
        title_label.set_hexpand(true);
        title_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        title_label.add_css_class("heading");

        let preview_label = Label::new(Some(preview));
        preview_label.set_xalign(0.0);
        preview_label.set_hexpand(true);
        preview_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        preview_label.set_lines(1);
        preview_label.add_css_class("dim-label");

        text_box.append(&title_label);
        text_box.append(&preview_label);
        row_box.append(&text_box);

        let pin_icon = Image::from_icon_name("view-pin-symbolic");
        pin_icon.set_pixel_size(16);
        pin_icon.set_visible(
            presentation
                .and_then(|presentation| presentation.pin_badge)
                .is_some(),
        );
        pin_icon.add_css_class("clip-row-pin");
        row_box.append(&pin_icon);

        row_box
    }

    fn gtk_clip_row_icon_name(kind_icon: Option<NativeHostClipKindIcon>) -> &'static str {
        match kind_icon.unwrap_or(NativeHostClipKindIcon::Text) {
            NativeHostClipKindIcon::Text => "text-x-generic-symbolic",
            NativeHostClipKindIcon::Image => "image-x-generic-symbolic",
            NativeHostClipKindIcon::Phrase => "format-text-symbolic",
            NativeHostClipKindIcon::Files | NativeHostClipKindIcon::Folder => "folder-symbolic",
        }
    }

    fn refresh_clip_rows(rows: &[ListBoxRow], items: &[NativeHostClipListItemProjection]) {
        let specs = native_host_clip_row_specs(items, rows.len());
        for (index, (row, spec)) in rows.iter().zip(specs.iter()).enumerate() {
            let action = spec.action;
            let presentation = items
                .get(index)
                .map(native_host_clip_row_presentation_for_projection);
            row.set_child(Some(&gtk_clip_row_content(
                presentation.as_ref(),
                &spec.label,
            )));
            row.set_widget_name(&action.item_id.to_string());
            row.set_visible(action.has_item());
            row.set_selectable(action.has_item());
            row.set_activatable(action.has_item());
        }
    }

    #[derive(Clone)]
    struct NativeSettingsEntryBinding {
        control_key: &'static str,
        initial_value: String,
        entry: Entry,
    }

    #[derive(Clone)]
    struct NativeSettingsToggleBinding {
        control_key: &'static str,
        initial_value: bool,
        switch: Switch,
    }

    #[derive(Clone)]
    struct NativeSettingsDropdownBinding {
        control_key: &'static str,
        initial_value: String,
        option_count: u32,
        raw_values: Vec<String>,
        dropdown: DropDown,
    }

    #[derive(Clone, Default)]
    struct NativeSettingsControlBindings {
        entries: Vec<NativeSettingsEntryBinding>,
        toggles: Vec<NativeSettingsToggleBinding>,
        dropdowns: Vec<NativeSettingsDropdownBinding>,
    }

    fn native_settings_dropdown_options_for_host(
        control: &crate::settings_model::SettingsNativeControlSummary,
        settings_json: &serde_json::Value,
    ) -> Option<crate::settings_model::SettingsNativeDropdownOptions> {
        crate::settings_model::settings_native_dropdown_options(control, settings_json).or_else(
            || {
                let category =
                    crate::settings_model::settings_native_vv_source_tab(settings_json) as i64;
                let groups = crate::db_runtime::native_clip_groups(category).unwrap_or_default();
                crate::settings_model::settings_native_vv_group_dropdown_options(
                    control,
                    settings_json,
                    groups.iter().map(|group| (group.id, group.name.as_str())),
                )
            },
        )
    }

    fn append_settings_control_blueprint(
        root: &GtkBox,
        route_status: &Label,
        controls: &[crate::settings_model::SettingsNativeControlSummary],
        settings_json: &serde_json::Value,
    ) -> NativeSettingsControlBindings {
        let title = Label::new(native_host_settings_section_label("settings_controls"));
        title.set_xalign(0.0);
        root.append(&title);

        let mut bindings = NativeSettingsControlBindings::default();
        for control in controls {
            let text = format!(
                "{} / {}: {} -> {} / {}",
                control.page_label,
                control.section_title,
                control.label,
                control.route_label(),
                control.binding_label()
            );
            match control.kind {
                crate::settings_model::SettingsNativeControlKind::Toggle => {
                    let row = GtkBox::new(Orientation::Horizontal, 8);
                    row.add_css_class("settings-row");
                    let label = Label::new(Some(&text));
                    label.set_xalign(0.0);
                    label.set_hexpand(true);
                    let switch = Switch::new();
                    if let Some(display) =
                        crate::settings_model::settings_native_control_display_value(
                            control,
                            settings_json,
                        )
                    {
                        let initial_value = display.value.eq_ignore_ascii_case("true");
                        switch.set_active(initial_value);
                        bindings.toggles.push(NativeSettingsToggleBinding {
                            control_key: control.key,
                            initial_value,
                            switch: switch.clone(),
                        });
                    }
                    row.append(&label);
                    row.append(&switch);
                    root.append(&row);
                }
                crate::settings_model::SettingsNativeControlKind::TextInput => {
                    let row = GtkBox::new(Orientation::Horizontal, 8);
                    row.add_css_class("settings-row");
                    let label = Label::new(Some(&text));
                    label.set_xalign(0.0);
                    let entry = Entry::new();
                    entry.set_placeholder_text(Some(control.key));
                    let mut initial_value = String::new();
                    if let Some(display) =
                        crate::settings_model::settings_native_control_display_value(
                            control,
                            settings_json,
                        )
                    {
                        if display.sensitive {
                            entry.set_placeholder_text(Some("stored securely"));
                        } else {
                            entry.set_text(&display.value);
                            initial_value = display.value;
                        }
                    }
                    bindings.entries.push(NativeSettingsEntryBinding {
                        control_key: control.key,
                        initial_value,
                        entry: entry.clone(),
                    });
                    row.append(&label);
                    row.append(&entry);
                    root.append(&row);
                }
                crate::settings_model::SettingsNativeControlKind::Dropdown => {
                    if let Some(options) =
                        native_settings_dropdown_options_for_host(control, settings_json)
                    {
                        let row = GtkBox::new(Orientation::Horizontal, 8);
                        row.add_css_class("settings-row");
                        let label = Label::new(Some(&text));
                        label.set_xalign(0.0);
                        let labels = options
                            .options
                            .iter()
                            .map(|option| option.label.as_str())
                            .collect::<Vec<_>>();
                        let dropdown = DropDown::from_strings(&labels);
                        let initial_value = options
                            .options
                            .get(options.selected_index)
                            .map(|option| option.raw_value.clone())
                            .unwrap_or_default();
                        dropdown.set_selected(options.selected_index as u32);
                        bindings.dropdowns.push(NativeSettingsDropdownBinding {
                            control_key: options.control_key,
                            initial_value,
                            option_count: options.options.len() as u32,
                            raw_values: options
                                .options
                                .iter()
                                .map(|option| option.raw_value.clone())
                                .collect(),
                            dropdown: dropdown.clone(),
                        });
                        row.append(&label);
                        row.append(&dropdown);
                        root.append(&row);
                    } else {
                        let row =
                            Button::with_label(&format!("{} [{}]", text, control.kind.role_name()));
                        row.set_halign(gtk::Align::Start);
                        row.add_css_class("settings-row");
                        root.append(&row);
                    }
                }
                crate::settings_model::SettingsNativeControlKind::Button => {
                    let row =
                        Button::with_label(&format!("{} [{}]", text, control.kind.role_name()));
                    row.set_halign(gtk::Align::Start);
                    row.add_css_class("settings-row");
                    if let Some(control_route) = control.route {
                        if control_route.kind
                            == crate::settings_model::SettingsNativeControlRouteKind::Action
                        {
                            if let Some(action_name) = control_route.action_name {
                                let route_name = control_route.route_name;
                                let action_name = action_name;
                                let route_status = route_status.clone();
                                row.connect_clicked(move |_| {
                                    let result =
                                        crate::linux_app::dispatch_linux_native_settings_route_action(
                                            route_name,
                                            action_name,
                                        );
                                    eprintln!(
                                        "ZSClip GTK settings route action {}/{} -> {}",
                                        route_name, action_name, result.result_name
                                    );
                                    route_status.set_text(&result.result_name);
                                });
                            }
                        }
                    }
                    root.append(&row);
                }
                crate::settings_model::SettingsNativeControlKind::List => {
                    let row =
                        Button::with_label(&format!("{} [{}]", text, control.kind.role_name()));
                    row.set_halign(gtk::Align::Start);
                    row.add_css_class("settings-row");
                    root.append(&row);
                }
                crate::settings_model::SettingsNativeControlKind::Label => {
                    let row = Label::new(Some(&format!("{} [{}]", text, control.kind.role_name())));
                    row.set_xalign(0.0);
                    row.add_css_class("settings-row");
                    root.append(&row);
                }
            }
        }
        bindings
    }

    fn apply_gtk_settings_control_action(
        action: NativeHostSettingsControlAction,
        bindings: &NativeSettingsControlBindings,
    ) -> bool {
        let Some(control_key) = action.binding_control_key() else {
            return false;
        };
        if action.role() == SettingsControlRole::Toggle {
            if let Some(binding) = bindings
                .toggles
                .iter()
                .find(|binding| binding.control_key == control_key)
            {
                binding.switch.set_active(!binding.switch.is_active());
                return true;
            }
        }
        if action.role() == SettingsControlRole::Dropdown {
            if let Some(binding) = bindings
                .dropdowns
                .iter()
                .find(|binding| binding.control_key == control_key)
            {
                let next_index = binding.dropdown.selected().saturating_add(1);
                let next_index = if next_index < binding.option_count {
                    next_index
                } else {
                    0
                };
                binding.dropdown.set_selected(next_index);
                return true;
            }
        }
        false
    }

    fn present_settings_window(
        app: &Application,
        route_name: &str,
        group_popup_menus: Option<GroupPopupMenus>,
    ) {
        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(720)
            .default_height(520)
            .title("ZSClip Settings")
            .build();

        let root = GtkBox::new(Orientation::Vertical, 10);
        root.set_margin_top(24);
        root.set_margin_bottom(24);
        root.set_margin_start(24);
        root.set_margin_end(24);

        let title = Label::new(Some("Settings"));
        title.add_css_class("title-2");
        title.set_xalign(0.0);

        let route = Label::new(Some(route_name));
        route.add_css_class("dim-label");
        route.set_xalign(0.0);

        root.append(&title);
        root.append(&route);
        let notebook = Notebook::new();
        notebook.set_hexpand(true);
        notebook.set_vexpand(true);

        let mut settings_page = None;
        let mut group_page = None;
        let mut actions_page = None;
        for spec in native_host_settings_page_tab_specs() {
            let page = GtkBox::new(Orientation::Vertical, 10);
            page.set_margin_top(12);
            page.set_margin_bottom(12);
            page.set_margin_start(12);
            page.set_margin_end(12);
            let scroller = ScrolledWindow::builder()
                .hexpand(true)
                .vexpand(true)
                .hscrollbar_policy(PolicyType::Never)
                .vscrollbar_policy(PolicyType::Automatic)
                .build();
            scroller.set_child(Some(&page));
            notebook.append_page(&scroller, Some(&Label::new(Some(spec.label))));
            match spec.kind {
                NativeSettingsPageTabKind::General => settings_page = Some(page),
                NativeSettingsPageTabKind::Groups => group_page = Some(page),
                NativeSettingsPageTabKind::Actions => actions_page = Some(page),
            }
        }
        let settings_page = settings_page.expect("settings general tab spec");
        let group_page = group_page.expect("settings groups tab spec");
        let actions_page = actions_page.expect("settings actions tab spec");
        root.append(&notebook);

        let page_summaries = crate::settings_model::settings_native_page_summaries();
        let section_summaries = crate::settings_model::settings_native_section_summaries();
        let control_summaries = crate::settings_model::settings_native_control_summaries();
        let settings_json = crate::linux_app::linux_native_settings_json_snapshot();
        eprintln!(
            "ZSClip GTK settings native page summaries count={} section summaries count={} control summaries count={}",
            page_summaries.len(),
            section_summaries.len(),
            control_summaries.len()
        );
        for summary in page_summaries {
            let control_rows = section_summaries
                .iter()
                .filter(|section| section.page == summary.page)
                .map(|section| section.control_rows)
                .sum::<i32>();
            let control_count = control_summaries
                .iter()
                .filter(|control| control.page == summary.page)
                .count();
            let row = Label::new(Some(&format!(
                "{}: {} sections, {} control rows, {} native controls",
                summary.label,
                summary.section_titles.len(),
                control_rows,
                control_count
            )));
            row.set_xalign(0.0);
            settings_page.append(&row);
        }
        let native_control_bindings = append_settings_control_blueprint(
            &settings_page,
            &route,
            &control_summaries,
            &settings_json,
        );

        let group_category = Rc::new(Cell::new(0_i64));
        let selected_group_id = Rc::new(Cell::new(0_i64));
        let group_title = Label::new(native_host_settings_section_label("group_selector"));
        group_title.set_xalign(0.0);
        group_page.append(&group_title);
        let group_editor = GtkBox::new(Orientation::Horizontal, 8);
        let group_name = Entry::new();
        group_name.set_text("新分组");
        group_editor.append(&group_name);

        let group_rows: Vec<_> = (0..5)
            .map(|_| {
                let row = Button::with_label("");
                row.set_hexpand(true);
                row.set_halign(gtk::Align::Fill);
                row.set_visible(false);
                row
            })
            .collect();
        let group_actions = GtkBox::new(Orientation::Horizontal, 8);
        for spec in native_host_settings_group_button_specs() {
            let action = spec.action;
            let button = Button::with_label(spec.label);
            button.set_widget_name(spec.id);
            apply_gtk_button_style_role(&button, spec.style_role);
            if let Some(target_category) = action.target_category() {
                let rows = group_rows.clone();
                let selected = selected_group_id.clone();
                let category = group_category.clone();
                button.connect_clicked(move |_| {
                    category.set(target_category);
                    selected.set(0);
                    refresh_settings_group_rows(category.get(), &selected, &rows);
                });
                group_editor.append(&button);
                continue;
            }

            let route = route.clone();
            let rows = group_rows.clone();
            let selected = selected_group_id.clone();
            let category = group_category.clone();
            let name = group_name.clone();
            let menus = group_popup_menus.clone();
            button.connect_clicked(move |_| {
                let result = match action {
                    NativeHostSettingsGroupAction::Add => {
                        crate::linux_app::dispatch_linux_native_create_group(
                            category.get(),
                            name.text().as_str(),
                        )
                    }
                    NativeHostSettingsGroupAction::Rename => {
                        crate::linux_app::dispatch_linux_native_rename_group(
                            category.get(),
                            selected.get(),
                            name.text().as_str(),
                        )
                    }
                    NativeHostSettingsGroupAction::Delete => {
                        let result =
                            crate::linux_app::dispatch_linux_native_delete_group(selected.get());
                        selected.set(0);
                        result
                    }
                    NativeHostSettingsGroupAction::MoveUp
                    | NativeHostSettingsGroupAction::MoveDown => {
                        crate::linux_app::dispatch_linux_native_move_group(
                            category.get(),
                            selected.get(),
                            action
                                .move_step()
                                .expect("move group action must define a step")
                                as i32,
                        )
                    }
                    NativeHostSettingsGroupAction::ShowRecords
                    | NativeHostSettingsGroupAction::ShowPhrases => unreachable!(
                        "settings group category actions are handled before operation actions"
                    ),
                };
                route.set_text(&result.result_name);
                refresh_settings_group_rows(category.get(), &selected, &rows);
                refresh_group_popup_menus_for_category(category.get(), menus.as_ref());
            });
            group_actions.append(&button);
        }
        group_page.append(&group_editor);
        for row in &group_rows {
            let selected_group_id = selected_group_id.clone();
            let group_category = group_category.clone();
            let group_name = group_name.clone();
            let rows = group_rows.clone();
            row.connect_clicked(move |button| {
                let group_id = button.widget_name().parse::<i64>().unwrap_or_default();
                if group_id > 0 {
                    select_settings_group(
                        group_id,
                        group_category.get(),
                        &selected_group_id,
                        &group_name,
                        &rows,
                    );
                }
            });
            group_page.append(row);
        }
        refresh_settings_group_rows(group_category.get(), &selected_group_id, &group_rows);
        group_page.append(&group_actions);

        let controls = GtkBox::new(Orientation::Horizontal, 8);
        let _settings_control_compat_specs = native_host_settings_control_button_specs();
        for spec in native_host_settings_toggle_specs() {
            let action = spec.action;
            let row = GtkBox::new(Orientation::Horizontal, 6);
            row.add_css_class("settings-row");
            let label = Label::new(Some(spec.label));
            label.set_xalign(0.0);
            let switch = Switch::new();
            switch.set_widget_name(spec.id);
            let route = route.clone();
            let native_control_bindings = native_control_bindings.clone();
            switch.connect_active_notify(move |_| {
                let result =
                    crate::linux_app::dispatch_linux_native_settings_control_action(action);
                let applied = apply_gtk_settings_control_action(action, &native_control_bindings);
                eprintln!(
                    "ZSClip GTK settings toggle action {} -> {} applied={}",
                    action.action_name(),
                    result.result_name,
                    applied
                );
                route.set_text(&format!("{} | applied={}", result.result_name, applied));
            });
            row.append(&label);
            row.append(&switch);
            controls.append(&row);
        }
        for spec in native_host_settings_dropdown_specs() {
            let action = spec.action;
            let labels = if spec.options.is_empty() {
                native_control_bindings
                    .dropdowns
                    .iter()
                    .find(|binding| action.binding_control_key() == Some(binding.control_key))
                    .map(|binding| {
                        binding
                            .raw_values
                            .iter()
                            .map(String::as_str)
                            .collect::<Vec<_>>()
                    })
                    .filter(|labels| !labels.is_empty())
                    .unwrap_or_else(|| vec![spec.label])
            } else {
                spec.options
                    .iter()
                    .map(|option| option.label)
                    .collect::<Vec<_>>()
            };
            let dropdown = DropDown::from_strings(&labels);
            if let Some(binding) = native_control_bindings
                .dropdowns
                .iter()
                .find(|binding| action.binding_control_key() == Some(binding.control_key))
            {
                if let Some(index) = spec
                    .options
                    .iter()
                    .position(|option| option.raw_value == binding.initial_value)
                {
                    dropdown.set_selected(index as u32);
                }
            }
            dropdown.set_widget_name(spec.id);
            let route = route.clone();
            let native_control_bindings = native_control_bindings.clone();
            dropdown.connect_selected_notify(move |_| {
                let result =
                    crate::linux_app::dispatch_linux_native_settings_control_action(action);
                let applied = apply_gtk_settings_control_action(action, &native_control_bindings);
                eprintln!(
                    "ZSClip GTK settings dropdown action {} -> {} applied={}",
                    action.action_name(),
                    result.result_name,
                    applied
                );
                route.set_text(&format!("{} | applied={}", result.result_name, applied));
            });
            controls.append(&dropdown);
        }
        settings_page.append(&controls);

        let actions = GtkBox::new(Orientation::Horizontal, 8);
        for spec in native_host_settings_action_button_specs() {
            let action = spec.action;
            let button = Button::with_label(spec.label);
            button.set_widget_name(spec.id);
            apply_gtk_button_style_role(&button, spec.style_role);
            let route = route.clone();
            let window = window.clone();
            let native_control_bindings = native_control_bindings.clone();
            button.connect_clicked(move |_| {
                let plan = if matches!(action, NativeHostSettingsAction::Save) {
                    let plan = crate::settings_model::settings_native_apply_collect_plan();
                    let mut submitted_values = native_control_bindings
                        .entries
                        .iter()
                        .map(|binding| {
                            let raw_value = binding.entry.text().to_string();
                            crate::settings_model::SettingsNativeSubmittedControlValue {
                                control_key: binding.control_key.to_string(),
                                raw_value,
                            }
                        })
                        .collect::<Vec<_>>();
                    submitted_values.extend(native_control_bindings.toggles.iter().map(
                        |binding| {
                            let value = binding.switch.is_active();
                            crate::settings_model::SettingsNativeSubmittedControlValue {
                                control_key: binding.control_key.to_string(),
                                raw_value: value.to_string(),
                            }
                        },
                    ));
                    submitted_values.extend(native_control_bindings.dropdowns.iter().filter_map(
                        |binding| {
                            let raw_value = binding
                                .raw_values
                                .get(binding.dropdown.selected() as usize)?
                                .to_string();
                            Some(crate::settings_model::SettingsNativeSubmittedControlValue {
                                control_key: binding.control_key.to_string(),
                                raw_value,
                            })
                        },
                    ));
                    let submission = crate::settings_model::settings_native_collect_submission(
                        &submitted_values,
                    );
                    let json_apply =
                        crate::settings_model::settings_native_apply_submission_to_json(
                            serde_json::json!({}),
                            &submission,
                        );
                    let persist_result =
                        crate::linux_app::persist_linux_native_settings_submission(&submission);
                    let label = format!(
                        "{} | {} | {} | {}",
                        plan.summary_label(),
                        submission.summary_label(),
                        json_apply.summary_label(),
                        persist_result.result_name
                    );
                    eprintln!("ZSClip GTK settings apply/collect submission -> {}", label);
                    Some(label)
                } else {
                    None
                };
                let result = crate::linux_app::dispatch_linux_native_settings_action(action);
                eprintln!(
                    "ZSClip GTK settings action {} -> {}",
                    action.action_name(),
                    result.result_name
                );
                if let Some(plan) = plan {
                    route.set_text(&format!("{} | {}", result.result_name, plan));
                } else {
                    route.set_text(&result.result_name);
                }
                if action.should_close_settings_surface() {
                    window.close();
                }
            });
            actions.append(&button);
        }
        actions_page.append(&actions);

        let platform_actions = GtkBox::new(Orientation::Horizontal, 8);
        for spec in native_host_settings_platform_button_specs() {
            let action = spec.action;
            let button = Button::with_label(spec.label);
            button.set_widget_name(spec.id);
            let route = route.clone();
            button.connect_clicked(move |_| {
                let result =
                    crate::linux_app::dispatch_linux_native_settings_platform_action(action);
                eprintln!(
                    "ZSClip GTK settings platform action {} -> {}",
                    action.action_name(),
                    result.result_name
                );
                route.set_text(&result.result_name);
            });
            platform_actions.append(&button);
        }
        actions_page.append(&platform_actions);

        let dialog_actions = GtkBox::new(Orientation::Horizontal, 8);
        for spec in native_host_dialog_button_specs() {
            let action = spec.action;
            let button = Button::with_label(spec.label);
            button.set_widget_name(spec.id);
            let route = route.clone();
            let window = window.clone();
            button.connect_clicked(move |_| {
                let result = present_gtk_dialog_action(&window, action);
                eprintln!(
                    "ZSClip GTK dialog action {} -> {}",
                    action.action_name(),
                    result.result_name
                );
                route.set_text(&result.result_name);
            });
            dialog_actions.append(&button);
        }
        actions_page.append(&dialog_actions);

        window.set_child(Some(&root));
        window.present();
    }

    fn select_settings_group(
        group_id: i64,
        category: i64,
        selected_group_id: &Cell<i64>,
        group_name: &Entry,
        rows: &[Button],
    ) {
        selected_group_id.set(group_id);
        if let Ok(groups) = crate::db_runtime::native_clip_groups(category) {
            if let Some(group) = groups.into_iter().find(|group| group.id == group_id) {
                group_name.set_text(&group.name);
            }
        }
        refresh_settings_group_rows(category, selected_group_id, rows);
        eprintln!("ZSClip GTK settings group selected id={}", group_id);
    }

    fn refresh_settings_group_rows(category: i64, selected_group_id: &Cell<i64>, rows: &[Button]) {
        let groups = crate::db_runtime::native_clip_groups(category).unwrap_or_default();
        let mut selected = selected_group_id.get();
        if selected == 0 || !groups.iter().any(|group| group.id == selected) {
            selected = groups.first().map(|group| group.id).unwrap_or_default();
            selected_group_id.set(selected);
        }
        for (index, row) in rows.iter().enumerate() {
            if let Some(group) = groups.get(index) {
                let prefix = if group.id == selected { "> " } else { "  " };
                row.set_label(&format!("{}{}", prefix, group.name));
                row.set_widget_name(&group.id.to_string());
                row.set_visible(true);
            } else {
                row.set_label("");
                row.set_widget_name("0");
                row.set_visible(false);
            }
        }
    }

    fn present_gtk_dialog_action(
        parent: &ApplicationWindow,
        action: NativeHostDialogAction,
    ) -> ProductAdapterCommandResult {
        match action {
            NativeHostDialogAction::ShowInfoMessage => {
                let result = crate::linux_app::dispatch_linux_native_dialog_action(action);
                let dialog = MessageDialog::builder()
                    .title(action.title())
                    .text(action.title())
                    .secondary_text(action.message())
                    .message_type(MessageType::Info)
                    .buttons(ButtonsType::Ok)
                    .modal(true)
                    .transient_for(parent)
                    .build();
                dialog.connect_response(|dialog, _| dialog.close());
                dialog.present();
                result
            }
            NativeHostDialogAction::ConfirmQuestion => {
                let dialog = MessageDialog::builder()
                    .title(action.title())
                    .text(action.title())
                    .secondary_text(action.message())
                    .message_type(MessageType::Question)
                    .buttons(ButtonsType::YesNo)
                    .modal(true)
                    .transient_for(parent)
                    .build();
                dialog.connect_response(move |dialog, response| {
                    let native_response = match response {
                        gtk::ResponseType::Yes => NativeDialogResponse::Yes,
                        gtk::ResponseType::No => NativeDialogResponse::No,
                        _ => NativeDialogResponse::Cancel,
                    };
                    eprintln!(
                        "ZSClip GTK dialog action {} -> zsclip.dialog.confirm_{}",
                        action.action_name(),
                        native_dialog_response_name(native_response)
                    );
                    dialog.close();
                });
                dialog.present();
                ProductAdapterCommandResult {
                    accepted: true,
                    result_name: "zsclip.dialog.confirm_pending".to_string(),
                }
            }
        }
    }

    fn native_dialog_response_name(response: NativeDialogResponse) -> &'static str {
        match response {
            NativeDialogResponse::Yes => "yes",
            NativeDialogResponse::No => "no",
            NativeDialogResponse::Cancel => "cancel",
        }
    }

    fn update_clip_list_visibility(
        rows: &[ListBoxRow],
        items: &[NativeHostClipListItemProjection],
        selected_item_id: &Cell<i64>,
        query: &str,
    ) {
        let visible_ids = native_host_filtered_projected_clip_item_ids(items, query);
        for row in rows {
            let item_id = row.widget_name().parse::<i64>().unwrap_or_default();
            row.set_visible(item_id > 0 && visible_ids.contains(&item_id));
        }
        let visible_items = items
            .iter()
            .filter(|item| visible_ids.contains(&item.id))
            .cloned()
            .collect::<Vec<_>>();
        reconcile_selected_item_id(selected_item_id, &visible_items);
    }
}

#[cfg(target_os = "linux")]
pub(crate) fn real_gtk_host_is_compiled() -> bool {
    true
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn real_gtk_host_is_compiled() -> bool {
    false
}

#[cfg(target_os = "linux")]
pub(crate) fn run_real_gtk_host(summary: LinuxHostContractSummary) -> Result<(), String> {
    gtk_host::run(summary)
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn run_real_gtk_host(_summary: LinuxHostContractSummary) -> Result<(), String> {
    Err("GTK host can only be launched on Linux".to_string())
}

pub(crate) fn dispatch_gtk_host_action(action: NativeHostUiAction) -> ProductAdapterCommandResult {
    crate::linux_app::dispatch_linux_native_host_action(action)
}

pub(crate) fn dispatch_gtk_settings_action(
    action: NativeHostSettingsAction,
) -> ProductAdapterCommandResult {
    crate::linux_app::dispatch_linux_native_settings_action(action)
}

pub(crate) fn dispatch_gtk_settings_control_action(
    action: crate::app_core::NativeHostSettingsControlAction,
) -> ProductAdapterCommandResult {
    crate::linux_app::dispatch_linux_native_settings_control_action(action)
}

pub(crate) fn dispatch_gtk_settings_platform_action(
    action: NativeHostSettingsPlatformAction,
) -> ProductAdapterCommandResult {
    crate::linux_app::dispatch_linux_native_settings_platform_action(action)
}

pub(crate) fn dispatch_gtk_dialog_action(
    action: NativeHostDialogAction,
) -> ProductAdapterCommandResult {
    crate::linux_app::dispatch_linux_native_dialog_action(action)
}

pub(crate) fn dispatch_gtk_status_menu_action(
    action: NativeHostStatusMenuAction,
) -> ProductAdapterCommandResult {
    crate::linux_app::dispatch_linux_native_status_menu_action(action)
}

pub(crate) fn dispatch_gtk_menu_command_id(menu_id: usize) -> ProductAdapterCommandResult {
    crate::linux_app::dispatch_linux_native_menu_command_id(menu_id)
}

pub(crate) fn dispatch_gtk_vv_select_event(index: usize) -> ProductAdapterAsyncBridgeResult {
    crate::linux_app::dispatch_linux_native_vv_select_event(index)
}

#[allow(dead_code)]
pub(crate) fn dispatch_gtk_vv_trigger_key(
    input: NativeHostVvTriggerInput,
) -> NativeHostVvTriggerTransition {
    crate::linux_app::dispatch_linux_native_vv_trigger_key(input)
}

pub(crate) fn dispatch_gtk_vv_paste(index: usize) -> NativeHostVvPasteExecution {
    crate::linux_app::dispatch_linux_native_vv_paste(index)
}

pub(crate) fn dispatch_gtk_row_action(action: NativeHostRowAction) -> ProductAdapterCommandResult {
    crate::linux_app::dispatch_linux_native_row_action(action)
}

pub(crate) fn dispatch_gtk_search_text_action(
    action: NativeHostSearchTextAction,
) -> ProductAdapterCommandResult {
    crate::linux_app::dispatch_linux_native_search_text_action(action)
}

#[cfg(all(test, not(target_os = "linux")))]
mod tests {
    use super::*;

    #[test]
    fn gtk_settings_platform_bridge_is_callable_from_non_target_tests() {
        let result = dispatch_gtk_settings_platform_action(
            NativeHostSettingsPlatformAction::CheckForUpdates,
        );

        assert!(result.accepted);
        assert_eq!(
            result.result_name,
            "zsclip.settings.check_for_updates_failed"
        );
    }

    #[test]
    fn gtk_dialog_bridge_is_callable_from_non_target_tests() {
        let info = dispatch_gtk_dialog_action(NativeHostDialogAction::ShowInfoMessage);
        let confirm = dispatch_gtk_dialog_action(NativeHostDialogAction::ConfirmQuestion);

        assert!(info.accepted);
        assert_eq!(info.result_name, "zsclip.dialog.show_info_message");
        assert!(confirm.accepted);
        assert_eq!(confirm.result_name, "zsclip.dialog.confirm_cancel");
    }
}
