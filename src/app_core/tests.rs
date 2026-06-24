use super::*;

#[test]
fn lifecycle_rejects_out_of_order_transitions() {
    let mut state = LifecycleState::new();
    assert_eq!(state.phase(), ComponentPhase::New);
    assert!(!state.apply(LifecycleEvent::Resume));
    assert!(state.apply(LifecycleEvent::Mount));
    assert_eq!(state.phase(), ComponentPhase::Mounted);
    assert!(state.apply(LifecycleEvent::Resume));
    assert_eq!(state.phase(), ComponentPhase::Active);
    assert!(state.apply(LifecycleEvent::Suspend));
    assert_eq!(state.phase(), ComponentPhase::Suspended);
    assert!(state.apply(LifecycleEvent::Unmount));
    assert_eq!(state.phase(), ComponentPhase::Unmounted);
}

#[test]
fn shared_ui_protocols_are_explicitly_not_platform_host_surfaces() {
    assert_eq!(
        SHARED_NON_HOST_UI_PROTOCOLS,
        [
            SharedUiProtocol::Command,
            SharedUiProtocol::LayoutProtocol,
            SharedUiProtocol::Component
        ]
    );
    let names: Vec<_> = SHARED_NON_HOST_UI_PROTOCOLS
        .iter()
        .map(|protocol| protocol.protocol_name())
        .collect();
    assert_eq!(names, ["Command", "LayoutProtocol", "Component"]);
}

#[test]
fn native_ui_protocol_surfaces_classify_host_entry_points() {
    let surfaces = native_ui_protocol_surfaces();
    assert_eq!(
        surfaces
            .iter()
            .map(|surface| surface.surface_name())
            .collect::<Vec<_>>(),
        vec![
            "main_window",
            "menu",
            "settings_page",
            "dialog",
            "dynamic_controls"
        ]
    );

    let all_builder_names = surfaces
        .iter()
        .flat_map(|surface| {
            surface
                .protocol_builder_names
                .iter()
                .chain(surface.dynamic_protocol_builder_names.iter())
        })
        .copied()
        .collect::<Vec<_>>();
    for builder_name in [
        "native_host_main_action_button_specs",
        "native_host_main_tool_button_specs",
        "native_host_search_input_specs",
        "native_host_row_action_button_specs",
        "native_host_status_menu_item_specs",
        "native_host_full_row_popup_menu_entries_for_groups",
        "native_host_group_filter_popup_menu_entries_for_groups",
        "native_host_settings_action_button_specs",
        "native_host_settings_control_button_specs",
        "native_host_settings_toggle_specs",
        "native_host_settings_dropdown_specs",
        "native_host_settings_group_button_specs",
        "native_host_settings_platform_button_specs",
        "native_host_dialog_button_specs",
        "native_host_edit_text_button_specs",
        "native_host_clip_row_specs",
        #[cfg(feature = "vv-paste")]
        "native_host_vv_select_specs",
    ] {
        assert!(
            all_builder_names.contains(&builder_name),
            "missing native UI protocol builder {builder_name}"
        );
    }

    let dynamic = surfaces
        .iter()
        .find(|surface| surface.kind == NativeUiProtocolSurfaceKind::DynamicControls)
        .unwrap();
    assert!(dynamic.protocol_builder_names.is_empty());
    assert_eq!(
        dynamic.dynamic_protocol_builder_names,
        [
            "native_host_clip_row_specs",
            #[cfg(feature = "vv-paste")]
            "native_host_vv_select_specs"
        ]
    );
    assert_eq!(
        dynamic.action_family_names,
        [
            "ClipRow",
            #[cfg(feature = "vv-paste")]
            "VvSelect"
        ]
    );
    assert!(surfaces
        .iter()
        .all(|surface| surface.platform_host_rule.contains("translate")));
}

#[test]
fn host_private_ui_ingress_audit_classifies_remaining_native_entry_points() {
    let audits = zsui_host_private_ui_ingress_audits();
    assert_eq!(audits.len(), SUPPORTED_NATIVE_UI_PLATFORMS.len() * 5);

    for platform in SUPPORTED_NATIVE_UI_PLATFORMS {
        let platform_audits = zsui_host_private_ui_ingress_audits_for_platform(platform);
        assert_eq!(platform_audits.len(), 5);
        assert_eq!(
            platform_audits
                .iter()
                .map(|audit| audit.surface_name)
                .collect::<Vec<_>>(),
            vec![
                "main_window",
                "menu",
                "settings_page",
                "dialog",
                "dynamic_controls"
            ]
        );
        assert!(platform_audits
            .iter()
            .all(|audit| !audit.protocol_anchor_names.is_empty()));
    }

    let macos_menu = audits
        .iter()
        .find(|audit| audit.platform == NativeUiPlatform::Macos && audit.surface_name == "menu")
        .expect("macOS menu audit");
    assert_eq!(
        macos_menu.audit_status,
        ZsuiHostPrivateUiIngressAuditStatus::PrivateIngressNeedsProtocol
    );
    assert!(macos_menu
        .private_native_entry_names
        .contains(&"disabled_menu_item_policy"));
    assert_eq!(
        macos_menu.next_protocolization_step,
        Some("add disabled-state display policy to app_core menu entries")
    );

    let linux_settings = audits
        .iter()
        .find(|audit| {
            audit.platform == NativeUiPlatform::Linux && audit.surface_name == "settings_page"
        })
        .expect("Linux settings audit");
    assert_eq!(
        linux_settings.audit_status_name,
        "private_ingress_needs_protocol"
    );
    assert!(linux_settings
        .private_native_entry_names
        .contains(&"settings_group_rows"));
    assert!(!linux_settings
        .private_native_entry_names
        .contains(&"settings_notebook_layout"));

    let macos_settings = audits
        .iter()
        .find(|audit| {
            audit.platform == NativeUiPlatform::Macos && audit.surface_name == "settings_page"
        })
        .expect("macOS settings audit");
    assert!(!macos_settings
        .private_native_entry_names
        .contains(&"settings_tab_layout"));

    let menu_audits = zsui_host_private_ui_ingress_audits_for_surface("menu");
    assert_eq!(menu_audits.len(), 3);
    assert_eq!(
        menu_audits
            .iter()
            .filter(|audit| audit.audit_status.needs_protocol_work())
            .count(),
        2
    );

    let work_items = zsui_host_private_ui_ingress_protocol_work_items();
    assert!(work_items.iter().any(|audit| {
        audit.platform == NativeUiPlatform::Macos && audit.surface_name == "settings_page"
    }));
    assert!(work_items
        .iter()
        .any(|audit| audit.platform == NativeUiPlatform::Linux && audit.surface_name == "menu"));

    let manifest = zsui_framework_manifest();
    assert_eq!(manifest.host_private_ui_ingress_audits, audits);
    let context = zsui_agent_context();
    assert_eq!(context.host_private_ui_ingress_audits, audits);
}

#[test]
fn ui_ingress_classifications_keep_ai_edits_in_app_core_first() {
    let classifications = zsui_ui_ingress_classifications();
    assert_eq!(
        classifications
            .iter()
            .map(|entry| entry.ingress_name)
            .collect::<Vec<_>>(),
        vec![
            "main_window",
            "menu",
            "settings_page",
            "dialog",
            "dynamic_controls"
        ]
    );

    for entry in &classifications {
        assert!(
            entry
                .preferred_app_core_edit_modules
                .contains(&"src/app_core/native_host_actions.rs"),
            "{} should route action additions through native_host_actions first",
            entry.ingress_name
        );
        assert!(
            entry
                .preferred_app_core_edit_modules
                .contains(&"src/app_core/native_component_protocol.rs"),
            "{} should route component additions through native_component_protocol first",
            entry.ingress_name
        );
        assert!(
            entry
                .preferred_app_core_edit_modules
                .iter()
                .all(|module| module.starts_with("src/app_core/")),
            "{} preferred edit modules must stay in app_core",
            entry.ingress_name
        );
        assert!(entry.platform_host_scope.contains("translate"));
        assert!(!entry.platform_adapter_touchpoints.is_empty());
        assert!(
            !entry.protocol_builder_names.is_empty()
                || !entry.dynamic_protocol_builder_names.is_empty()
        );
    }

    let settings = classifications
        .iter()
        .find(|entry| entry.ingress_name == "settings_page")
        .unwrap();
    assert!(settings
        .preferred_app_core_edit_modules
        .contains(&"src/app_core/settings_protocol.rs"));
    assert_eq!(
        settings.platform_adapter_touchpoints,
        vec![
            "src/settings_ui_host.rs",
            "src/macos_native_host.rs",
            "src/linux_native_host.rs"
        ]
    );

    let manifest = zsui_framework_manifest();
    assert_eq!(manifest.ui_ingress_classifications, classifications);
    let context = zsui_agent_context();
    assert_eq!(context.ui_ingress_classifications, classifications);
}

#[test]
fn ui_protocol_convergence_report_tracks_protocolized_ingress_without_smoke_claims() {
    let convergence = zsui_ui_protocol_convergence();
    assert_eq!(
        convergence
            .iter()
            .map(|status| status.ingress_name)
            .collect::<Vec<_>>(),
        vec![
            "main_window",
            "menu",
            "settings_page",
            "dialog",
            "dynamic_controls"
        ]
    );
    assert!(convergence.iter().all(|status| status.protocolized));
    assert!(convergence
        .iter()
        .all(|status| status.platform_host_count == 3));
    assert!(convergence
        .iter()
        .all(|status| status.source_gap_platform_count == 0));
    assert!(convergence
        .iter()
        .all(|status| status.target_smoke_required_platform_count == 3));
    assert!(convergence
        .iter()
        .all(|status| status.system_complete_platform_count == 0));

    let dynamic = zsui_ui_protocol_convergence_for_ingress("dynamic_controls").unwrap();
    assert_eq!(dynamic.static_builder_count, 0);
    assert!(dynamic.dynamic_builder_count >= 1);
    assert_eq!(dynamic.source_gap_platform_count, 0);
    assert_eq!(dynamic.next_platform_name, Some("windows"));
    assert_eq!(dynamic.next_missing_protocol_builder_name, None);
    assert_eq!(
        dynamic.next_missing_requirement,
        Some("target native smoke verification")
    );
    assert!(dynamic
        .preferred_app_core_edit_modules
        .contains(&"src/app_core/native_component_protocol.rs"));

    let settings = zsui_ui_protocol_convergence_for_ingress("settings_page").unwrap();
    assert_eq!(settings.category_name, "settings_page");
    assert!(settings.total_builder_count >= 4);
    assert!(settings
        .platform_adapter_touchpoints
        .contains(&"src/linux_native_host.rs"));

    let manifest = zsui_framework_manifest();
    assert_eq!(manifest.ui_protocol_convergence, convergence);
    let context = zsui_agent_context();
    assert_eq!(context.ui_protocol_convergence, convergence);
}

#[test]
fn ui_ingress_reverse_index_routes_builders_and_actions_to_one_surface() {
    let settings = zsui_ui_ingress_for_protocol_builder("native_host_settings_group_button_specs")
        .expect("settings group builder should map to settings page ingress");
    assert_eq!(settings.ingress_name, "settings_page");
    assert!(settings
        .preferred_app_core_edit_modules
        .contains(&"src/app_core/settings_protocol.rs"));

    let row_menu =
        zsui_ui_ingress_for_action_family("Row").expect("row actions should map to menu ingress");
    assert_eq!(row_menu.ingress_name, "menu");
    assert!(row_menu
        .protocol_builder_names
        .contains(&"native_host_row_action_button_specs"));

    #[cfg(feature = "vv-paste")]
    {
        let vv_dynamic = zsui_ui_ingress_for_protocol_builder("native_host_vv_select_specs")
            .expect("VV dynamic builder should map to dynamic controls ingress");
        assert_eq!(vv_dynamic.ingress_name, "dynamic_controls");
        assert!(vv_dynamic
            .preferred_app_core_edit_modules
            .contains(&"src/app_core/render_protocol.rs"));
    }

    assert!(zsui_ui_ingress_for_protocol_builder("platform_local_button").is_none());
    assert!(zsui_ui_ingress_for_action_family("PlatformPrivateAction").is_none());
}

#[test]
fn native_feature_matrix_declares_ui_ingress_requirements_for_every_feature() {
    let mut feature_names = zsui_native_feature_parity_statuses()
        .into_iter()
        .map(|status| status.feature_name)
        .collect::<Vec<_>>();
    feature_names.sort_unstable();
    feature_names.dedup();

    let requirements = zsui_native_feature_ui_ingress_requirements();
    let mut requirement_names = requirements
        .iter()
        .map(|requirement| requirement.feature_name)
        .collect::<Vec<_>>();
    requirement_names.sort_unstable();

    assert_eq!(requirement_names, feature_names);

    let ingress_names = zsui_ui_ingress_classifications()
        .iter()
        .map(|entry| entry.ingress_name)
        .collect::<Vec<_>>();

    for requirement in &requirements {
        assert!(!requirement.ingress_names.is_empty());
        assert!(requirement
            .ingress_names
            .iter()
            .all(|name| ingress_names.contains(name)));
        assert!(requirement
            .preferred_app_core_edit_modules
            .iter()
            .all(|module| module.starts_with("src/app_core/")));
        assert!(!requirement.native_component_family_names.is_empty());
        assert!(!requirement.typed_component_spec_names.is_empty());
        assert!(
            !requirement.protocol_builder_names.is_empty()
                || !requirement.dynamic_protocol_builder_names.is_empty()
        );
    }

    let edit = requirements
        .iter()
        .find(|requirement| requirement.feature_name == "right_click_edit_save")
        .unwrap();
    assert_eq!(edit.ingress_names, vec!["menu", "dialog"]);
    assert!(edit.action_family_names.contains(&"Row"));
    assert!(edit.action_family_names.contains(&"Dialog"));
    assert!(edit.action_family_names.contains(&"EditText"));
    assert_eq!(
        edit.native_component_family_names,
        vec!["row_action_button", "edit_text_button"]
    );
    assert!(edit
        .typed_component_spec_names
        .contains(&"NativeButtonSpec<NativeHostRowAction>"));
    assert!(edit
        .typed_component_spec_names
        .contains(&"NativeButtonSpec<NativeHostEditTextAction>"));

    let groups = requirements
        .iter()
        .find(|requirement| requirement.feature_name == "group_create_rename_delete_reorder_filter")
        .unwrap();
    assert_eq!(groups.ingress_names, vec!["menu", "settings_page"]);
    assert!(groups
        .preferred_app_core_edit_modules
        .contains(&"src/app_core/settings_protocol.rs"));

    let vv = requirements
        .iter()
        .find(|requirement| requirement.feature_name == "vv_popup_select")
        .unwrap();
    assert_eq!(vv.ingress_names, vec!["main_window", "dynamic_controls"]);
    assert!(vv
        .native_component_family_names
        .contains(&"main_tool_button"));
    #[cfg(feature = "vv-paste")]
    {
        assert!(vv
            .native_component_family_names
            .contains(&"vv_select_instance"));
        assert!(vv
            .dynamic_protocol_builder_names
            .contains(&"native_host_vv_select_specs"));
    }
    #[cfg(not(feature = "vv-paste"))]
    {
        assert!(!vv
            .native_component_family_names
            .contains(&"vv_select_instance"));
        assert!(!vv
            .dynamic_protocol_builder_names
            .contains(&"native_host_vv_select_specs"));
    }

    let lan = requirements
        .iter()
        .find(|requirement| requirement.feature_name == "sync_lan")
        .unwrap();
    assert_eq!(lan.ingress_names, vec!["settings_page"]);
    assert_eq!(
        lan.native_component_family_names,
        vec!["settings_toggle", "settings_dropdown"]
    );
    assert!(lan
        .typed_component_spec_names
        .contains(&"NativeToggleSpec<NativeHostSettingsControlAction>"));
    assert!(lan
        .typed_component_spec_names
        .contains(&"NativeDropdownSpec<NativeHostSettingsControlAction>"));

    let row_presentation = requirements
        .iter()
        .find(|requirement| requirement.feature_name == "clip_row_presentation_plan")
        .unwrap();
    assert_eq!(
        row_presentation.ingress_names,
        vec!["main_window", "dynamic_controls"]
    );
    assert_eq!(
        row_presentation.native_component_family_names,
        vec!["clip_row_instance"]
    );
    assert!(row_presentation
        .dynamic_protocol_builder_names
        .contains(&"native_host_clip_row_specs"));

    let window_system = requirements
        .iter()
        .find(|requirement| requirement.feature_name == "window_system_integration")
        .unwrap();
    assert_eq!(window_system.ingress_names, vec!["main_window"]);
    assert!(window_system
        .preferred_app_core_edit_modules
        .contains(&"src/app_core/native_component_protocol.rs"));

    let startup_autostart = requirements
        .iter()
        .find(|requirement| requirement.feature_name == "startup_autostart")
        .unwrap();
    assert_eq!(startup_autostart.ingress_names, vec!["settings_page"]);
    assert!(startup_autostart
        .protocol_builder_names
        .contains(&"native_host_settings_toggle_specs"));

    let manifest = zsui_framework_manifest();
    assert_eq!(
        manifest.native_feature_ui_ingress_requirements,
        requirements
    );
    let context = zsui_agent_context();
    assert_eq!(context.native_feature_ui_ingress_requirements, requirements);
}

#[test]
fn user_feature_platform_statuses_cover_named_clipboard_features() {
    let statuses = zsui_user_feature_platform_statuses();
    let expected_user_features = [
        "right_click_edit",
        "right_click_copy",
        "right_click_paste",
        "right_click_delete",
        "right_click_pin",
        "grouping",
        "search",
        "vv_mode",
        "settings_pages",
        "window_system",
        "sync_webdav",
        "sync_lan",
        "tray_status_menu",
        "popups_dialogs",
        "text_payload",
        "image_payload",
        "file_payload",
    ];

    assert_eq!(
        statuses.len(),
        expected_user_features.len() * SUPPORTED_NATIVE_UI_PLATFORMS.len()
    );

    for platform in SUPPORTED_NATIVE_UI_PLATFORMS {
        for feature_name in expected_user_features {
            let row = statuses
                .iter()
                .find(|status| {
                    status.platform == platform && status.user_feature_name == feature_name
                })
                .unwrap_or_else(|| {
                    panic!(
                        "missing user feature status for {} on {}",
                        feature_name,
                        platform.platform_name()
                    )
                });
            assert_eq!(row.platform_name, platform.platform_name());
            assert!(!row.display_name.is_empty());
            assert!(!row.required_native_feature_names.is_empty());
            assert!(!row.ui_ingress_names.is_empty());
            assert!(row
                .preferred_app_core_edit_modules
                .iter()
                .all(|module| module.starts_with("src/app_core/")));
            assert_eq!(row.target_smoke_verified, row.system_complete);
        }
    }

    let edit = statuses
        .iter()
        .find(|status| {
            status.platform == NativeUiPlatform::Macos
                && status.user_feature_name == "right_click_edit"
        })
        .unwrap();
    assert_eq!(
        edit.required_native_feature_names,
        vec!["right_click_edit_save", "dialog_input_confirm_edit"]
    );
    assert_eq!(edit.ui_ingress_names, vec!["menu", "dialog"]);
    assert_eq!(
        edit.support_status_name,
        ZsuiNativeFeatureSupportStatus::CodeLevelReadyPendingTargetSmoke.status_name()
    );
    assert!(edit.code_level_ready);
    assert_eq!(
        edit.host_maturity_name,
        ZsuiUserFeatureHostMaturity::HostUsablePendingTargetSmoke.maturity_name()
    );
    assert_eq!(edit.host_maturity_percent, 80);
    assert!(edit.host_usable);
    assert!(edit.target_smoke_required);
    assert!(!edit.system_complete);

    let linux_webdav = statuses
        .iter()
        .find(|status| {
            status.platform == NativeUiPlatform::Linux && status.user_feature_name == "sync_webdav"
        })
        .unwrap();
    assert_eq!(
        linux_webdav.required_native_feature_names,
        vec!["sync_webdav"]
    );
    assert_eq!(
        linux_webdav.support_status_name,
        ZsuiNativeFeatureSupportStatus::CodeLevelReadyPendingTargetSmoke.status_name()
    );
    assert!(linux_webdav.code_level_ready);
    assert!(linux_webdav.host_usable);
    assert!(linux_webdav.target_smoke_required);

    let linux_lan = statuses
        .iter()
        .find(|status| {
            status.platform == NativeUiPlatform::Linux && status.user_feature_name == "sync_lan"
        })
        .unwrap();
    assert_eq!(linux_lan.required_native_feature_names, vec!["sync_lan"]);
    assert_eq!(
        linux_lan.support_status_name,
        ZsuiNativeFeatureSupportStatus::CodeLevelReadyPendingTargetSmoke.status_name()
    );
    assert!(linux_lan.code_level_ready);
    assert_eq!(
        linux_lan.host_maturity_name,
        ZsuiUserFeatureHostMaturity::HostUsablePendingTargetSmoke.maturity_name()
    );
    assert!(linux_lan.host_usable);
    assert!(linux_lan.target_smoke_required);

    let vv = statuses
        .iter()
        .find(|status| {
            status.platform == NativeUiPlatform::Windows && status.user_feature_name == "vv_mode"
        })
        .unwrap();
    assert_eq!(vv.ui_ingress_names, vec!["main_window", "dynamic_controls"]);

    let search = statuses
        .iter()
        .find(|status| {
            status.platform == NativeUiPlatform::Windows && status.user_feature_name == "search"
        })
        .unwrap();
    assert!(search
        .required_native_feature_names
        .contains(&"clip_row_presentation_plan"));

    for user_feature_name in ["text_payload", "image_payload", "file_payload"] {
        let payload = statuses
            .iter()
            .find(|status| {
                status.platform == NativeUiPlatform::Windows
                    && status.user_feature_name == user_feature_name
            })
            .unwrap_or_else(|| panic!("missing payload user feature {user_feature_name}"));
        assert!(payload
            .required_native_feature_names
            .contains(&"clip_row_presentation_plan"));
    }

    let settings = statuses
        .iter()
        .find(|status| {
            status.platform == NativeUiPlatform::Linux
                && status.user_feature_name == "settings_pages"
        })
        .unwrap();
    assert!(settings
        .preferred_app_core_edit_modules
        .contains(&"src/app_core/settings_protocol.rs"));
    assert!(settings
        .required_native_feature_names
        .contains(&"startup_autostart"));

    let linux_window_system = statuses
        .iter()
        .find(|status| {
            status.platform == NativeUiPlatform::Linux
                && status.user_feature_name == "window_system"
        })
        .unwrap();
    assert_eq!(
        linux_window_system.required_native_feature_names,
        vec!["main_window_db_rows", "window_system_integration"]
    );
    assert_eq!(
        linux_window_system.support_status_name,
        ZsuiNativeFeatureSupportStatus::CodeLevelReadyPendingTargetSmoke.status_name()
    );
    assert_eq!(
        linux_window_system.host_maturity_name,
        ZsuiUserFeatureHostMaturity::HostUsablePendingTargetSmoke.maturity_name()
    );
    assert_eq!(linux_window_system.host_maturity_percent, 80);
    assert!(linux_window_system.code_level_ready);
    assert!(linux_window_system.host_usable);
    assert!(linux_window_system
        .missing_system_requirements
        .iter()
        .any(|requirement| requirement.contains("target GTK X11 command backend smoke")));

    let manifest = zsui_framework_manifest();
    assert_eq!(manifest.user_feature_platform_statuses, statuses);
    let context = zsui_agent_context();
    assert_eq!(context.user_feature_platform_statuses, statuses);
}

#[test]
fn user_feature_status_queries_answer_feature_progress_directly() {
    let mac_edit = zsui_user_feature_status_for(NativeUiPlatform::Macos, "right_click_edit")
        .expect("macOS right-click edit status");
    assert_eq!(mac_edit.display_name, "右键编辑");
    assert_eq!(
        mac_edit.required_native_feature_names,
        vec!["right_click_edit_save", "dialog_input_confirm_edit"]
    );
    assert_eq!(mac_edit.ui_ingress_names, vec!["menu", "dialog"]);
    assert!(mac_edit.code_level_ready);
    assert!(mac_edit.target_smoke_required);
    assert!(!mac_edit.system_complete);

    let linux_grouping = zsui_user_feature_status_for(NativeUiPlatform::Linux, "grouping")
        .expect("Linux grouping status");
    assert_eq!(linux_grouping.display_name, "分组功能");
    assert_eq!(
        linux_grouping.required_native_feature_names,
        vec![
            "right_click_group_assign_remove",
            "group_create_rename_delete_reorder_filter"
        ]
    );
    assert!(linux_grouping.ui_ingress_names.contains(&"menu"));
    assert!(linux_grouping.ui_ingress_names.contains(&"settings_page"));

    let windows_vv = zsui_user_feature_status_for(NativeUiPlatform::Windows, "vv_mode")
        .expect("Windows VV status");
    assert_eq!(windows_vv.display_name, "VV 模式");
    assert!(windows_vv
        .required_native_feature_names
        .contains(&"window_paste_target_identity"));
    assert_eq!(
        windows_vv.support_status_name,
        ZsuiNativeFeatureSupportStatus::CodeLevelReadyPendingTargetSmoke.status_name()
    );

    let linux_webdav = zsui_user_feature_status_for(NativeUiPlatform::Linux, "sync_webdav")
        .expect("Linux WebDAV sync status");
    assert_eq!(linux_webdav.display_name, "WebDAV 同步");
    assert_eq!(
        linux_webdav.required_native_feature_names,
        vec!["sync_webdav"]
    );
    assert!(linux_webdav.code_level_ready);
    assert_eq!(linux_webdav.ui_ingress_names, vec!["settings_page"]);

    let linux_lan = zsui_user_feature_status_for(NativeUiPlatform::Linux, "sync_lan")
        .expect("Linux LAN sync status");
    assert_eq!(linux_lan.display_name, "LAN 同步");
    assert_eq!(
        linux_lan.support_status_name,
        ZsuiNativeFeatureSupportStatus::CodeLevelReadyPendingTargetSmoke.status_name()
    );
    assert!(linux_lan.code_level_ready);
    assert!(linux_lan.target_smoke_required);

    assert!(zsui_user_feature_status_for(NativeUiPlatform::Macos, "unknown_feature").is_none());
}

#[test]
fn user_feature_completion_summaries_report_platform_progress() {
    let summaries = zsui_user_feature_completion_summaries();
    assert_eq!(summaries.len(), SUPPORTED_NATIVE_UI_PLATFORMS.len());

    let windows = summaries
        .iter()
        .find(|summary| summary.platform == NativeUiPlatform::Windows)
        .unwrap();
    assert_eq!(windows.platform_name, "windows");
    assert_eq!(windows.total_user_feature_count, 17);
    assert_eq!(windows.code_level_ready_count, 17);
    assert_eq!(windows.host_usable_count, 17);
    assert_eq!(windows.planned_not_implemented_count, 0);
    assert_eq!(windows.target_smoke_required_count, 17);
    assert_eq!(windows.system_complete_count, 0);
    assert_eq!(windows.code_level_ready_percent, 100);
    assert_eq!(windows.host_usable_percent, 100);
    assert_eq!(windows.system_complete_percent, 0);
    assert_eq!(windows.next_user_feature_name, Some("right_click_edit"));
    assert!(windows.next_missing_requirement.is_some());

    let macos = summaries
        .iter()
        .find(|summary| summary.platform == NativeUiPlatform::Macos)
        .expect("missing macOS summary");
    assert_eq!(macos.total_user_feature_count, 17);
    assert_eq!(macos.code_level_ready_count, 17);
    assert_eq!(macos.host_usable_count, 17);
    assert_eq!(macos.planned_not_implemented_count, 0);
    assert_eq!(macos.target_smoke_required_count, 17);
    assert_eq!(macos.system_complete_count, 0);
    assert_eq!(macos.code_level_ready_percent, 100);
    assert_eq!(macos.host_usable_percent, 100);
    assert_eq!(macos.system_complete_percent, 0);
    assert_eq!(macos.next_user_feature_name, Some("right_click_edit"));
    assert!(macos.next_missing_requirement.is_some());

    let linux = summaries
        .iter()
        .find(|summary| summary.platform == NativeUiPlatform::Linux)
        .expect("missing Linux summary");
    assert_eq!(linux.total_user_feature_count, 17);
    assert_eq!(linux.code_level_ready_count, 17);
    assert_eq!(linux.host_usable_count, 17);
    assert_eq!(linux.planned_not_implemented_count, 0);
    assert_eq!(linux.target_smoke_required_count, 17);
    assert_eq!(linux.system_complete_count, 0);
    assert_eq!(linux.code_level_ready_percent, 100);
    assert_eq!(linux.host_usable_percent, 100);
    assert_eq!(linux.system_complete_percent, 0);
    assert_eq!(linux.next_user_feature_name, Some("right_click_edit"));
    assert!(linux.next_missing_requirement.is_some());

    let manifest = zsui_framework_manifest();
    assert_eq!(manifest.user_feature_completion_summaries, summaries);
    let context = zsui_agent_context();
    assert_eq!(context.user_feature_completion_summaries, summaries);
}

#[test]
fn user_feature_completion_summary_query_answers_platform_progress_directly() {
    let windows = zsui_user_feature_completion_summary_for(NativeUiPlatform::Windows)
        .expect("Windows user feature summary");
    assert_eq!(windows.total_user_feature_count, 17);
    assert_eq!(windows.code_level_ready_count, 17);
    assert_eq!(windows.code_level_ready_percent, 100);
    assert_eq!(windows.host_usable_count, 17);
    assert_eq!(windows.host_usable_percent, 100);
    assert_eq!(windows.system_complete_percent, 0);

    let linux = zsui_user_feature_completion_summary_for(NativeUiPlatform::Linux)
        .expect("Linux user feature summary");
    assert_eq!(linux.total_user_feature_count, 17);
    assert_eq!(linux.code_level_ready_count, 17);
    assert_eq!(linux.host_usable_count, 17);
    assert_eq!(linux.planned_not_implemented_count, 0);
    assert_eq!(linux.next_user_feature_name, Some("right_click_edit"));
    assert!(linux.next_missing_requirement.is_some());
}

#[test]
fn user_feature_cross_platform_summaries_answer_feature_progress_directly() {
    let summaries = zsui_user_feature_cross_platform_summaries();
    assert_eq!(summaries.len(), 17);

    let grouping = summaries
        .iter()
        .find(|summary| summary.user_feature_name == "grouping")
        .expect("grouping cross-platform summary");
    assert_eq!(grouping.display_name, "分组功能");
    assert_eq!(grouping.total_platform_count, 3);
    assert_eq!(grouping.code_level_ready_count, 3);
    assert_eq!(grouping.host_usable_count, 3);
    assert_eq!(grouping.planned_not_implemented_count, 0);
    assert_eq!(grouping.target_smoke_required_count, 3);
    assert_eq!(grouping.system_complete_count, 0);
    assert_eq!(
        grouping.code_level_ready_platform_names,
        vec!["windows", "macos", "linux"]
    );
    assert_eq!(
        grouping.host_usable_platform_names,
        vec!["windows", "macos", "linux"]
    );
    assert_eq!(grouping.code_level_ready_percent, 100);
    assert_eq!(grouping.host_usable_percent, 100);
    assert_eq!(grouping.system_complete_percent, 0);
    assert_eq!(grouping.next_platform_name, Some("windows"));
    assert!(grouping.next_missing_requirement.is_some());

    let webdav = zsui_user_feature_cross_platform_summary_for("sync_webdav")
        .expect("WebDAV sync cross-platform summary");
    assert_eq!(webdav.display_name, "WebDAV 同步");
    assert_eq!(
        webdav.code_level_ready_platform_names,
        vec!["windows", "macos", "linux"]
    );
    assert!(webdav.planned_platform_names.is_empty());
    assert_eq!(webdav.code_level_ready_count, 3);
    assert_eq!(webdav.host_usable_count, 3);
    assert_eq!(webdav.planned_not_implemented_count, 0);
    assert_eq!(webdav.code_level_ready_percent, 100);
    assert_eq!(webdav.host_usable_percent, 100);
    assert_eq!(webdav.next_platform_name, Some("windows"));

    let lan = zsui_user_feature_cross_platform_summary_for("sync_lan")
        .expect("LAN sync cross-platform summary");
    assert_eq!(lan.display_name, "LAN 同步");
    assert_eq!(
        lan.code_level_ready_platform_names,
        vec!["windows", "macos", "linux"]
    );
    assert!(lan.planned_platform_names.is_empty());
    assert_eq!(lan.code_level_ready_count, 3);
    assert_eq!(lan.host_usable_count, 3);
    assert_eq!(lan.planned_not_implemented_count, 0);
    assert_eq!(lan.code_level_ready_percent, 100);
    assert_eq!(lan.host_usable_percent, 100);
    assert_eq!(lan.next_platform_name, Some("windows"));

    let window_system = zsui_user_feature_cross_platform_summary_for("window_system")
        .expect("window system cross-platform summary");
    assert_eq!(window_system.display_name, "窗口系统");
    assert_eq!(window_system.total_platform_count, 3);
    assert_eq!(window_system.code_level_ready_count, 3);
    assert_eq!(window_system.host_usable_count, 3);
    assert_eq!(
        window_system.code_level_ready_platform_names,
        vec!["windows", "macos", "linux"]
    );
    assert_eq!(
        window_system.host_usable_platform_names,
        vec!["windows", "macos", "linux"]
    );
    assert_eq!(window_system.code_level_ready_percent, 100);
    assert_eq!(window_system.host_usable_percent, 100);
    assert_eq!(window_system.next_platform_name, Some("windows"));

    let vv =
        zsui_user_feature_cross_platform_summary_for("vv_mode").expect("VV cross-platform summary");
    assert_eq!(
        vv.target_smoke_required_platform_names,
        vec!["windows", "macos", "linux"]
    );
    assert!(vv.system_complete_platform_names.is_empty());

    assert!(zsui_user_feature_cross_platform_summary_for("unknown_feature").is_none());

    let manifest = zsui_framework_manifest();
    assert_eq!(manifest.user_feature_cross_platform_summaries, summaries);
    let context = zsui_agent_context();
    assert_eq!(context.user_feature_cross_platform_summaries, summaries);
}

#[test]
fn user_feature_release_progress_answers_overall_project_progress_directly() {
    let progress = zsui_user_feature_release_progress();

    assert_eq!(progress.total_platform_feature_slots, 51);
    assert_eq!(progress.code_level_ready_slots, 51);
    assert_eq!(progress.host_usable_slots, 51);
    assert_eq!(progress.planned_not_implemented_slots, 0);
    assert_eq!(progress.target_smoke_required_slots, 51);
    assert_eq!(progress.system_complete_slots, 0);
    assert_eq!(progress.non_windows_host_slots, 34);
    assert_eq!(progress.non_windows_host_code_level_ready_slots, 34);
    assert_eq!(progress.non_windows_host_usable_slots, 34);
    assert_eq!(progress.non_windows_host_code_gap_slots, 0);
    assert_eq!(progress.non_windows_host_system_complete_slots, 0);
    assert_eq!(progress.code_level_ready_percent, 100);
    assert_eq!(progress.host_usable_percent, 100);
    assert_eq!(progress.non_windows_host_usable_percent, 100);
    assert_eq!(progress.system_complete_percent, 0);
    assert_eq!(progress.next_platform_name, Some("macos"));
    assert_eq!(progress.next_user_feature_name, Some("right_click_edit"));
    assert_eq!(progress.next_display_name, Some("右键编辑"));
    assert_eq!(progress.next_ui_ingress_names, vec!["menu", "dialog"]);
    assert_eq!(
        progress.next_native_component_family_names,
        vec!["row_action_button", "edit_text_button", "dialog_button"]
    );
    assert!(progress
        .next_typed_component_spec_names
        .contains(&"NativeButtonSpec<NativeHostRowAction>"));
    assert!(progress
        .next_typed_component_spec_names
        .contains(&"NativeButtonSpec<NativeHostEditTextAction>"));
    assert!(progress
        .next_preferred_app_core_edit_modules
        .contains(&"src/app_core/native_host_actions.rs"));
    assert!(progress
        .next_preferred_app_core_edit_modules
        .contains(&"src/app_core/native_component_protocol.rs"));
    assert!(progress
        .next_preferred_app_core_edit_modules
        .contains(&"src/app_core/host_protocol.rs"));
    assert_eq!(
        progress.next_platform_host_module_paths,
        vec!["src/macos_native_host.rs", "src/macos_app.rs"]
    );
    assert!(progress.next_missing_requirement.is_some());
    assert_eq!(progress.next_host_platform_name, Some("macos"));
    assert_eq!(
        progress.next_host_user_feature_name,
        Some("right_click_edit")
    );
    assert_eq!(progress.next_host_display_name, Some("右键编辑"));
    assert_eq!(progress.next_host_ui_ingress_names, vec!["menu", "dialog"]);
    assert_eq!(
        progress.next_host_native_component_family_names,
        vec!["row_action_button", "edit_text_button", "dialog_button"]
    );
    assert!(progress
        .next_host_typed_component_spec_names
        .contains(&"NativeButtonSpec<NativeHostRowAction>"));
    assert!(progress
        .next_host_typed_component_spec_names
        .contains(&"NativeButtonSpec<NativeHostEditTextAction>"));
    assert!(progress
        .next_host_preferred_app_core_edit_modules
        .contains(&"src/app_core/native_host_actions.rs"));
    assert!(progress
        .next_host_preferred_app_core_edit_modules
        .contains(&"src/app_core/native_component_protocol.rs"));
    assert!(progress
        .next_host_preferred_app_core_edit_modules
        .contains(&"src/app_core/host_protocol.rs"));
    assert_eq!(
        progress.next_host_module_paths,
        vec!["src/macos_native_host.rs", "src/macos_app.rs"]
    );
    assert!(progress.next_host_missing_requirement.is_some());
    assert_eq!(progress.next_host_code_gap_platform_name, None);
    assert_eq!(progress.next_host_code_gap_user_feature_name, None);
    assert_eq!(progress.next_host_code_gap_display_name, None);
    assert!(progress.next_host_code_gap_ui_ingress_names.is_empty());
    assert!(progress
        .next_host_code_gap_native_component_family_names
        .is_empty());
    assert!(progress
        .next_host_code_gap_typed_component_spec_names
        .is_empty());
    assert!(progress
        .next_host_code_gap_preferred_app_core_edit_modules
        .is_empty());
    assert!(progress.next_host_code_gap_module_paths.is_empty());
    assert_eq!(progress.next_host_code_gap_missing_requirement, None);

    let manifest = zsui_framework_manifest();
    assert_eq!(manifest.user_feature_release_progress, progress);
    let context = zsui_agent_context();
    assert_eq!(context.user_feature_release_progress, progress);
}

#[test]
fn user_feature_progress_report_combines_cross_platform_summary_and_platform_rows() {
    let reports = zsui_user_feature_progress_reports();
    assert_eq!(reports.len(), 17);

    let grouping =
        zsui_user_feature_progress_report_for("grouping").expect("grouping progress report");
    assert_eq!(grouping.user_feature_name, "grouping");
    assert_eq!(grouping.display_name, "分组功能");
    assert_eq!(grouping.cross_platform_summary.code_level_ready_count, 3);
    assert_eq!(
        grouping
            .platform_statuses
            .iter()
            .map(|status| status.platform_name)
            .collect::<Vec<_>>(),
        vec!["windows", "macos", "linux"]
    );
    assert!(grouping
        .platform_statuses
        .iter()
        .all(|status| status.user_feature_name == "grouping"
            && status
                .required_native_feature_names
                .contains(&"group_create_rename_delete_reorder_filter")));

    let webdav =
        zsui_user_feature_progress_report_for("sync_webdav").expect("WebDAV sync progress report");
    assert_eq!(webdav.cross_platform_summary.code_level_ready_count, 3);
    assert!(webdav
        .cross_platform_summary
        .planned_platform_names
        .is_empty());

    let lan = zsui_user_feature_progress_report_for("sync_lan").expect("LAN sync progress report");
    assert_eq!(lan.cross_platform_summary.code_level_ready_count, 3);
    assert!(lan.cross_platform_summary.planned_platform_names.is_empty());
    let linux_lan = lan
        .platform_statuses
        .iter()
        .find(|status| status.platform == NativeUiPlatform::Linux)
        .expect("Linux LAN sync row");
    assert_eq!(
        linux_lan.support_status_name,
        ZsuiNativeFeatureSupportStatus::CodeLevelReadyPendingTargetSmoke.status_name()
    );

    let window_system = zsui_user_feature_progress_report_for("window_system")
        .expect("window system progress report");
    assert_eq!(window_system.display_name, "窗口系统");
    assert_eq!(
        window_system.cross_platform_summary.code_level_ready_count,
        3
    );
    assert_eq!(window_system.cross_platform_summary.host_usable_count, 3);
    let linux_window_system = window_system
        .platform_statuses
        .iter()
        .find(|status| status.platform == NativeUiPlatform::Linux)
        .expect("Linux window system row");
    assert_eq!(
        linux_window_system.host_maturity_name,
        ZsuiUserFeatureHostMaturity::HostUsablePendingTargetSmoke.maturity_name()
    );

    assert!(zsui_user_feature_progress_report_for("unknown_feature").is_none());
}

#[test]
fn user_feature_work_items_point_ai_to_app_core_and_platform_hosts() {
    let work_items = zsui_user_feature_work_items();
    assert_eq!(work_items.len(), 51);
    assert!(work_items.iter().all(|item| {
        !item.ui_ingress_names.is_empty()
            && !item.native_component_family_names.is_empty()
            && !item.typed_component_spec_names.is_empty()
            && !item.preferred_app_core_edit_modules.is_empty()
            && !item.platform_host_module_paths.is_empty()
            && !item.required_native_feature_names.is_empty()
    }));
    assert!(work_items
        .iter()
        .all(|item| item.platform_host_module_paths.iter().all(|path| {
            match item.platform {
                NativeUiPlatform::Windows => {
                    path.starts_with("src/app")
                        || path.starts_with("src/settings")
                        || path.starts_with("src/platform")
                        || path.starts_with("src/windows")
                }
                NativeUiPlatform::Macos => path.starts_with("src/macos_"),
                NativeUiPlatform::Linux => path.starts_with("src/linux_"),
            }
        })));

    let linux_lan = work_items
        .iter()
        .find(|item| {
            item.platform == NativeUiPlatform::Linux && item.user_feature_name == "sync_lan"
        })
        .expect("linux LAN sync user feature work item");
    assert_eq!(linux_lan.display_name, "LAN 同步");
    assert_eq!(
        linux_lan.support_status_name,
        ZsuiNativeFeatureSupportStatus::CodeLevelReadyPendingTargetSmoke.status_name()
    );
    assert_eq!(linux_lan.ui_ingress_names, vec!["settings_page"]);
    assert_eq!(
        linux_lan.native_component_family_names,
        vec!["settings_toggle", "settings_dropdown"]
    );
    assert_eq!(
        linux_lan.typed_component_spec_names,
        vec![
            "NativeToggleSpec<NativeHostSettingsControlAction>",
            "NativeDropdownSpec<NativeHostSettingsControlAction>"
        ]
    );
    assert!(linux_lan
        .preferred_app_core_edit_modules
        .contains(&"src/app_core/settings_protocol.rs"));
    assert_eq!(
        linux_lan.platform_host_module_paths,
        vec!["src/linux_native_host.rs", "src/linux_app.rs"]
    );
    assert_eq!(linux_lan.required_native_feature_names, vec!["sync_lan"]);
    assert!(linux_lan
        .next_missing_requirement
        .unwrap()
        .contains("target settings sync toggle smoke"));

    let linux_window_system = work_items
        .iter()
        .find(|item| {
            item.platform == NativeUiPlatform::Linux && item.user_feature_name == "window_system"
        })
        .expect("linux window system work item");
    assert_eq!(linux_window_system.display_name, "窗口系统");
    assert_eq!(
        linux_window_system.support_status_name,
        ZsuiNativeFeatureSupportStatus::PartiallyCodeReadyPendingTargetSmoke.status_name()
    );
    assert_eq!(
        linux_window_system.ui_ingress_names,
        vec!["main_window", "dynamic_controls"]
    );
    assert_eq!(
        linux_window_system.platform_host_module_paths,
        vec!["src/linux_native_host.rs", "src/linux_app.rs"]
    );
    assert!(linux_window_system.next_missing_requirement.is_some());

    let windows_edit = work_items
        .iter()
        .find(|item| {
            item.platform == NativeUiPlatform::Windows
                && item.user_feature_name == "right_click_edit"
        })
        .expect("windows edit work item");
    assert_eq!(windows_edit.ui_ingress_names, vec!["menu", "dialog"]);
    assert_eq!(
        windows_edit.native_component_family_names,
        vec!["row_action_button", "edit_text_button", "dialog_button"]
    );
    assert!(windows_edit
        .typed_component_spec_names
        .contains(&"NativeButtonSpec<NativeHostRowAction>"));
    assert!(windows_edit
        .typed_component_spec_names
        .contains(&"NativeButtonSpec<NativeHostEditTextAction>"));
    assert!(windows_edit
        .platform_host_module_paths
        .contains(&"src/app/main_popup_menus.rs"));
    assert!(windows_edit
        .platform_host_module_paths
        .contains(&"src/windows_edit_text_dialog.rs"));

    let context = zsui_agent_context();
    assert_eq!(context.user_feature_work_items, work_items);
}

#[test]
fn native_target_smoke_work_items_turn_platform_progress_into_a_verification_queue() {
    let work_items = zsui_native_target_smoke_work_items();
    assert_eq!(work_items.len(), 51);
    assert!(work_items.iter().all(|item| {
        item.code_level_ready
            && item.target_smoke_required
            && !item.target_smoke_verified
            && !item.system_complete
            && !item.ui_ingress_names.is_empty()
            && !item.native_component_family_names.is_empty()
            && !item.typed_component_spec_names.is_empty()
            && !item.preferred_app_core_edit_modules.is_empty()
            && !item.platform_host_module_paths.is_empty()
            && !item.required_native_feature_names.is_empty()
            && !item.target_smoke_steps.is_empty()
            && item.next_missing_requirement == Some(item.target_smoke_name)
    }));

    assert_eq!(
        zsui_native_target_smoke_work_items_for_platform(NativeUiPlatform::Windows).len(),
        17
    );
    assert_eq!(
        zsui_native_target_smoke_work_items_for_platform(NativeUiPlatform::Macos).len(),
        17
    );
    assert_eq!(
        zsui_native_target_smoke_work_items_for_platform(NativeUiPlatform::Linux).len(),
        17
    );
    assert!(
        zsui_native_target_smoke_work_item_for(NativeUiPlatform::Linux, "window_system").is_some()
    );

    let linux_lan = zsui_native_target_smoke_work_item_for(NativeUiPlatform::Linux, "sync_lan")
        .expect("Linux LAN target-smoke item");
    assert_eq!(linux_lan.platform_name, "linux");
    assert_eq!(
        linux_lan.target_environment_name,
        "real Ubuntu GTK host smoke verification"
    );
    assert_eq!(
        linux_lan.support_status_name,
        ZsuiNativeFeatureSupportStatus::CodeLevelReadyPendingTargetSmoke.status_name()
    );
    assert_eq!(linux_lan.ui_ingress_names, vec!["settings_page"]);
    assert!(linux_lan
        .preferred_app_core_edit_modules
        .contains(&"src/app_core/settings_protocol.rs"));
    assert_eq!(
        linux_lan.platform_host_module_paths,
        vec!["src/linux_native_host.rs", "src/linux_app.rs"]
    );
    assert_eq!(linux_lan.required_native_feature_names, vec!["sync_lan"]);
    assert!(linux_lan
        .target_smoke_name
        .contains("target settings sync toggle smoke"));

    let macos_edit =
        zsui_native_target_smoke_work_item_for(NativeUiPlatform::Macos, "right_click_edit")
            .expect("macOS edit target-smoke item");
    assert_eq!(
        macos_edit.target_environment_name,
        "real macOS AppKit host smoke verification"
    );
    assert_eq!(macos_edit.ui_ingress_names, vec!["menu", "dialog"]);
    assert_eq!(
        macos_edit.native_component_family_names,
        vec!["row_action_button", "edit_text_button", "dialog_button"]
    );
    assert!(macos_edit
        .typed_component_spec_names
        .contains(&"NativeButtonSpec<NativeHostRowAction>"));
    assert!(macos_edit
        .typed_component_spec_names
        .contains(&"NativeButtonSpec<NativeHostEditTextAction>"));
    assert_eq!(
        macos_edit.platform_host_module_paths,
        vec!["src/macos_native_host.rs", "src/macos_app.rs"]
    );
    assert_eq!(
        macos_edit.target_smoke_steps,
        vec![
            "open native clipboard window with at least one text row",
            "select a row and invoke the shared row edit action",
            "verify the native edit surface uses a multiline text editor",
            "change text, save, and confirm the row refreshes with edited content",
            "change text again, close without saving, and verify the unsaved-change prompt"
        ]
    );

    assert_eq!(
        zsui_next_native_target_smoke_work_item_for_platform(NativeUiPlatform::Linux)
            .map(|item| item.user_feature_name),
        Some("right_click_edit")
    );

    let macos_batch = zsui_native_target_smoke_batch_for_platform(NativeUiPlatform::Macos);
    assert_eq!(macos_batch.len(), ZSUI_NATIVE_TARGET_SMOKE_BATCH_SIZE);
    assert!(macos_batch
        .iter()
        .all(|item| item.platform == NativeUiPlatform::Macos
            && item.target_environment_name == "real macOS AppKit host smoke verification"));
    assert_eq!(
        macos_batch
            .iter()
            .map(|item| item.user_feature_name)
            .collect::<Vec<_>>(),
        vec![
            "right_click_edit",
            "right_click_copy",
            "right_click_paste",
            "right_click_delete",
            "right_click_pin"
        ]
    );

    let linux_batch = zsui_native_target_smoke_batch_for_platform(NativeUiPlatform::Linux);
    assert_eq!(linux_batch.len(), ZSUI_NATIVE_TARGET_SMOKE_BATCH_SIZE);
    assert!(linux_batch
        .iter()
        .all(|item| item.platform == NativeUiPlatform::Linux
            && item.target_environment_name == "real Ubuntu GTK host smoke verification"));
    assert_eq!(
        linux_batch
            .iter()
            .map(|item| item.user_feature_name)
            .collect::<Vec<_>>(),
        vec![
            "right_click_edit",
            "right_click_copy",
            "right_click_paste",
            "right_click_delete",
            "right_click_pin"
        ]
    );
    assert_eq!(
        zsui_native_target_smoke_batch_for_macos_and_linux().len(),
        ZSUI_NATIVE_TARGET_SMOKE_BATCH_SIZE * 2
    );

    let manifest = zsui_framework_manifest();
    assert_eq!(manifest.native_target_smoke_work_items, work_items);
    let context = zsui_agent_context();
    assert_eq!(context.native_target_smoke_work_items, work_items);
}

#[test]
fn cargo_features_expose_optional_native_app_modules() {
    let cargo_toml = std::fs::read_to_string("Cargo.toml").expect("Cargo.toml source");

    assert!(cargo_toml.contains("[features]"));
    assert!(cargo_toml.contains(
        "default = [\"vv-paste\", \"cloud-sync\", \"lan-sync\", \"mail-merge\", \"ai-actions\", \"sticker\"]"
    ));
    for feature in [
        "vv-paste = []",
        "cloud-sync = []",
        "lan-sync = []",
        "mail-merge = []",
        "ai-actions = []",
        "sticker = []",
    ] {
        assert!(
            cargo_toml.contains(feature),
            "Cargo.toml must declare feature {feature}"
        );
    }

    let main_rs = std::fs::read_to_string("src/main.rs").expect("main.rs source");
    let prelude_rs = std::fs::read_to_string("src/app/prelude.rs").expect("prelude.rs source");
    let native_component_protocol_rs =
        std::fs::read_to_string("src/app_core/native_component_protocol.rs")
            .expect("native component protocol source");
    let native_host_actions_rs = std::fs::read_to_string("src/app_core/native_host_actions.rs")
        .expect("native host actions source");
    assert!(main_rs.contains("#[cfg(all(target_os = \"windows\", feature = \"mail-merge\"))]"));
    assert!(main_rs.contains("#[cfg(all(target_os = \"windows\", feature = \"sticker\"))]"));
    assert!(prelude_rs.contains("#[cfg(feature = \"mail-merge\")]"));
    assert!(prelude_rs.contains("#[cfg(feature = \"sticker\")]"));
    assert!(native_component_protocol_rs
        .contains("#[cfg(feature = \"vv-paste\")]\n    VvSelect(NativeHostVvSelectAction)"));
    assert!(native_component_protocol_rs
        .contains("#[cfg(feature = \"vv-paste\")]\n    pub(crate) fn vv_select_button"));
    assert!(native_component_protocol_rs.contains(
        "#[cfg(not(feature = \"vv-paste\"))]\nconst fn native_host_dynamic_action_family_names"
    ));
    assert!(native_host_actions_rs
        .contains("#[cfg(feature = \"vv-paste\")]\npub(crate) struct NativeHostVvSelectAction"));
    assert!(native_host_actions_rs.contains("#[cfg(feature = \"lan-sync\")]\n    ToggleLanSync"));
    assert!(
        native_host_actions_rs.contains("#[cfg(feature = \"cloud-sync\")]\n    ToggleCloudSync")
    );
    assert!(native_host_actions_rs.contains(
        "#[cfg(any(feature = \"cloud-sync\", feature = \"lan-sync\"))]\n    OpenSyncModeDropdown"
    ));
}

#[test]
fn native_ui_protocol_host_statuses_track_three_platform_source_coverage() {
    let statuses = zsui_native_ui_protocol_host_statuses();
    assert_eq!(statuses.len(), 15);
    assert_eq!(
        statuses
            .iter()
            .map(|status| (status.platform, status.surface_name))
            .collect::<Vec<_>>(),
        vec![
            (NativeUiPlatform::Windows, "main_window"),
            (NativeUiPlatform::Windows, "menu"),
            (NativeUiPlatform::Windows, "settings_page"),
            (NativeUiPlatform::Windows, "dialog"),
            (NativeUiPlatform::Windows, "dynamic_controls"),
            (NativeUiPlatform::Macos, "main_window"),
            (NativeUiPlatform::Macos, "menu"),
            (NativeUiPlatform::Macos, "settings_page"),
            (NativeUiPlatform::Macos, "dialog"),
            (NativeUiPlatform::Macos, "dynamic_controls"),
            (NativeUiPlatform::Linux, "main_window"),
            (NativeUiPlatform::Linux, "menu"),
            (NativeUiPlatform::Linux, "settings_page"),
            (NativeUiPlatform::Linux, "dialog"),
            (NativeUiPlatform::Linux, "dynamic_controls"),
        ]
    );

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for status in &statuses {
        assert!(status.target_smoke_required);
        assert!(!status.target_smoke_verified);
        assert!(!status.system_complete);
        if status.platform == NativeUiPlatform::Windows {
            assert!(status.source_guard_required);
            assert_eq!(
                status.host_module_path,
                "src/app.rs + src/app/* + src/tray.rs + src/settings_ui_host.rs + src/platform/dialog.rs + src/windows_edit_text_dialog.rs"
            );
            if status.surface_name == "main_window"
                || status.surface_name == "settings_page"
                || status.surface_name == "dialog"
                || status.surface_name == "dynamic_controls"
            {
                assert!(status.source_coverage_verified);
                assert!(status.missing_protocol_builder_names.is_empty());
            } else if status.surface_name == "menu" {
                assert!(status.source_coverage_verified);
                assert!(status.missing_protocol_builder_names.is_empty());
            } else {
                assert!(!status.source_coverage_verified);
                assert!(!status.missing_protocol_builder_names.is_empty());
                assert_eq!(
                    status.missing_protocol_builder_names,
                    status
                        .protocol_builder_names
                        .iter()
                        .chain(status.dynamic_protocol_builder_names.iter())
                        .copied()
                        .collect::<Vec<_>>()
                );
            }
            continue;
        }

        assert!(status.source_guard_required);
        assert!(status.source_coverage_verified);
        assert!(status.missing_protocol_builder_names.is_empty());
        let source = std::fs::read_to_string(root.join(status.host_module_path))
            .expect("native host source must be readable");
        for builder_name in status
            .protocol_builder_names
            .iter()
            .chain(status.dynamic_protocol_builder_names.iter())
        {
            assert!(
                source.contains(builder_name),
                "{} must consume {} for {}",
                status.host_module_path,
                builder_name,
                status.surface_name
            );
        }
    }

    let windows_menu = statuses
        .iter()
        .find(|status| {
            status.platform == NativeUiPlatform::Windows && status.surface_name == "menu"
        })
        .unwrap();
    assert!(windows_menu.source_coverage_verified);
    assert!(windows_menu.missing_protocol_builder_names.is_empty());
    let windows_tray_source =
        std::fs::read_to_string(root.join("src/tray.rs")).expect("tray source must be readable");
    assert!(windows_tray_source.contains("native_host_status_menu_item_specs"));
    assert!(windows_tray_source.contains("if spec.starts_section"));
    assert!(!windows_tray_source.contains("tray_action == MainTrayMenuAction::Exit"));
    assert!(windows_tray_source.contains("icon_name: spec.icon_name.to_string()"));
    assert!(!windows_tray_source.contains("NativeComponentAction::StatusMenu(action)"));
    let windows_popup_source = std::fs::read_to_string(root.join("src/app/main_popup_menus.rs"))
        .expect("windows popup menu source must be readable");
    assert!(windows_popup_source.contains("native_host_row_action_button_specs"));
    assert!(!windows_popup_source.contains("native_host_row_action_component_specs"));
    assert!(!windows_popup_source.contains("NativeComponentAction::Row(action)"));
    assert!(windows_popup_source.contains("native_host_full_row_popup_menu_entries_for_groups"));
    assert!(windows_popup_source.contains("native_host_group_filter_popup_menu_entries_for_groups"));
    assert!(windows_popup_source.contains("localize_group_filter_entry"));
    let windows_search_source = std::fs::read_to_string(root.join("src/app/main_search_host.rs"))
        .expect("windows search host source must be readable");
    assert!(windows_search_source.contains("native_host_search_input_specs"));
    assert!(!windows_search_source.contains("native_host_search_component_specs"));
    assert!(!windows_search_source.contains("NativeComponentAction::SearchControl"));
    assert!(windows_search_source.contains("NativeHostSearchControlAction::UpdateText"));
    let windows_events_source = std::fs::read_to_string(root.join("src/app/main_events.rs"))
        .expect("windows main event source must be readable");
    assert!(windows_events_source.contains("native_host_main_action_button_specs"));
    assert!(!windows_events_source.contains("native_host_main_action_component_specs"));
    assert!(!windows_events_source.contains("NativeComponentAction::HostUi(action)"));
    let windows_input_source = std::fs::read_to_string(root.join("src/app/main_input.rs"))
        .expect("windows main input source must be readable");
    assert!(windows_input_source.contains("native_host_main_tool_button_specs"));
    assert!(!windows_input_source.contains("native_host_main_tool_component_specs"));
    assert!(!windows_input_source.contains("NativeComponentAction::MainTool(tool)"));
    let windows_settings_source = std::fs::read_to_string(root.join("src/settings_ui_host.rs"))
        .expect("windows settings host source must be readable");
    assert!(windows_settings_source.contains("native_host_settings_action_button_specs"));
    assert!(!windows_settings_source.contains("native_host_settings_action_component_specs"));
    assert!(windows_settings_source.contains("native_host_settings_control_button_specs"));
    assert!(windows_settings_source.contains("native_host_settings_toggle_specs"));
    assert!(windows_settings_source.contains("native_host_settings_dropdown_specs"));
    assert!(!windows_settings_source.contains("native_host_settings_control_component_specs"));
    assert!(windows_settings_source.contains("native_host_settings_group_button_specs"));
    assert!(!windows_settings_source.contains("native_host_settings_group_component_specs"));
    assert!(windows_settings_source.contains("native_host_settings_platform_button_specs"));
    assert!(!windows_settings_source.contains("native_host_settings_platform_component_specs"));
    assert!(!windows_settings_source.contains("NativeComponentAction::Settings(action)"));
    assert!(!windows_settings_source.contains("NativeComponentAction::SettingsControl(action)"));
    assert!(
        !windows_settings_source.contains("NativeComponentAction::SettingsGroup(native_action)")
    );
    assert!(
        !windows_settings_source.contains("NativeComponentAction::SettingsPlatform(native_action)")
    );
    let windows_dialog_source = std::fs::read_to_string(root.join("src/platform/dialog.rs"))
        .expect("windows dialog host source must be readable");
    assert!(windows_dialog_source.contains("native_host_dialog_button_specs"));
    assert!(!windows_dialog_source.contains("native_host_dialog_component_specs"));
    assert!(!windows_dialog_source.contains("NativeComponentAction::Dialog(dialog_action)"));
    let windows_edit_source = std::fs::read_to_string(root.join("src/windows_edit_text_dialog.rs"))
        .expect("windows edit text dialog host source must be readable");
    assert!(windows_edit_source.contains("native_host_edit_text_button_specs"));
    assert!(!windows_edit_source.contains("native_host_edit_text_component_specs"));
    assert!(windows_edit_source.contains("spec.action == NativeHostEditTextAction::Save"));
    assert!(windows_edit_source.contains("spec.action == NativeHostEditTextAction::Cancel"));
    let windows_dynamic_source = std::fs::read_to_string(root.join("src/app/main_renderer.rs"))
        .expect("windows main renderer source must be readable");
    assert!(windows_dynamic_source.contains("native_host_clip_row_specs"));
    assert!(windows_dynamic_source.contains("Vec<crate::app_core::NativeClipRowSpec>"));
    assert!(windows_dynamic_source.contains("spec.action.has_item()"));
    assert!(windows_dynamic_source.contains("debug_assert_eq!(dynamic_row_item_id"));
    #[cfg(feature = "vv-paste")]
    {
        let windows_vv_source = std::fs::read_to_string(root.join("src/app/vv_popup.rs"))
            .expect("windows VV popup source must be readable");
        assert!(windows_vv_source.contains("native_host_vv_select_specs"));
        assert!(windows_vv_source.contains("spec.action.index"));
    }

    let linux_dynamic = statuses
        .iter()
        .find(|status| {
            status.platform == NativeUiPlatform::Linux && status.surface_name == "dynamic_controls"
        })
        .unwrap();
    assert_eq!(
        linux_dynamic.dynamic_protocol_builder_names,
        vec![
            "native_host_clip_row_specs",
            #[cfg(feature = "vv-paste")]
            "native_host_vv_select_specs"
        ]
    );
    assert_eq!(
        linux_dynamic.action_family_names,
        vec![
            "ClipRow",
            #[cfg(feature = "vv-paste")]
            "VvSelect"
        ]
    );
}

#[test]
fn native_ui_host_translation_work_items_point_ai_to_app_core_then_small_hosts() {
    let work_items = zsui_native_ui_host_translation_work_items();
    assert_eq!(work_items.len(), SUPPORTED_NATIVE_UI_PLATFORMS.len() * 5);
    assert!(work_items.iter().all(|item| {
        !item.preferred_app_core_edit_modules.is_empty()
            && !item.platform_host_module_paths.is_empty()
            && !item.action_family_names.is_empty()
            && item.next_missing_requirement == Some("target native smoke verification")
    }));

    let linux_settings =
        zsui_native_ui_host_translation_work_item_for(NativeUiPlatform::Linux, "settings_page")
            .expect("Linux settings-page translation item");
    assert_eq!(linux_settings.platform_name, "linux");
    assert_eq!(linux_settings.toolkit_name, "gtk4_libadwaita");
    assert_eq!(linux_settings.host_module_path, "src/linux_native_host.rs");
    assert_eq!(
        linux_settings.platform_host_module_paths,
        vec!["src/linux_native_host.rs", "src/linux_app.rs"]
    );
    assert!(linux_settings
        .preferred_app_core_edit_modules
        .contains(&"src/app_core/native_host_actions.rs"));
    assert!(linux_settings
        .preferred_app_core_edit_modules
        .contains(&"src/app_core/native_component_protocol.rs"));
    assert!(linux_settings
        .preferred_app_core_edit_modules
        .contains(&"src/app_core/settings_protocol.rs"));
    assert!(linux_settings
        .protocol_builder_names
        .contains(&"native_host_settings_control_button_specs"));
    assert!(linux_settings
        .protocol_builder_names
        .contains(&"native_host_settings_toggle_specs"));
    assert!(linux_settings
        .protocol_builder_names
        .contains(&"native_host_settings_dropdown_specs"));
    assert!(linux_settings.source_coverage_verified);
    assert!(linux_settings.target_smoke_required);
    assert!(!linux_settings.system_complete);

    let macos_dynamic =
        zsui_native_ui_host_translation_work_item_for(NativeUiPlatform::Macos, "dynamic_controls")
            .expect("macOS dynamic-controls translation item");
    assert!(macos_dynamic
        .dynamic_protocol_builder_names
        .contains(&"native_host_clip_row_specs"));
    assert!(macos_dynamic
        .preferred_app_core_edit_modules
        .contains(&"src/app_core/render_protocol.rs"));

    let linux_items =
        zsui_native_ui_host_translation_work_items_for_platform(NativeUiPlatform::Linux);
    assert_eq!(linux_items.len(), 5);
    assert_eq!(
        zsui_next_native_ui_host_translation_work_item_for_platform(NativeUiPlatform::Linux)
            .map(|item| item.surface_name),
        Some("main_window")
    );

    let manifest = zsui_framework_manifest();
    assert_eq!(manifest.native_ui_host_translation_work_items, work_items);
    let context = zsui_agent_context();
    assert_eq!(context.native_ui_host_translation_work_items, work_items);
}

#[test]
fn ui_extension_recipes_tell_ai_to_extend_app_core_before_native_hosts() {
    let recipes = zsui_ui_extension_recipes();
    assert_eq!(recipes.len(), 5);
    assert!(recipes.iter().all(|recipe| {
        !recipe.primary_app_core_modules.is_empty()
            && recipe
                .primary_app_core_modules
                .iter()
                .all(|path| path.starts_with("src/app_core") || *path == "src/settings_model.rs")
            && !recipe.platform_host_touchpoints.is_empty()
            && !recipe.expected_edit_order.is_empty()
            && !recipe.required_test_focus.is_empty()
            && !recipe.must_not_edit_first.is_empty()
    }));

    let settings = zsui_ui_extension_recipe_for_action_family("SettingsControl")
        .expect("settings-control extension recipe");
    assert_eq!(settings.recipe_name, "add_settings_control");
    assert_eq!(settings.surface_name, "settings_page");
    assert!(settings
        .primary_app_core_modules
        .contains(&"src/app_core/native_component_protocol.rs"));
    assert!(settings
        .primary_app_core_modules
        .contains(&"src/app_core/settings_protocol.rs"));
    assert!(settings
        .primary_app_core_modules
        .contains(&"src/settings_model.rs"));
    assert!(settings
        .spec_builder_names
        .contains(&"native_host_settings_control_button_specs"));
    assert!(settings
        .spec_builder_names
        .contains(&"native_host_settings_toggle_specs"));
    assert!(settings
        .spec_builder_names
        .contains(&"native_host_settings_dropdown_specs"));
    assert_eq!(
        settings.platform_host_touchpoints,
        vec![
            "src/settings_ui_host.rs",
            "src/macos_native_host.rs",
            "src/linux_native_host.rs"
        ]
    );
    assert!(settings
        .must_not_edit_first
        .contains(&"src/linux_native_host.rs"));

    let menu =
        zsui_ui_extension_recipe_for_action_family("Row").expect("row/menu extension recipe");
    assert_eq!(menu.surface_name, "menu");
    assert!(menu
        .spec_builder_names
        .contains(&"native_host_full_row_popup_menu_entries_for_groups"));
    assert!(menu
        .required_test_focus
        .contains(&"menu ids remain stable and non-overlapping"));

    let dynamic = zsui_ui_extension_recipes_for_surface("dynamic_controls");
    assert_eq!(dynamic.len(), 1);
    assert!(dynamic[0]
        .dynamic_spec_builder_names
        .contains(&"native_host_clip_row_specs"));

    let manifest = zsui_framework_manifest();
    assert_eq!(manifest.ui_extension_recipes, recipes);
    let context = zsui_agent_context();
    assert_eq!(context.ui_extension_recipes, recipes);
}

#[test]
fn command_queue_preserves_command_order() {
    let mut queue = CommandQueue::default();
    queue.push(Command {
        id: CommandId("copy"),
        scope: CommandScope::App,
        payload: CommandPayload::Text("a".to_string()),
    });
    queue.push(Command {
        id: CommandId("delete"),
        scope: CommandScope::Component(ComponentId(7)),
        payload: CommandPayload::ItemId(42),
    });

    assert_eq!(queue.len(), 2);
    assert_eq!(queue.pop().unwrap().id, CommandId("copy"));
    assert_eq!(queue.pop().unwrap().id, CommandId("delete"));
    assert!(queue.is_empty());
}

#[test]
fn native_host_ui_actions_map_to_window_commands() {
    let actions: Vec<_> = REQUIRED_NATIVE_HOST_UI_ACTIONS
        .iter()
        .map(|action| {
            (
                action.action_name(),
                action.button_label(),
                action.command().id.0,
                action.command().scope,
                action.toggles_search_surface(),
                action.opens_settings_surface(),
                action.should_close_host(),
            )
        })
        .collect();

    assert_eq!(
        actions,
        vec![
            (
                "toggle_search",
                "Search",
                command_ids::TOGGLE_SEARCH.0,
                CommandScope::Window,
                true,
                false,
                false
            ),
            (
                "open_settings",
                "Settings",
                command_ids::OPEN_SETTINGS.0,
                CommandScope::Window,
                false,
                true,
                false
            ),
            (
                "hide_window",
                "Hide",
                command_ids::HIDE_WINDOW.0,
                CommandScope::Window,
                false,
                false,
                false
            ),
            (
                "close_window",
                "Close",
                command_ids::CLOSE_WINDOW.0,
                CommandScope::Window,
                false,
                false,
                true
            ),
        ]
    );
}

#[test]
fn native_host_main_action_components_describe_reusable_native_buttons() {
    let specs = native_host_main_action_button_specs();
    let summary: Vec<_> = specs
        .iter()
        .map(|spec| {
            (
                spec.id,
                spec.label,
                NativeComponentKind::Button.role_name(),
                spec.style_role.role_name(),
                spec.action.action_name(),
                spec.width(),
                spec.height(),
            )
        })
        .collect();

    assert_eq!(
        summary,
        vec![
            (
                "main.toggle_search",
                "Search",
                "button",
                "plain",
                "toggle_search",
                96,
                32
            ),
            (
                "main.open_settings",
                "Settings",
                "button",
                "plain",
                "open_settings",
                96,
                32
            ),
            (
                "main.hide_window",
                "Hide",
                "button",
                "plain",
                "hide_window",
                96,
                32
            ),
            (
                "main.close_window",
                "Close",
                "button",
                "plain",
                "close_window",
                96,
                32
            ),
        ]
    );
    assert_eq!(
        specs.map(|spec| spec.action),
        REQUIRED_NATIVE_HOST_UI_ACTIONS
    );

    let erased_specs = native_host_main_action_component_specs();
    assert_eq!(
        erased_specs.map(|spec| match spec.action {
            NativeComponentAction::HostUi(action) => action,
            NativeComponentAction::MainTool(_) => {
                panic!("main action components must use host UI actions")
            }
            NativeComponentAction::ClipRow(_) => {
                panic!("main components must use host UI actions")
            }
            NativeComponentAction::Row(_) => panic!("main components must use host UI actions"),
            NativeComponentAction::SearchControl(_) => {
                panic!("main components must use host UI actions")
            }
            NativeComponentAction::StatusMenu(_) => {
                panic!("main components must use host UI actions")
            }
            NativeComponentAction::Settings(_) => {
                panic!("main components must use host UI actions")
            }
            NativeComponentAction::SettingsControl(_) => {
                panic!("main components must use host UI actions")
            }
            NativeComponentAction::SettingsPlatform(_) => {
                panic!("main components must use host UI actions")
            }
            NativeComponentAction::SettingsGroup(_) => {
                panic!("main components must use host UI actions")
            }
            NativeComponentAction::EditText(_) => {
                panic!("main components must use host UI actions")
            }
            #[cfg(feature = "vv-paste")]
            NativeComponentAction::VvSelect(_) => {
                panic!("main components must use host UI actions")
            }
            NativeComponentAction::Dialog(_) => panic!("main components must use host UI actions"),
        }),
        REQUIRED_NATIVE_HOST_UI_ACTIONS
    );
}

#[test]
fn native_component_protocol_exposes_typed_specs_before_erasing_for_hosts() {
    let buttons = native_host_main_action_button_specs();
    assert_eq!(buttons.len(), 4);
    assert_eq!(buttons[0].id, "main.toggle_search");
    assert_eq!(buttons[0].action, NativeHostUiAction::ToggleSearch);
    assert_eq!(
        buttons[0].label,
        NativeHostUiAction::ToggleSearch.button_label()
    );

    let search = native_host_search_input_specs();
    assert_eq!(search.len(), 1);
    assert_eq!(search[0].id, "main.search");
    assert_eq!(search[0].action, NativeHostSearchControlAction::UpdateText);
    assert_eq!(
        search[0].placeholder,
        NativeHostSearchControlAction::UpdateText.placeholder()
    );

    let menu = native_host_status_menu_item_specs();
    assert_eq!(menu.len(), REQUIRED_NATIVE_HOST_STATUS_MENU_ACTIONS.len());
    assert_eq!(menu[0].id, "status.toggle_window");
    assert_eq!(menu[0].action, NativeHostStatusMenuAction::ToggleWindow);
    assert_eq!(menu[0].icon_name, "window-new-symbolic");
    assert_eq!(menu[0].accelerator_key, "z");
    assert!(!menu[0].starts_section);
    assert_eq!(
        menu.iter()
            .find(|spec| spec.action == NativeHostStatusMenuAction::ToggleClipboardCapture)
            .map(|spec| spec.accelerator_key),
        Some("c")
    );
    assert_eq!(menu[menu.len() - 1].icon_name, "application-exit-symbolic");
    assert_eq!(menu[menu.len() - 1].accelerator_key, "q");
    assert!(menu[menu.len() - 1].starts_section);

    let erased = native_host_main_action_component_specs();
    assert_eq!(erased[0].kind, NativeComponentKind::Button);
    assert_eq!(
        erased[0].action,
        NativeComponentAction::HostUi(buttons[0].action)
    );
}

#[test]
fn native_component_family_descriptors_keep_typed_specs_ahead_of_erased_components() {
    let descriptors = native_component_family_descriptors();
    assert!(descriptors.len() >= 12);
    assert!(descriptors.iter().all(|descriptor| {
        !descriptor.family_name.is_empty()
            && !descriptor.surface_name.is_empty()
            && !descriptor.action_family_name.is_empty()
            && !descriptor.typed_spec_name.is_empty()
            && !descriptor.spec_builder_name.is_empty()
            && !descriptor.erased_spec_name.is_empty()
            && !descriptor.extension_rule.contains("platform first")
    }));

    let settings_control =
        native_component_family_descriptor_for_builder("native_host_settings_control_button_specs")
            .expect("settings control typed spec family");
    assert_eq!(settings_control.family_name, "settings_control_button");
    assert_eq!(settings_control.surface_name, "settings_page");
    assert_eq!(settings_control.action_family_name, "SettingsControl");
    assert_eq!(
        settings_control.typed_spec_name,
        "NativeButtonSpec<NativeHostSettingsControlAction>"
    );
    assert_eq!(
        settings_control.erased_spec_builder_name,
        Some("native_host_settings_control_component_specs")
    );
    assert_eq!(
        settings_control.erased_component_kind,
        NativeComponentKind::Button
    );
    assert!(!settings_control.dynamic);
    assert!(!settings_control.is_feature_gated());
    assert!(settings_control
        .extension_rule
        .contains("settings control action"));

    let settings_toggle =
        native_component_family_descriptor_for_builder("native_host_settings_toggle_specs")
            .expect("settings toggle typed spec family");
    assert_eq!(settings_toggle.family_name, "settings_toggle");
    assert_eq!(
        settings_toggle.typed_spec_name,
        "NativeToggleSpec<NativeHostSettingsControlAction>"
    );
    assert_eq!(
        settings_toggle.erased_component_kind,
        NativeComponentKind::Toggle
    );

    let settings_dropdown =
        native_component_family_descriptor_for_builder("native_host_settings_dropdown_specs")
            .expect("settings dropdown typed spec family");
    assert_eq!(settings_dropdown.family_name, "settings_dropdown");
    assert_eq!(
        settings_dropdown.typed_spec_name,
        "NativeDropdownSpec<NativeHostSettingsControlAction>"
    );
    assert_eq!(
        settings_dropdown.erased_component_kind,
        NativeComponentKind::Dropdown
    );

    let settings_control_erased = native_component_family_descriptor_for_builder(
        "native_host_settings_control_component_specs",
    )
    .expect("settings control erased spec family");
    assert_eq!(settings_control_erased, settings_control);

    let dynamic = native_component_family_descriptors_for_surface("dynamic_controls");
    assert!(dynamic.iter().any(|descriptor| {
        descriptor.family_name == "clip_row_instance"
            && descriptor.spec_builder_name == "native_host_clip_row_specs"
            && descriptor.typed_spec_name == "NativeClipRowSpec"
            && descriptor.action_family_name == "ClipRow"
            && descriptor.dynamic
            && descriptor.erased_spec_builder_name == Some("native_host_clip_row_component_specs")
    }));

    #[cfg(feature = "vv-paste")]
    {
        let vv_select =
            native_component_family_descriptor_for_builder("native_host_vv_select_specs")
                .expect("VV select component family");
        assert_eq!(vv_select.typed_spec_name, "NativeVvSelectSpec");
        assert_eq!(
            vv_select.erased_spec_builder_name,
            Some("native_host_vv_select_component_specs")
        );
        assert_eq!(vv_select.feature_gate, Some("vv-paste"));
        assert!(vv_select.is_feature_gated());
        assert!(vv_select.dynamic);
    }

    #[cfg(not(feature = "vv-paste"))]
    {
        assert!(
            native_component_family_descriptor_for_builder("native_host_vv_select_specs").is_none()
        );
    }

    let main_window = native_component_family_descriptors_for_surface("main_window");
    assert_eq!(
        main_window
            .iter()
            .map(|descriptor| descriptor.spec_builder_name)
            .collect::<Vec<_>>(),
        vec![
            "native_host_main_action_button_specs",
            "native_host_main_tool_button_specs",
            "native_host_search_input_specs"
        ]
    );
}

#[test]
fn zsui_manifest_and_agent_context_expose_native_component_families() {
    let families = native_component_family_descriptors();
    let manifest = zsui_framework_manifest();
    assert_eq!(manifest.native_component_families, families);
    assert!(manifest.native_component_families.iter().any(|family| {
        family.surface_name == "settings_page"
            && family.action_family_name == "SettingsControl"
            && family.typed_spec_name == "NativeToggleSpec<NativeHostSettingsControlAction>"
            && family.spec_builder_name == "native_host_settings_toggle_specs"
    }));

    let context = zsui_agent_context();
    assert_eq!(context.native_component_families, families);
    assert!(context.native_component_families.iter().any(|family| {
        family.surface_name == "dynamic_controls"
            && family.action_family_name == "ClipRow"
            && family.typed_spec_name == "NativeClipRowSpec"
            && family.dynamic
    }));
}

#[test]
fn feature_gated_optional_native_component_specs_follow_enabled_features() {
    let main_tools = native_host_main_tool_button_specs()
        .iter()
        .map(|spec| spec.action.action_name())
        .collect::<Vec<_>>();
    assert_eq!(
        main_tools.contains(&"main_vv_popup"),
        cfg!(feature = "vv-paste")
    );
    assert_eq!(
        main_tools.contains(&"main_vv_trigger"),
        cfg!(feature = "vv-paste")
    );
    assert_eq!(
        main_tools.contains(&"main_vv_trigger"),
        cfg!(feature = "vv-paste")
    );

    let vv_select_specs =
        native_host_vv_select_component_specs(&native_host_vv_popup_render_plan(), 480, 240);
    assert_eq!(!vv_select_specs.is_empty(), cfg!(feature = "vv-paste"));

    let status_actions = native_host_status_menu_component_specs()
        .iter()
        .map(|spec| spec.action.action_name())
        .collect::<Vec<_>>();
    assert_eq!(
        status_actions.contains(&"status_toggle_lan_sync"),
        cfg!(feature = "lan-sync")
    );

    let settings_controls = native_host_settings_control_button_specs()
        .iter()
        .map(|spec| spec.action.action_name())
        .collect::<Vec<_>>();
    assert!(settings_controls.contains(&"settings_toggle_autostart"));
    assert_eq!(
        settings_controls.contains(&"settings_toggle_lan_sync"),
        cfg!(feature = "lan-sync")
    );
    assert_eq!(
        settings_controls.contains(&"settings_toggle_cloud_sync"),
        cfg!(feature = "cloud-sync")
    );
    assert_eq!(
        settings_controls.contains(&"settings_open_sync_mode_dropdown"),
        cfg!(any(feature = "cloud-sync", feature = "lan-sync"))
    );
    let settings_toggles = native_host_settings_toggle_specs()
        .iter()
        .map(|spec| spec.action.action_name())
        .collect::<Vec<_>>();
    assert!(settings_toggles.contains(&"settings_toggle_autostart"));
    assert!(settings_toggles.contains(&"settings_toggle_clipboard_capture"));
    assert_eq!(
        settings_toggles.contains(&"settings_toggle_lan_sync"),
        cfg!(feature = "lan-sync")
    );
    assert_eq!(
        settings_toggles.contains(&"settings_toggle_cloud_sync"),
        cfg!(feature = "cloud-sync")
    );
    let settings_dropdowns = native_host_settings_dropdown_specs()
        .iter()
        .map(|spec| spec.action.action_name())
        .collect::<Vec<_>>();
    assert_eq!(
        settings_dropdowns.contains(&"settings_open_sync_mode_dropdown"),
        cfg!(any(feature = "cloud-sync", feature = "lan-sync"))
    );

    let row_actions = native_host_row_action_button_specs()
        .iter()
        .map(|spec| spec.action.action_name())
        .collect::<Vec<_>>();
    assert_eq!(
        row_actions.contains(&"row_text_translate"),
        cfg!(feature = "ai-actions")
    );
}

#[test]
fn native_host_main_tool_components_describe_reusable_native_buttons() {
    let specs = native_host_main_tool_button_specs();
    let expected_summary = vec![
        (
            "main.row_menu",
            "Row Menu",
            "button",
            "plain",
            "main_row_menu",
            64,
            68,
        ),
        (
            "main.group_filter",
            "Group Filter",
            "button",
            "plain",
            "main_group_filter",
            88,
            32,
        ),
        #[cfg(feature = "vv-paste")]
        (
            "main.vv_popup",
            "VV Popup",
            "button",
            "plain",
            "main_vv_popup",
            88,
            32,
        ),
        #[cfg(feature = "vv-paste")]
        (
            "main.vv_trigger",
            "VV Trigger",
            "button",
            "plain",
            "main_vv_trigger",
            88,
            32,
        ),
    ];
    let summary: Vec<_> = specs
        .iter()
        .map(|spec| {
            (
                spec.id,
                spec.label,
                NativeComponentKind::Button.role_name(),
                spec.style_role.role_name(),
                spec.action.action_name(),
                spec.width(),
                spec.height(),
            )
        })
        .collect();

    assert_eq!(summary, expected_summary);
    assert_eq!(
        specs.iter().map(|spec| spec.action).collect::<Vec<_>>(),
        native_host_main_tool_actions()
    );

    let erased_specs = native_host_main_tool_component_specs();
    assert_eq!(
        erased_specs
            .iter()
            .map(|spec| match spec.action {
                NativeComponentAction::MainTool(action) => action,
                NativeComponentAction::HostUi(_) => {
                    panic!("main tool components must use main tool actions")
                }
                NativeComponentAction::ClipRow(_) => {
                    panic!("main tool components must use tool actions")
                }
                NativeComponentAction::Row(_) =>
                    panic!("main tool components must use tool actions"),
                NativeComponentAction::SearchControl(_) => {
                    panic!("main tool components must use tool actions")
                }
                NativeComponentAction::StatusMenu(_) => {
                    panic!("main tool components must use tool actions")
                }
                NativeComponentAction::Settings(_) => {
                    panic!("main tool components must use tool actions")
                }
                NativeComponentAction::SettingsControl(_) => {
                    panic!("main tool components must use tool actions")
                }
                NativeComponentAction::SettingsPlatform(_) => {
                    panic!("main tool components must use tool actions")
                }
                NativeComponentAction::SettingsGroup(_) => {
                    panic!("main tool components must use tool actions")
                }
                NativeComponentAction::EditText(_) => {
                    panic!("main tool components must use tool actions")
                }
                #[cfg(feature = "vv-paste")]
                NativeComponentAction::VvSelect(_) => {
                    panic!("main tool components must use tool actions")
                }
                NativeComponentAction::Dialog(_) => {
                    panic!("main tool components must use tool actions")
                }
            })
            .collect::<Vec<_>>(),
        native_host_main_tool_actions()
    );

    let actions = native_host_main_tool_actions();
    assert!(actions[0].opens_row_menu());
    assert!(actions[1].opens_group_filter_menu());
    assert_eq!(
        actions.iter().any(|action| action.opens_vv_popup()),
        cfg!(feature = "vv-paste")
    );
    assert_eq!(
        actions.iter().any(|action| action.triggers_vv_demo()),
        cfg!(feature = "vv-paste")
    );
}

#[test]
fn native_host_row_action_components_describe_reusable_native_buttons() {
    let specs = native_host_row_action_button_specs();
    let summary: Vec<_> = specs
        .iter()
        .map(|spec| {
            (
                spec.id,
                spec.label,
                NativeComponentKind::Button.role_name(),
                spec.style_role.role_name(),
                spec.action.action_name(),
                spec.width(),
                spec.height(),
            )
        })
        .collect();

    #[cfg(not(feature = "ai-actions"))]
    let expected = vec![
        (
            "row.paste",
            "Paste",
            "button",
            "plain",
            "row_paste",
            104,
            32,
        ),
        ("row.copy", "Copy", "button", "plain", "row_copy", 104, 32),
        (
            "row.pin",
            "Pin",
            "button",
            "plain",
            "row_toggle_pin",
            104,
            32,
        ),
        (
            "row.to_phrase",
            "To Phrase",
            "button",
            "plain",
            "row_to_phrase",
            104,
            32,
        ),
        (
            "row.delete",
            "Delete",
            "button",
            "plain",
            "row_delete",
            104,
            32,
        ),
        ("row.edit", "Edit", "button", "plain", "row_edit", 104, 32),
        (
            "row.open_path",
            "Open Path",
            "button",
            "plain",
            "row_open_path",
            104,
            32,
        ),
    ];

    #[cfg(feature = "ai-actions")]
    let expected = vec![
        (
            "row.paste",
            "Paste",
            "button",
            "plain",
            "row_paste",
            104,
            32,
        ),
        ("row.copy", "Copy", "button", "plain", "row_copy", 104, 32),
        (
            "row.pin",
            "Pin",
            "button",
            "plain",
            "row_toggle_pin",
            104,
            32,
        ),
        (
            "row.to_phrase",
            "To Phrase",
            "button",
            "plain",
            "row_to_phrase",
            104,
            32,
        ),
        (
            "row.delete",
            "Delete",
            "button",
            "plain",
            "row_delete",
            104,
            32,
        ),
        ("row.edit", "Edit", "button", "plain", "row_edit", 104, 32),
        (
            "row.open_path",
            "Open Path",
            "button",
            "plain",
            "row_open_path",
            104,
            32,
        ),
        (
            "row.text_translate",
            "Translate",
            "button",
            "plain",
            "row_text_translate",
            104,
            32,
        ),
    ];

    assert_eq!(summary, expected);
    assert_eq!(
        specs.iter().map(|spec| spec.action).collect::<Vec<_>>(),
        native_host_row_actions()
    );

    let erased_specs = native_host_row_action_component_specs();
    assert_eq!(
        erased_specs
            .iter()
            .map(|spec| match spec.action {
                NativeComponentAction::Row(action) => action,
                NativeComponentAction::MainTool(_) => panic!("row components must use row actions"),
                NativeComponentAction::HostUi(_) => panic!("row components must use row actions"),
                NativeComponentAction::ClipRow(_) => panic!("row components must use row actions"),
                NativeComponentAction::SearchControl(_) => {
                    panic!("row components must use row actions")
                }
                NativeComponentAction::StatusMenu(_) => {
                    panic!("row components must use row actions")
                }
                NativeComponentAction::Settings(_) => panic!("row components must use row actions"),
                NativeComponentAction::SettingsControl(_) => {
                    panic!("row components must use row actions")
                }
                NativeComponentAction::SettingsPlatform(_) => {
                    panic!("row components must use row actions")
                }
                NativeComponentAction::SettingsGroup(_) => {
                    panic!("row components must use row actions")
                }
                NativeComponentAction::EditText(_) => {
                    panic!("row components must use row actions")
                }
                #[cfg(feature = "vv-paste")]
                NativeComponentAction::VvSelect(_) => {
                    panic!("row components must use row actions")
                }
                NativeComponentAction::Dialog(_) => panic!("row components must use row actions"),
            })
            .collect::<Vec<_>>(),
        native_host_row_actions()
    );
}

#[test]
fn native_host_clip_row_specs_describe_dynamic_native_rows() {
    let items = vec![
        NativeHostClipListItemProjection::new(7, "Browser", "Copied text"),
        NativeHostClipListItemProjection::new(9, "Files", "C:\\Temp\\a.txt"),
    ];
    let specs = native_host_clip_row_specs(&items, 3);
    let summary: Vec<_> = specs
        .iter()
        .map(|spec| {
            (
                spec.id.as_str(),
                spec.label.as_str(),
                spec.action.action_name(),
                spec.width(),
                spec.height(),
            )
        })
        .collect();

    assert_eq!(
        summary,
        vec![
            (
                "clip.row.1",
                "Browser - Copied text",
                "clip_row_select",
                512,
                22
            ),
            (
                "clip.row.2",
                "Files - C:\\Temp\\a.txt",
                "clip_row_select",
                512,
                22
            ),
            ("clip.row.3", "", "clip_row_select", 512, 22),
        ]
    );

    for (index, spec) in specs.iter().enumerate() {
        let action = spec.action;
        assert_eq!(action.index, index);
        assert_eq!(action.has_item(), index < items.len());
    }
    let last_action = specs[2].action;
    assert_eq!(last_action.item_id, 0);
    assert_eq!(NATIVE_HOST_CLIP_ROW_CAPACITY, 64);
}

#[test]
fn native_host_clip_row_component_specs_remain_erased_compatibility_layer() {
    let items = vec![NativeHostClipListItemProjection::new(
        7,
        "Browser",
        "Copied text",
    )];
    let specs = native_host_clip_row_component_specs(&items, 2);

    assert_eq!(specs[0].kind, NativeComponentKind::Button);
    let NativeComponentAction::ClipRow(action) = specs[0].action else {
        panic!("erased clip row component must still carry clip row action");
    };
    assert_eq!(action.item_id, 7);
    let NativeComponentAction::ClipRow(empty_action) = specs[1].action else {
        panic!("empty erased clip row must still carry clip row action");
    };
    assert_eq!(empty_action.item_id, 0);
}

#[test]
fn native_host_status_menu_actions_map_to_tray_commands() {
    let mut cases = vec![
        (
            NativeHostStatusMenuAction::ToggleWindow,
            "status_toggle_window",
            "Show ZSClip",
            MainTrayMenuAction::ToggleWindow,
            menu_ids::TRAY_TOGGLE,
            false,
        ),
        (
            NativeHostStatusMenuAction::ToggleClipboardCapture,
            "status_toggle_clipboard_capture",
            "Toggle Capture",
            MainTrayMenuAction::ToggleClipboardCapture,
            menu_ids::TRAY_CAPTURE_TOGGLE,
            false,
        ),
        (
            NativeHostStatusMenuAction::Exit,
            "status_exit",
            "Exit",
            MainTrayMenuAction::Exit,
            menu_ids::TRAY_EXIT,
            true,
        ),
    ];
    #[cfg(feature = "lan-sync")]
    cases.insert(
        2,
        (
            NativeHostStatusMenuAction::ToggleLanSync,
            "status_toggle_lan_sync",
            "Toggle LAN Sync",
            MainTrayMenuAction::ToggleLanSync,
            menu_ids::TRAY_LAN_TOGGLE,
            false,
        ),
    );

    for (action, action_name, menu_label, tray_action, menu_id, should_exit) in cases {
        let command = action.command();
        assert_eq!(action.action_name(), action_name);
        assert_eq!(action.menu_label(), menu_label);
        assert_eq!(action.tray_action(), tray_action);
        assert_eq!(action.menu_id(), menu_id);
        assert_eq!(action.should_exit_host(), should_exit);
        assert_eq!(command.id, command_ids::INVOKE_MAIN_MENU_COMMAND);
        assert_eq!(command.scope, CommandScope::Window);
        assert_eq!(command.payload, CommandPayload::ControlId(menu_id as i64));
    }
}

#[test]
fn native_host_status_menu_components_describe_reusable_native_menu_items() {
    let specs = native_host_status_menu_component_specs();
    let menu_specs = native_host_status_menu_item_specs();
    let summary: Vec<_> = specs
        .iter()
        .map(|spec| {
            (
                spec.id,
                spec.label,
                spec.kind.role_name(),
                spec.action.action_name(),
            )
        })
        .collect();

    assert_eq!(
        summary,
        vec![
            (
                "status.toggle_window",
                "Show ZSClip",
                "menu_item",
                "status_toggle_window",
            ),
            (
                "status.toggle_capture",
                "Toggle Capture",
                "menu_item",
                "status_toggle_clipboard_capture",
            ),
            #[cfg(feature = "lan-sync")]
            (
                "status.toggle_lan_sync",
                "Toggle LAN Sync",
                "menu_item",
                "status_toggle_lan_sync",
            ),
            ("status.exit", "Exit", "menu_item", "status_exit"),
        ]
    );
    assert_eq!(
        menu_specs
            .iter()
            .map(|spec| {
                (
                    spec.id,
                    spec.icon_name,
                    spec.accelerator_key,
                    spec.starts_section,
                )
            })
            .collect::<Vec<_>>(),
        vec![
            ("status.toggle_window", "window-new-symbolic", "z", false),
            ("status.toggle_capture", "media-record-symbolic", "c", false),
            #[cfg(feature = "lan-sync")]
            (
                "status.toggle_lan_sync",
                "network-wireless-symbolic",
                "l",
                false
            ),
            ("status.exit", "application-exit-symbolic", "q", true),
        ]
    );
    assert_eq!(
        specs
            .iter()
            .map(|spec| match spec.action {
                NativeComponentAction::StatusMenu(action) => action,
                _ => panic!("status menu components must use status menu actions"),
            })
            .collect::<Vec<_>>(),
        REQUIRED_NATIVE_HOST_STATUS_MENU_ACTIONS.to_vec()
    );
}

#[test]
fn native_host_settings_actions_map_to_settings_commands() {
    let actions: Vec<_> = REQUIRED_NATIVE_HOST_SETTINGS_ACTIONS
        .iter()
        .map(|action| {
            (
                action.action_name(),
                action.button_label(),
                action.command().id.0,
                action.command().scope,
                action.should_close_settings_surface(),
            )
        })
        .collect();

    assert_eq!(
        actions,
        vec![
            (
                "save_settings",
                "Save",
                command_ids::SAVE_SETTINGS.0,
                CommandScope::Window,
                false
            ),
            (
                "close_settings",
                "Close",
                command_ids::CLOSE_SETTINGS.0,
                CommandScope::Window,
                true
            ),
            (
                "open_settings_config",
                "Open Config",
                command_ids::OPEN_SETTINGS_CONFIG.0,
                CommandScope::Window,
                false
            ),
        ]
    );
}

#[test]
fn native_host_settings_action_components_describe_reusable_native_buttons() {
    let specs = native_host_settings_action_button_specs();
    let summary: Vec<_> = specs
        .iter()
        .map(|spec| {
            (
                spec.id,
                spec.label,
                NativeComponentKind::Button.role_name(),
                spec.style_role.role_name(),
                spec.action.action_name(),
                spec.width(),
                spec.height(),
            )
        })
        .collect();

    assert_eq!(
        summary,
        vec![
            (
                "settings.action.save",
                "Save",
                "button",
                "suggested",
                "save_settings",
                116,
                32
            ),
            (
                "settings.action.close",
                "Close",
                "button",
                "plain",
                "close_settings",
                116,
                32
            ),
            (
                "settings.action.open_config",
                "Open Config",
                "button",
                "plain",
                "open_settings_config",
                116,
                32
            ),
        ]
    );
    assert_eq!(
        specs.map(|spec| spec.action),
        REQUIRED_NATIVE_HOST_SETTINGS_ACTIONS
    );

    let erased_specs = native_host_settings_action_component_specs();
    assert_eq!(
        erased_specs.map(|spec| match spec.action {
            NativeComponentAction::Settings(action) => action,
            _ => panic!("settings action compatibility specs must erase settings actions"),
        }),
        REQUIRED_NATIVE_HOST_SETTINGS_ACTIONS
    );
}

#[test]
fn native_host_search_text_action_maps_to_text_command() {
    let action = NativeHostSearchTextAction::new("  hello  ");
    let command = action.command();

    assert_eq!(action.normalized_text(), "hello");
    assert_eq!(command.id, command_ids::UPDATE_SEARCH_TEXT);
    assert_eq!(command.scope, CommandScope::Window);
    assert_eq!(
        command.payload,
        CommandPayload::Text("  hello  ".to_string())
    );
}

#[test]
fn native_host_search_components_describe_reusable_native_inputs() {
    let specs = native_host_search_component_specs();
    let summary: Vec<_> = specs
        .iter()
        .map(|spec| {
            (
                spec.id,
                spec.label,
                spec.kind.role_name(),
                spec.action.action_name(),
                spec.width(),
                spec.height(),
            )
        })
        .collect();

    assert_eq!(
        summary,
        vec![(
            "main.search",
            "Search clipboard",
            "search_input",
            "search_text_changed",
            400,
            28
        )]
    );
    assert_eq!(
        specs.map(|spec| match spec.action {
            NativeComponentAction::SearchControl(action) => action,
            _ => panic!("search components must use search control actions"),
        }),
        REQUIRED_NATIVE_HOST_SEARCH_CONTROL_ACTIONS
    );
}

#[test]
fn native_host_settings_control_actions_map_to_settings_commands() {
    let mut cases = vec![
        (
            NativeHostSettingsControlAction::ToggleAutostart,
            "settings_toggle_autostart",
            "Auto Start",
            SettingsControlRole::Toggle,
            command_ids::TOGGLE_SETTINGS_CONTROL,
            5_010,
            Some("auto_start"),
        ),
        (
            NativeHostSettingsControlAction::ToggleClipboardCapture,
            "settings_toggle_clipboard_capture",
            "Capture",
            SettingsControlRole::Toggle,
            command_ids::TOGGLE_SETTINGS_CONTROL,
            5_101,
            Some("capture_enable"),
        ),
    ];
    #[cfg(feature = "lan-sync")]
    cases.push((
        NativeHostSettingsControlAction::ToggleLanSync,
        "settings_toggle_lan_sync",
        "LAN Sync",
        SettingsControlRole::Toggle,
        command_ids::TOGGLE_SETTINGS_CONTROL,
        7_102,
        Some("lan_enable"),
    ));
    #[cfg(feature = "cloud-sync")]
    cases.push((
        NativeHostSettingsControlAction::ToggleCloudSync,
        "settings_toggle_cloud_sync",
        "Cloud Sync",
        SettingsControlRole::Toggle,
        command_ids::TOGGLE_SETTINGS_CONTROL,
        7_103,
        Some("cloud_enable"),
    ));
    #[cfg(any(feature = "cloud-sync", feature = "lan-sync"))]
    cases.push((
        NativeHostSettingsControlAction::OpenSyncModeDropdown,
        "settings_open_sync_mode_dropdown",
        "Sync Mode",
        SettingsControlRole::Dropdown,
        command_ids::OPEN_SETTINGS_DROPDOWN,
        6_102,
        Some("multi_sync_mode"),
    ));

    for (action, action_name, button_label, role, command_id, control_id, binding_key) in cases {
        let command = action.command();
        assert_eq!(action.action_name(), action_name);
        assert_eq!(action.button_label(), button_label);
        assert_eq!(action.role(), role);
        assert_eq!(action.control_id(), control_id);
        assert_eq!(action.binding_control_key(), binding_key);
        assert_eq!(command.id, command_id);
        assert_eq!(command.scope, CommandScope::Window);
        assert_eq!(command.payload, CommandPayload::ControlId(control_id));
    }
}

#[test]
fn native_host_settings_control_components_describe_reusable_native_controls() {
    let tab_specs = native_host_settings_page_tab_specs();
    assert_eq!(
        tab_specs
            .iter()
            .map(|spec| (spec.id, spec.label, spec.section_names))
            .collect::<Vec<_>>(),
        vec![
            (
                "settings.tab.general",
                "General",
                &["settings_summary", "settings_controls"][..]
            ),
            (
                "settings.tab.groups",
                "Groups",
                &["group_selector", "group_actions"][..]
            ),
            (
                "settings.tab.actions",
                "Actions",
                &["settings_actions", "platform_actions", "dialog_actions"][..]
            ),
        ]
    );
    assert_eq!(
        native_host_settings_section_specs()
            .iter()
            .map(|spec| (spec.section_name, spec.tab_kind, spec.label))
            .collect::<Vec<_>>(),
        vec![
            (
                "settings_summary",
                NativeSettingsPageTabKind::General,
                "Settings Summary"
            ),
            (
                "settings_controls",
                NativeSettingsPageTabKind::General,
                "Shared Controls"
            ),
            (
                "group_selector",
                NativeSettingsPageTabKind::Groups,
                "Group Management"
            ),
            (
                "group_actions",
                NativeSettingsPageTabKind::Groups,
                "Group Actions"
            ),
            (
                "settings_actions",
                NativeSettingsPageTabKind::Actions,
                "Settings Actions"
            ),
            (
                "platform_actions",
                NativeSettingsPageTabKind::Actions,
                "Platform Actions"
            ),
            (
                "dialog_actions",
                NativeSettingsPageTabKind::Actions,
                "Dialog Actions"
            ),
        ]
    );
    assert_eq!(
        native_host_settings_section_label("settings_controls"),
        Some("Shared Controls")
    );
    assert_eq!(native_host_settings_section_label("missing_section"), None);

    let toggle_specs = native_host_settings_toggle_specs();
    let toggle_summary: Vec<_> = toggle_specs
        .iter()
        .map(|spec| {
            (
                spec.id,
                spec.label,
                NativeComponentKind::Toggle.role_name(),
                spec.action.action_name(),
                spec.width(),
                spec.height(),
            )
        })
        .collect();

    assert_eq!(
        toggle_summary,
        vec![
            (
                "settings.control.autostart",
                "Auto Start",
                "toggle",
                "settings_toggle_autostart",
                132,
                32
            ),
            (
                "settings.control.capture",
                "Capture",
                "toggle",
                "settings_toggle_clipboard_capture",
                132,
                32
            ),
            #[cfg(feature = "lan-sync")]
            (
                "settings.control.lan_sync",
                "LAN Sync",
                "toggle",
                "settings_toggle_lan_sync",
                132,
                32
            ),
            #[cfg(feature = "cloud-sync")]
            (
                "settings.control.cloud_sync",
                "Cloud Sync",
                "toggle",
                "settings_toggle_cloud_sync",
                132,
                32
            ),
        ]
    );

    let dropdown_specs = native_host_settings_dropdown_specs();
    let dropdown_summary: Vec<_> = dropdown_specs
        .iter()
        .map(|spec| {
            (
                spec.id,
                spec.label,
                NativeComponentKind::Dropdown.role_name(),
                spec.action.action_name(),
                spec.width(),
                spec.height(),
                spec.options
                    .iter()
                    .map(|option| (option.raw_value, option.label))
                    .collect::<Vec<_>>(),
            )
        })
        .collect();

    assert_eq!(
        dropdown_summary,
        vec![
            #[cfg(any(feature = "cloud-sync", feature = "lan-sync"))]
            (
                "settings.control.sync_mode",
                "Sync Mode",
                "dropdown",
                "settings_open_sync_mode_dropdown",
                132,
                32,
                vec![("off", "关闭"), ("webdav", "WebDAV"), ("lan", "局域网")]
            ),
        ]
    );

    let specs = native_host_settings_control_button_specs();
    assert_eq!(
        specs.iter().map(|spec| spec.action).collect::<Vec<_>>(),
        REQUIRED_NATIVE_HOST_SETTINGS_CONTROL_ACTIONS.to_vec()
    );

    let erased_specs = native_host_settings_control_component_specs();
    assert_eq!(
        erased_specs
            .iter()
            .map(|spec| match spec.action {
                NativeComponentAction::SettingsControl(action) => action,
                _ => panic!("settings control compatibility specs must erase control actions"),
            })
            .collect::<Vec<_>>(),
        REQUIRED_NATIVE_HOST_SETTINGS_CONTROL_ACTIONS.to_vec()
    );
    assert!(erased_specs
        .iter()
        .any(|spec| spec.kind == NativeComponentKind::Toggle));
    assert_eq!(
        erased_specs
            .iter()
            .any(|spec| spec.kind == NativeComponentKind::Dropdown),
        cfg!(any(feature = "cloud-sync", feature = "lan-sync"))
    );
}

#[test]
fn native_host_settings_platform_actions_have_expected_labels() {
    let cases = [
        (
            NativeHostSettingsPlatformAction::OpenSourceRepository,
            "settings_open_source_repository",
            "Open Source",
        ),
        (
            NativeHostSettingsPlatformAction::CheckForUpdates,
            "settings_check_for_updates",
            "Check Updates",
        ),
        (
            NativeHostSettingsPlatformAction::OpenWpsTaskpaneDocs,
            "settings_open_wps_taskpane_docs",
            "WPS Docs",
        ),
    ];

    for (action, action_name, button_label) in cases {
        assert_eq!(action.action_name(), action_name);
        assert_eq!(action.button_label(), button_label);
    }
}

#[test]
fn native_host_settings_platform_components_describe_reusable_native_buttons() {
    let specs = native_host_settings_platform_button_specs();
    let summary: Vec<_> = specs
        .iter()
        .map(|spec| {
            (
                spec.id,
                spec.label,
                NativeComponentKind::Button.role_name(),
                spec.style_role.role_name(),
                spec.action.action_name(),
                spec.width(),
                spec.height(),
            )
        })
        .collect();

    assert_eq!(
        summary,
        vec![
            (
                "settings.platform.open_source",
                "Open Source",
                "button",
                "plain",
                "settings_open_source_repository",
                132,
                32
            ),
            (
                "settings.platform.check_updates",
                "Check Updates",
                "button",
                "plain",
                "settings_check_for_updates",
                132,
                32
            ),
            (
                "settings.platform.wps_docs",
                "WPS Docs",
                "button",
                "plain",
                "settings_open_wps_taskpane_docs",
                132,
                32
            ),
        ]
    );
    assert_eq!(
        specs.map(|spec| spec.action),
        REQUIRED_NATIVE_HOST_SETTINGS_PLATFORM_ACTIONS
    );

    let erased_specs = native_host_settings_platform_component_specs();
    assert_eq!(
        erased_specs.map(|spec| match spec.action {
            NativeComponentAction::SettingsPlatform(action) => action,
            _ => panic!("settings platform compatibility specs must erase platform actions"),
        }),
        REQUIRED_NATIVE_HOST_SETTINGS_PLATFORM_ACTIONS
    );
}

#[test]
fn native_host_settings_group_components_describe_reusable_native_buttons() {
    let specs = native_host_settings_group_button_specs();
    let summary: Vec<_> = specs
        .iter()
        .map(|spec| {
            (
                spec.id,
                spec.label,
                NativeComponentKind::Button.role_name(),
                spec.style_role.role_name(),
                spec.action.action_name(),
                spec.width(),
                spec.height(),
            )
        })
        .collect();

    assert_eq!(
        summary,
        vec![
            (
                "settings.group.show_records",
                "Records",
                "button",
                "plain",
                "settings_group_show_records",
                92,
                28
            ),
            (
                "settings.group.show_phrases",
                "Phrases",
                "button",
                "plain",
                "settings_group_show_phrases",
                92,
                28
            ),
            (
                "settings.group.add",
                "Add",
                "button",
                "plain",
                "settings_group_add",
                78,
                28
            ),
            (
                "settings.group.rename",
                "Rename",
                "button",
                "plain",
                "settings_group_rename",
                78,
                28
            ),
            (
                "settings.group.delete",
                "Delete",
                "button",
                "destructive",
                "settings_group_delete",
                78,
                28
            ),
            (
                "settings.group.move_up",
                "Up",
                "button",
                "plain",
                "settings_group_move_up",
                78,
                28
            ),
            (
                "settings.group.move_down",
                "Down",
                "button",
                "plain",
                "settings_group_move_down",
                78,
                28
            ),
        ]
    );
    assert_eq!(
        specs.map(|spec| spec.action),
        REQUIRED_NATIVE_HOST_SETTINGS_GROUP_ACTIONS
    );
    let erased_specs = native_host_settings_group_component_specs();
    assert_eq!(
        erased_specs.map(|spec| match spec.action {
            NativeComponentAction::SettingsGroup(action) => action,
            _ => panic!("settings group compatibility specs must erase group actions"),
        }),
        REQUIRED_NATIVE_HOST_SETTINGS_GROUP_ACTIONS
    );
    assert_eq!(
        REQUIRED_NATIVE_HOST_SETTINGS_GROUP_ACTIONS[0].target_category(),
        Some(0)
    );
    assert_eq!(
        REQUIRED_NATIVE_HOST_SETTINGS_GROUP_ACTIONS[1].target_category(),
        Some(1)
    );
    assert_eq!(
        REQUIRED_NATIVE_HOST_SETTINGS_GROUP_ACTIONS[5].move_step(),
        Some(-1)
    );
    assert_eq!(
        REQUIRED_NATIVE_HOST_SETTINGS_GROUP_ACTIONS[6].move_step(),
        Some(1)
    );
}

#[test]
fn native_host_edit_text_components_describe_reusable_native_buttons() {
    let specs = native_host_edit_text_button_specs();
    let summary: Vec<_> = specs
        .iter()
        .map(|spec| {
            (
                spec.id,
                spec.label,
                NativeComponentKind::Button.role_name(),
                spec.style_role.role_name(),
                spec.action.action_name(),
                spec.width(),
                spec.height(),
            )
        })
        .collect();

    assert_eq!(
        summary,
        vec![
            (
                "edit.save",
                "Save",
                "button",
                "suggested",
                "edit_text_save",
                96,
                32
            ),
            (
                "edit.cancel",
                "Cancel",
                "button",
                "plain",
                "edit_text_cancel",
                96,
                32
            )
        ]
    );
    assert_eq!(
        specs.map(|spec| spec.action),
        REQUIRED_NATIVE_HOST_EDIT_TEXT_ACTIONS
    );

    let erased_specs = native_host_edit_text_component_specs();
    assert_eq!(
        erased_specs.map(|spec| match spec.action {
            NativeComponentAction::EditText(action) => action,
            _ => panic!("edit text compatibility specs must erase edit text actions"),
        }),
        REQUIRED_NATIVE_HOST_EDIT_TEXT_ACTIONS
    );
}

#[cfg(feature = "vv-paste")]
#[test]
fn native_host_vv_select_specs_describe_dynamic_native_buttons() {
    let plan = native_host_vv_popup_render_plan();
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
    let specs = native_host_vv_select_specs(&plan, width, height);
    let summary: Vec<_> = specs
        .iter()
        .map(|spec| {
            (
                spec.id.as_str(),
                spec.label.as_str(),
                spec.action.action_name(),
                spec.width(),
                spec.height(),
            )
        })
        .collect();

    assert_eq!(
        summary,
        vec![
            ("vv.select.1", "Select 1", "vv_select", 84, 24),
            ("vv.select.2", "Select 2", "vv_select", 84, 24),
            ("vv.select.3", "Select 3", "vv_select", 84, 24),
            ("vv.select.4", "Select 4", "vv_select", 84, 24),
        ]
    );
    for (index, spec) in specs.iter().enumerate() {
        let action = spec.action;
        assert_eq!(action.index, index);
        assert_eq!(action.event(), native_host_vv_select_event(index));
    }
}

#[cfg(feature = "vv-paste")]
#[test]
fn native_host_vv_select_component_specs_remain_erased_compatibility_layer() {
    let plan = native_host_vv_popup_render_plan();
    let specs = native_host_vv_select_component_specs(&plan, 480, 240);

    assert!(!specs.is_empty());
    assert_eq!(specs[0].kind, NativeComponentKind::Button);
    let NativeComponentAction::VvSelect(action) = specs[0].action else {
        panic!("erased VV select component must still carry VV select action");
    };
    assert_eq!(action.index, 0);
}

#[test]
fn native_host_dialog_actions_have_expected_labels_and_messages() {
    let cases = [
        (
            NativeHostDialogAction::ShowInfoMessage,
            "dialog_show_info_message",
            "Info Dialog",
            "ZSClip",
            "This message is presented by the platform native dialog host.",
        ),
        (
            NativeHostDialogAction::ConfirmQuestion,
            "dialog_confirm_question",
            "Confirm",
            "Confirm Native Dialog",
            "Route this confirmation through the native dialog host?",
        ),
    ];

    for (action, action_name, button_label, title, message) in cases {
        assert_eq!(action.action_name(), action_name);
        assert_eq!(action.button_label(), button_label);
        assert_eq!(action.title(), title);
        assert_eq!(action.message(), message);
    }
}

#[test]
fn native_host_dialog_components_describe_reusable_native_buttons() {
    let specs = native_host_dialog_button_specs();
    let summary: Vec<_> = specs
        .iter()
        .map(|spec| {
            (
                spec.id,
                spec.label,
                NativeComponentKind::Button.role_name(),
                spec.action.action_name(),
                spec.width(),
                spec.height(),
            )
        })
        .collect();

    assert_eq!(
        summary,
        vec![
            (
                "settings.dialog.info",
                "Info Dialog",
                "button",
                "dialog_show_info_message",
                132,
                32
            ),
            (
                "settings.dialog.confirm",
                "Confirm",
                "button",
                "dialog_confirm_question",
                132,
                32
            ),
        ]
    );
    assert_eq!(
        specs.map(|spec| spec.action),
        REQUIRED_NATIVE_HOST_DIALOG_ACTIONS
    );

    let erased_specs = native_host_dialog_component_specs();
    assert_eq!(
        erased_specs.map(|spec| match spec.action {
            NativeComponentAction::Dialog(action) => action,
            _ => panic!("dialog compatibility specs must erase dialog actions"),
        }),
        REQUIRED_NATIVE_HOST_DIALOG_ACTIONS
    );
}

#[test]
fn native_host_row_actions_map_to_existing_menu_commands() {
    let cases = [
        (
            NativeHostRowAction::Paste,
            "row_paste",
            "Paste",
            menu_ids::ROW_PASTE,
        ),
        (
            NativeHostRowAction::Copy,
            "row_copy",
            "Copy",
            menu_ids::ROW_COPY,
        ),
        (
            NativeHostRowAction::Pin,
            "row_toggle_pin",
            "Pin",
            menu_ids::ROW_PIN,
        ),
        (
            NativeHostRowAction::ToPhrase,
            "row_to_phrase",
            "To Phrase",
            menu_ids::ROW_TO_PHRASE,
        ),
        (
            NativeHostRowAction::Delete,
            "row_delete",
            "Delete",
            menu_ids::ROW_DELETE,
        ),
        (
            NativeHostRowAction::Edit,
            "row_edit",
            "Edit",
            menu_ids::ROW_EDIT,
        ),
        (
            NativeHostRowAction::OpenPath,
            "row_open_path",
            "Open Path",
            menu_ids::ROW_OPEN_PATH,
        ),
        (
            NativeHostRowAction::OpenFolder,
            "row_open_folder",
            "Open Folder",
            menu_ids::ROW_OPEN_FOLDER,
        ),
        (
            NativeHostRowAction::CopyPath,
            "row_copy_path",
            "Copy Path",
            menu_ids::ROW_COPY_PATH,
        ),
        #[cfg(feature = "ai-actions")]
        (
            NativeHostRowAction::TextTranslate,
            "row_text_translate",
            "Translate",
            menu_ids::ROW_TEXT_TRANSLATE,
        ),
    ];

    for (action, action_name, button_label, menu_id) in cases {
        let command = action.command();
        assert_eq!(action.action_name(), action_name);
        assert_eq!(action.button_label(), button_label);
        assert_eq!(action.menu_id(), menu_id);
        assert_eq!(command.id, command_ids::INVOKE_MAIN_MENU_COMMAND);
        assert_eq!(command.payload, CommandPayload::ControlId(menu_id as i64));
    }
}

#[test]
fn native_host_row_popup_entries_round_trip_to_row_actions() {
    let entries = native_host_row_popup_menu_entries();
    let actions = native_host_row_actions();

    assert_eq!(entries.len(), actions.len());
    for (entry, action) in entries.iter().zip(actions) {
        let NativePopupMenuEntry::Command {
            id,
            label,
            enabled,
            checked,
        } = entry
        else {
            panic!("row popup entries should be flat commands");
        };

        assert_eq!(NativeHostRowAction::from_menu_id(*id), Some(action));
        assert_eq!(label, action.button_label());
        assert!(enabled);
        assert!(!checked);
    }

    assert_eq!(
        native_popup_menu_command_icon_name(menu_ids::ROW_COPY),
        Some("edit-copy-symbolic")
    );
    assert_eq!(
        native_popup_menu_command_accelerator_label(menu_ids::ROW_COPY),
        Some("Ctrl+C")
    );
    assert_eq!(
        native_popup_menu_command_macos_symbol_name(menu_ids::ROW_DELETE),
        Some("trash")
    );
    assert_eq!(
        native_popup_menu_command_macos_key_equivalent(menu_ids::ROW_DELETE),
        Some("\u{8}")
    );
}
#[test]
fn native_host_edit_text_plan_targets_first_projected_item() {
    let items = vec![NativeHostClipListItemProjection::new(
        42,
        "Record",
        "hello from projected history",
    )];
    let plan = native_host_edit_text_plan(&items).expect("projected item should be editable");

    assert_eq!(plan.item_id, 42);
    assert_eq!(plan.initial_text, "hello from projected history");
    assert_eq!(plan.title, "编辑 - hello from projected history");
    assert!(native_host_edit_text_plan(&[]).is_none());
}

#[test]
fn native_host_edit_text_plan_can_target_selected_projected_item() {
    let items = vec![
        NativeHostClipListItemProjection::new(1, "First", "first text"),
        NativeHostClipListItemProjection::new(2, "Second", "second text"),
    ];
    let plan = native_host_edit_text_plan_for_item(&items, Some(2))
        .expect("selected projected item should be editable");

    assert_eq!(plan.item_id, 2);
    assert_eq!(plan.initial_text, "second text");
    assert_eq!(
        native_host_edit_text_plan_for_item(&items, Some(999))
            .unwrap()
            .item_id,
        1
    );
}

#[test]
fn native_host_full_row_popup_entries_route_through_menu_commands() {
    fn assert_entries_route(entries: &[NativePopupMenuEntry]) {
        for entry in entries {
            match entry {
                NativePopupMenuEntry::Command { id, enabled, .. } if *enabled => {
                    assert!(
                        main_menu_command_for_id(*id).is_some(),
                        "enabled native popup id {} must route to a product command",
                        id
                    );
                }
                NativePopupMenuEntry::Submenu {
                    enabled: true,
                    entries,
                    ..
                } => assert_entries_route(entries),
                _ => {}
            }
        }
    }

    let entries = native_host_full_row_popup_menu_entries();

    assert!(entries.iter().any(|entry| {
        matches!(
            entry,
            NativePopupMenuEntry::Submenu {
                label,
                enabled: true,
                entries,
            } if label == "添加到分组"
                && entries.iter().any(|entry| matches!(
                    entry,
                    NativePopupMenuEntry::Command {
                        id: menu_ids::ROW_GROUP_BASE,
                        enabled: true,
                        ..
                    }
                ))
        )
    }));
    assert_entries_route(&entries);
}

#[test]
fn native_host_row_popup_entries_accept_real_group_list() {
    let entries = native_host_full_row_popup_menu_entries_for_groups(
        &[
            ClipGroup {
                id: 10,
                category: 0,
                name: "客户资料".to_string(),
            },
            ClipGroup {
                id: 20,
                category: 0,
                name: "常用短语".to_string(),
            },
        ],
        NativeHostRowPopupMenuInput::demo(),
        |label| label.to_string(),
    );

    let group_submenu = entries.iter().find_map(|entry| match entry {
        NativePopupMenuEntry::Submenu { label, entries, .. } if label == "添加到分组" => {
            Some(entries)
        }
        _ => None,
    });
    let group_submenu = group_submenu.expect("row menu should expose Add To Group submenu");
    assert!(matches!(
        group_submenu.first(),
        Some(NativePopupMenuEntry::Command { label, .. }) if label == "客户资料"
    ));
    assert!(matches!(
        group_submenu.get(1),
        Some(NativePopupMenuEntry::Command { label, .. }) if label == "常用短语"
    ));
}

#[test]
fn native_host_row_popup_input_uses_projected_item_metadata() {
    let items = vec![
        NativeHostClipListItemProjection::with_metadata(
            7,
            "Image",
            "screenshot.png",
            ClipKind::Image,
            true,
        ),
        NativeHostClipListItemProjection::with_metadata(
            9,
            "File",
            "report.xlsx",
            ClipKind::Files,
            false,
        ),
    ];

    let image_input = native_host_row_popup_menu_input_for_projection(&items, 7, true);
    assert_eq!(image_input.menu.selected_count, 1);
    assert_eq!(image_input.menu.current_kind, ClipKind::Image);
    assert!(!image_input.menu.has_unpinned);
    assert!(image_input.menu.current_can_ocr);
    assert!(!image_input.menu.current_can_translate);
    assert_eq!(image_input.labels.has_unpinned, false);

    let file_input = native_host_row_popup_menu_input_for_projection(&items, 9, true);
    assert_eq!(file_input.menu.current_kind, ClipKind::Files);
    assert!(file_input.menu.has_unpinned);
    assert!(file_input.menu.current_is_excel);
    assert!(file_input.menu.lan_push_available);
    assert!(file_input.menu.super_mail_merge_enabled);

    let fallback_input = native_host_row_popup_menu_input_for_projection(&items, 404, false);
    assert_eq!(fallback_input.menu.current_kind, ClipKind::Image);
    assert!(!fallback_input.menu.grouping_enabled);
}

#[test]
fn native_host_group_filter_popup_entries_route_through_group_filter_commands() {
    let entries = native_host_group_filter_popup_menu_entries();

    assert_eq!(
        entries.first(),
        Some(&NativePopupMenuEntry::Command {
            id: menu_ids::GROUP_FILTER_ALL,
            label: "All".to_string(),
            enabled: true,
            checked: false,
        })
    );
    assert!(entries.iter().any(|entry| {
        matches!(
            entry,
            NativePopupMenuEntry::Command {
                id,
                label,
                enabled: true,
                checked: true,
            } if *id == menu_ids::GROUP_FILTER_BASE + 1 && label == "Phrase Bank"
        )
    }));
    assert_eq!(
        main_menu_command_intent_for_id(menu_ids::GROUP_FILTER_BASE + 1),
        Some(MainMenuCommandIntent::GroupFilter { index: 1 })
    );
    for entry in entries {
        if let NativePopupMenuEntry::Command {
            id, enabled: true, ..
        } = entry
        {
            assert!(main_menu_command_for_id(id).is_some());
        }
    }
}

#[test]
fn native_host_group_filter_entries_accept_real_group_list_and_checked_id() {
    let entries = native_host_group_filter_popup_menu_entries_for_groups(
        &[
            ClipGroup {
                id: 10,
                category: 0,
                name: "客户资料".to_string(),
            },
            ClipGroup {
                id: 20,
                category: 0,
                name: "常用短语".to_string(),
            },
        ],
        20,
    );

    assert!(matches!(
        entries.get(3),
        Some(NativePopupMenuEntry::Command {
            label,
            checked: true,
            ..
        }) if label == "常用短语"
    ));
}

#[test]
fn native_host_group_filter_label_prefers_current_group_name() {
    let groups = [
        ClipGroup {
            id: 7,
            category: 0,
            name: "Work".to_string(),
        },
        ClipGroup {
            id: 8,
            category: 0,
            name: "Phrase Bank".to_string(),
        },
    ];

    assert_eq!(native_host_group_filter_label_for_groups(&groups, 0), "All");
    assert_eq!(
        native_host_group_filter_label_for_groups(&groups, 8),
        "Phrase Bank"
    );
    assert_eq!(
        native_host_group_filter_label_for_groups(&groups, 99),
        "All"
    );
}

#[test]
fn native_host_vv_popup_render_plan_uses_shared_layout_roles() {
    let plan = native_host_vv_popup_render_plan();
    let roles = plan
        .text_commands
        .iter()
        .map(|command| command.role)
        .collect::<Vec<_>>();

    assert!(roles.contains(&MainVvPopupTextRole::Title));
    assert!(roles.contains(&MainVvPopupTextRole::Hint));
    assert!(roles.contains(&MainVvPopupTextRole::GroupName));
    assert_eq!(
        roles
            .iter()
            .filter(|role| **role == MainVvPopupTextRole::RowIndex)
            .count(),
        native_host_default_clip_list_projection().len()
    );
    assert_eq!(
        roles
            .iter()
            .filter(|role| **role == MainVvPopupTextRole::RowPreview)
            .count(),
        native_host_default_clip_list_projection().len()
    );
    assert!(!plan.paint_commands.is_empty());
}

#[test]
fn native_host_vv_popup_render_plan_uses_projected_clip_items() {
    let items = vec![
        NativeHostClipListItemProjection::with_metadata(
            1,
            "Screenshot",
            "capture.png",
            ClipKind::Image,
            true,
        ),
        NativeHostClipListItemProjection::with_metadata(
            2,
            "Files",
            "report.xlsx",
            ClipKind::Files,
            false,
        ),
    ];
    let plan = native_host_vv_popup_render_plan_for_projection(&items, "Work");
    let row_previews = plan
        .text_commands
        .iter()
        .filter(|command| command.role == MainVvPopupTextRole::RowPreview)
        .map(|command| command.text.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        row_previews,
        vec![
            "[PIN] [IMG] Screenshot - capture.png",
            "[FILE] Files - report.xlsx"
        ]
    );
    assert!(plan.text_commands.iter().any(|command| {
        command.role == MainVvPopupTextRole::GroupName && command.text == "Work"
    }));

    let many_items = (0..12)
        .map(|index| NativeHostClipListItemProjection::new(index, "Item", format!("row {index}")))
        .collect::<Vec<_>>();
    let capped = native_host_vv_popup_render_plan_for_projection(&many_items, "All");
    assert_eq!(
        capped
            .text_commands
            .iter()
            .filter(|command| command.role == MainVvPopupTextRole::RowPreview)
            .count(),
        9
    );
}

#[cfg(feature = "vv-paste")]
#[test]
fn native_host_vv_select_event_reuses_shared_application_event() {
    assert_eq!(
        native_host_vv_select_event(2),
        ApplicationEvent::VvSelectRequested { index: 2 }
    );
}

#[test]
fn native_host_vv_trigger_state_detects_double_v_and_popup_keys() {
    let mut state = NativeHostVvTriggerState::default();
    let base = NativeHostVvTriggerInput {
        key: NativeHostVvTriggerKey::TriggerV,
        target_token: 9,
        target_ready: true,
        command_modifier: false,
        popup_menu_active: false,
        now_ms: 100,
    };

    assert_eq!(
        state.apply(base),
        NativeHostVvTriggerTransition {
            action: NativeHostVvTriggerAction::Ignore,
            consume_key: false,
        }
    );
    assert_eq!(
        state.apply(NativeHostVvTriggerInput {
            now_ms: 250,
            ..base
        }),
        NativeHostVvTriggerTransition {
            action: NativeHostVvTriggerAction::Show { target_token: 9 },
            consume_key: false,
        }
    );
    assert_eq!(
        state.apply(NativeHostVvTriggerInput {
            key: NativeHostVvTriggerKey::Digit1To9(3),
            now_ms: 260,
            ..base
        }),
        NativeHostVvTriggerTransition {
            action: NativeHostVvTriggerAction::Select { index: 3 },
            consume_key: true,
        }
    );

    assert_eq!(
        state
            .apply(NativeHostVvTriggerInput {
                now_ms: 1000,
                ..base
            })
            .action,
        NativeHostVvTriggerAction::Ignore
    );
    assert_eq!(
        state
            .apply(NativeHostVvTriggerInput {
                now_ms: 1401,
                ..base
            })
            .action,
        NativeHostVvTriggerAction::Ignore
    );

    assert_eq!(
        state
            .apply(NativeHostVvTriggerInput {
                now_ms: 1500,
                ..base
            })
            .action,
        NativeHostVvTriggerAction::Show { target_token: 9 }
    );
    assert_eq!(
        state.apply(NativeHostVvTriggerInput {
            key: NativeHostVvTriggerKey::Escape,
            now_ms: 1510,
            ..base
        }),
        NativeHostVvTriggerTransition {
            action: NativeHostVvTriggerAction::Hide,
            consume_key: true,
        }
    );
}

#[test]
fn native_host_vv_trigger_state_rejects_wrong_targets_and_unready_inputs() {
    let mut state = NativeHostVvTriggerState::default();

    assert_eq!(
        state
            .apply(NativeHostVvTriggerInput {
                key: NativeHostVvTriggerKey::TriggerV,
                target_token: 10,
                target_ready: false,
                command_modifier: false,
                popup_menu_active: false,
                now_ms: 10,
            })
            .action,
        NativeHostVvTriggerAction::Ignore
    );
    state.mark_popup_active(10);
    assert_eq!(
        state.apply(NativeHostVvTriggerInput {
            key: NativeHostVvTriggerKey::Digit1To9(1),
            target_token: 11,
            target_ready: true,
            command_modifier: false,
            popup_menu_active: false,
            now_ms: 20,
        }),
        NativeHostVvTriggerTransition {
            action: NativeHostVvTriggerAction::Hide,
            consume_key: false,
        }
    );

    state.mark_popup_active(10);
    assert_eq!(
        state
            .apply(NativeHostVvTriggerInput {
                key: NativeHostVvTriggerKey::Digit1To9(1),
                target_token: 10,
                target_ready: true,
                command_modifier: false,
                popup_menu_active: true,
                now_ms: 30,
            })
            .action,
        NativeHostVvTriggerAction::Ignore
    );
}

#[test]
fn native_host_vv_paste_plan_prepares_clipboard_payloads() {
    let text = ClipItem {
        id: 41,
        kind: ClipKind::Text,
        preview: "hello".to_string(),
        text: Some("hello from vv".to_string()),
        source_app: "Notes".to_string(),
        file_paths: None,
        image_bytes: None,
        image_path: None,
        image_width: 0,
        image_height: 0,
        pinned: false,
        group_id: 0,
        created_at: String::new(),
    };
    let mut files = text.clone();
    files.id = 42;
    files.kind = ClipKind::Files;
    files.file_paths = Some(vec!["/tmp/a.txt".to_string(), "/tmp/b.txt".to_string()]);
    files.text = None;
    let mut image = text.clone();
    image.id = 43;
    image.kind = ClipKind::Image;
    image.image_bytes = Some(vec![255, 0, 0, 255]);
    image.image_width = 1;
    image.image_height = 1;
    image.text = None;

    let text_plan = native_host_vv_paste_plan(true, 0, &[text.clone()], 2).unwrap();
    assert_eq!(
        text_plan,
        NativeHostVvPastePlan::Paste(NativeHostVvPasteItem {
            item_id: 41,
            clipboard_write: NativeHostClipboardWrite::Text("hello from vv".to_string()),
            backspaces: 2,
        })
    );

    let files_plan = native_host_vv_paste_plan(true, 0, &[files], 0).unwrap();
    assert!(matches!(
        files_plan,
        NativeHostVvPastePlan::Paste(NativeHostVvPasteItem {
            clipboard_write: NativeHostClipboardWrite::FilePaths(_),
            ..
        })
    ));
    let image_plan = native_host_vv_paste_plan(true, 0, &[image], 0).unwrap();
    assert!(matches!(
        image_plan,
        NativeHostVvPastePlan::Paste(NativeHostVvPasteItem {
            clipboard_write: NativeHostClipboardWrite::ImageRgba { .. },
            ..
        })
    ));
    assert_eq!(
        native_host_vv_paste_plan(true, 3, &[], 0),
        Some(NativeHostVvPastePlan::HidePopup)
    );
    assert_eq!(native_host_vv_paste_plan(false, 0, &[text], 0), None);
}

#[test]
fn native_host_reconciles_selected_item_after_list_refresh() {
    let items = vec![
        NativeHostClipListItemProjection::new(7, "First", "first"),
        NativeHostClipListItemProjection::new(9, "Second", "second"),
    ];

    assert_eq!(native_host_reconciled_selected_item_id(9, &items), 9);
    assert_eq!(native_host_reconciled_selected_item_id(42, &items), 7);
    assert_eq!(native_host_reconciled_selected_item_id(7, &[]), 0);
}

#[test]
fn native_host_clip_list_filter_matches_title_and_preview() {
    let projected = native_host_default_clip_list_projection();

    assert_eq!(native_host_filtered_clip_item_ids(""), vec![1, 2, 3, 4]);
    assert_eq!(
        native_host_filtered_projected_clip_item_ids(&projected, ""),
        vec![1, 2, 3, 4]
    );
    assert_eq!(native_host_filtered_clip_item_ids("png"), vec![2]);
    assert_eq!(native_host_filtered_clip_item_ids("WEBDAV"), vec![4]);
    assert_eq!(native_host_filtered_clip_item_ids("report"), vec![3]);
    assert_eq!(
        native_host_clip_list_item_label(&REQUIRED_NATIVE_HOST_CLIP_LIST_ITEMS[0]),
        "Welcome text - ZSClip keeps clipboard history searchable."
    );
    assert_eq!(
        native_host_projected_clip_list_item_label(&projected[0]),
        "Welcome text - ZSClip keeps clipboard history searchable."
    );
}

#[test]
fn native_host_clip_projection_carries_kind_and_pinned_metadata() {
    let projected = NativeHostClipListItemProjection::with_metadata(
        42,
        "Screenshots",
        "PNG screenshot preview",
        ClipKind::Image,
        true,
    );
    let item = native_host_clip_item_from_projection(&projected);

    assert_eq!(projected.kind, ClipKind::Image);
    assert!(projected.pinned);
    assert_eq!(item.kind, ClipKind::Image);
    assert!(item.pinned);
    assert_eq!(item.source_app, "Screenshots");

    let fallback = NativeHostClipListItemProjection::new(7, "Notes", "plain text");
    assert_eq!(fallback.kind, ClipKind::Text);
    assert!(!fallback.pinned);
}

#[test]
fn native_host_projected_clip_row_title_exposes_kind_and_pin_state() {
    let image = NativeHostClipListItemProjection::with_metadata(
        42,
        "Screenshot",
        "capture.png",
        ClipKind::Image,
        true,
    );
    assert_eq!(
        native_host_projected_clip_row_title(&image),
        "[PIN] [IMG] Screenshot - capture.png"
    );

    let file = NativeHostClipListItemProjection::with_metadata(
        7,
        "Files",
        "report.xlsx",
        ClipKind::Files,
        false,
    );
    assert_eq!(
        native_host_projected_clip_row_title(&file),
        "[FILE] Files - report.xlsx"
    );
    assert_eq!(
        native_host_projected_clip_kind_prefix(ClipKind::Phrase),
        "[PHRASE]"
    );
}

#[test]
fn native_host_clip_row_presentation_is_platform_neutral() {
    let item = NativeHostClipListItemProjection::with_metadata(
        42,
        "Screenshot",
        "capture.png",
        ClipKind::Image,
        true,
    );
    let presentation = native_host_clip_row_presentation_for_projection(&item);

    assert_eq!(presentation.item_id, 42);
    assert_eq!(presentation.title, "Screenshot");
    assert_eq!(presentation.preview, "capture.png");
    assert_eq!(
        presentation.compact_label,
        "[PIN] [IMG] Screenshot - capture.png"
    );
    assert_eq!(presentation.kind_prefix, "[IMG]");
    assert_eq!(presentation.kind_icon, NativeHostClipKindIcon::Image);
    assert_eq!(presentation.kind_icon.semantic_name(), "image");
    assert_eq!(presentation.pin_badge, Some("PIN"));
    assert!(presentation
        .accessibility_label
        .contains("Pinned image clipboard item"));

    let text = native_host_clip_row_presentation_for_projection(
        &NativeHostClipListItemProjection::new(7, "Notes", "plain text"),
    );
    assert_eq!(text.kind_icon, NativeHostClipKindIcon::Text);
    assert_eq!(text.pin_badge, None);

    let folder = ClipItem {
        id: 9,
        kind: ClipKind::Files,
        preview: "C:\\Users\\Public".to_string(),
        text: None,
        source_app: "Explorer".to_string(),
        file_paths: Some(vec!["C:\\Users\\Public".to_string()]),
        image_bytes: None,
        image_path: None,
        image_width: 0,
        image_height: 0,
        pinned: true,
        group_id: 0,
        created_at: "2026-06-19 00:00:00".to_string(),
    };
    let folder_presentation = native_host_clip_row_presentation_for_clip_item(&folder, true);
    assert_eq!(
        folder_presentation.kind_icon,
        NativeHostClipKindIcon::Folder
    );
    assert_eq!(folder_presentation.kind_icon.semantic_name(), "folder");
    assert_eq!(folder_presentation.pin_badge, Some("PIN"));
}

#[test]
fn main_shortcuts_map_keys_to_platform_neutral_actions() {
    let none = ShortcutModifiers::default();
    let shift = ShortcutModifiers {
        shift: true,
        ..ShortcutModifiers::default()
    };
    let ctrl = ShortcutModifiers {
        ctrl: true,
        ..ShortcutModifiers::default()
    };
    let alt = ShortcutModifiers {
        alt: true,
        ..ShortcutModifiers::default()
    };

    assert_eq!(
        main_shortcut_action(ShortcutKey::Up, none),
        Some(MainShortcutAction::MoveSelection {
            delta: -1,
            extend: false
        })
    );
    assert_eq!(
        main_shortcut_action(ShortcutKey::Down, shift),
        Some(MainShortcutAction::MoveSelection {
            delta: 1,
            extend: true
        })
    );
    assert_eq!(
        main_shortcut_action(ShortcutKey::Enter, none),
        Some(MainShortcutAction::ActivateSelection)
    );
    assert_eq!(
        main_shortcut_action(ShortcutKey::A, ctrl),
        Some(MainShortcutAction::SelectAll)
    );
    assert_eq!(
        main_shortcut_action(ShortcutKey::C, ctrl),
        Some(MainShortcutAction::CopySelection)
    );
    assert_eq!(
        main_shortcut_action(ShortcutKey::Delete, none),
        Some(MainShortcutAction::DeleteSelection)
    );
    assert_eq!(
        main_shortcut_action(ShortcutKey::Escape, none),
        Some(MainShortcutAction::Escape)
    );
    assert_eq!(
        main_shortcut_action(ShortcutKey::P, ctrl),
        Some(MainShortcutAction::TogglePin)
    );
    assert_eq!(
        main_shortcut_action(ShortcutKey::F, ctrl),
        Some(MainShortcutAction::ToggleSearch)
    );
    assert_eq!(main_shortcut_action(ShortcutKey::F, alt), None);
    assert_eq!(main_shortcut_action(ShortcutKey::Other(0), none), None);
}

#[test]
fn main_shortcut_row_commands_map_to_stable_window_commands() {
    let cases = [
        (
            MainShortcutAction::CopySelection,
            MainShortcutRowCommand::CopySelection,
            menu_ids::ROW_COPY,
        ),
        (
            MainShortcutAction::DeleteSelection,
            MainShortcutRowCommand::DeleteSelection,
            menu_ids::ROW_DELETE,
        ),
        (
            MainShortcutAction::TogglePin,
            MainShortcutRowCommand::TogglePin,
            menu_ids::ROW_PIN,
        ),
    ];

    for (action, row_command, menu_id) in cases {
        assert_eq!(
            main_shortcut_row_command_for_action(action),
            Some(row_command)
        );
        assert_eq!(
            main_menu_command_for_shortcut_row_command(row_command),
            Command::window_with_payload(
                command_ids::INVOKE_MAIN_MENU_COMMAND,
                CommandPayload::ControlId(menu_id as i64)
            )
        );
    }

    assert_eq!(
        main_shortcut_row_command_for_action(MainShortcutAction::ActivateSelection),
        None
    );
    assert_eq!(
        main_shortcut_row_command_for_action(MainShortcutAction::ToggleSearch),
        None
    );
}

#[test]
fn main_shortcut_window_commands_map_to_stable_window_commands() {
    assert_eq!(
        main_shortcut_window_command_for_action(MainShortcutAction::ToggleSearch),
        Some(MainWindowCommandIntent::ToggleSearch)
    );
    assert_eq!(
        main_window_command_for_intent(MainWindowCommandIntent::ToggleSearch),
        Command::window(command_ids::TOGGLE_SEARCH)
    );
    assert_eq!(
        main_window_command_for_intent(MainWindowCommandIntent::HideWindow),
        Command::window(command_ids::HIDE_WINDOW)
    );

    assert_eq!(
        main_shortcut_window_command_for_action(MainShortcutAction::CopySelection),
        None
    );
    assert_eq!(
        main_shortcut_window_command_for_action(MainShortcutAction::Escape),
        None
    );
}

#[test]
fn shortcut_execution_plan_routes_actions_without_platform_state() {
    assert_eq!(
        main_shortcut_execution_plan(
            MainShortcutAction::MoveSelection {
                delta: 1,
                extend: true,
            },
            None,
        ),
        MainShortcutExecutionPlan::MoveSelection {
            delta: 1,
            extend: true,
        }
    );

    assert_eq!(
        main_shortcut_execution_plan(MainShortcutAction::TogglePin, None),
        MainShortcutExecutionPlan::RowCommand(MainShortcutRowCommand::TogglePin)
    );

    assert_eq!(
        main_shortcut_execution_plan(
            MainShortcutAction::Escape,
            Some(MainShortcutEscapePlan::CloseSearch),
        ),
        MainShortcutExecutionPlan::CloseSearch
    );

    assert_eq!(
        main_shortcut_execution_plan(
            MainShortcutAction::Escape,
            Some(MainShortcutEscapePlan::HideWindow),
        ),
        MainShortcutExecutionPlan::WindowCommand(MainWindowCommandIntent::HideWindow)
    );

    assert_eq!(
        main_shortcut_execution_plan(MainShortcutAction::ToggleSearch, None),
        MainShortcutExecutionPlan::WindowCommand(MainWindowCommandIntent::ToggleSearch)
    );
}

#[test]
fn main_title_buttons_map_to_stable_window_command_intents() {
    let cases = [
        (
            "search",
            MainWindowCommandIntent::ToggleSearch,
            command_ids::TOGGLE_SEARCH,
        ),
        (
            "setting",
            MainWindowCommandIntent::OpenSettings,
            command_ids::OPEN_SETTINGS,
        ),
        (
            "min",
            MainWindowCommandIntent::HideWindow,
            command_ids::HIDE_WINDOW,
        ),
        (
            "close",
            MainWindowCommandIntent::CloseWindow,
            command_ids::CLOSE_WINDOW,
        ),
        (
            "unknown",
            MainWindowCommandIntent::CloseWindow,
            command_ids::CLOSE_WINDOW,
        ),
    ];

    for (key, intent, command_id) in cases {
        assert_eq!(main_title_button_window_command_for_key(key), intent);
        assert_eq!(
            main_window_command_for_intent(intent),
            Command::window(command_id)
        );
    }
}

#[test]
fn window_commands_have_stable_namespaced_ids() {
    let commands = [
        command_ids::TOGGLE_SEARCH,
        command_ids::UPDATE_SEARCH_TEXT,
        command_ids::INVOKE_MAIN_MENU_COMMAND,
        command_ids::OPEN_SETTINGS,
        command_ids::SAVE_SETTINGS,
        command_ids::CLOSE_SETTINGS,
        command_ids::OPEN_SETTINGS_CONFIG,
        command_ids::OPEN_SETTINGS_DROPDOWN,
        command_ids::TOGGLE_SETTINGS_CONTROL,
        command_ids::HIDE_WINDOW,
        command_ids::CLOSE_WINDOW,
    ];
    let mut names = commands.map(|id| id.0).to_vec();
    names.sort_unstable();
    names.dedup();

    assert_eq!(names.len(), commands.len());
    assert!(names.iter().all(|name| name.starts_with("window.")));
    assert_eq!(
        Command::window(command_ids::TOGGLE_SEARCH).scope,
        CommandScope::Window
    );
}

#[test]
fn main_window_commands_map_to_platform_neutral_host_actions() {
    assert_eq!(
        main_host_action_for_command(&Command::window(command_ids::TOGGLE_SEARCH)),
        Some(MainHostAction::ToggleSearch)
    );
    assert_eq!(
        main_host_action_for_command(&Command::window(command_ids::OPEN_SETTINGS)),
        Some(MainHostAction::OpenSettings)
    );
    assert_eq!(
        main_host_action_for_command(&Command::window(command_ids::HIDE_WINDOW)),
        Some(MainHostAction::HideWindow)
    );
    assert_eq!(
        main_host_action_for_command(&Command::window(command_ids::CLOSE_WINDOW)),
        Some(MainHostAction::CloseWindow)
    );
    assert_eq!(
        main_host_action_for_command(&Command::window_with_payload(
            command_ids::INVOKE_MAIN_MENU_COMMAND,
            CommandPayload::ControlId(41001)
        )),
        Some(MainHostAction::InvokeMenuCommand(
            MainMenuCommandIntent::RowPaste
        ))
    );
    assert_eq!(
        main_host_action_for_command(&Command::window_with_payload(
            command_ids::INVOKE_MAIN_MENU_COMMAND,
            CommandPayload::ControlId(-1)
        )),
        None
    );
    assert_eq!(
        main_host_action_for_command(&Command {
            id: command_ids::TOGGLE_SEARCH,
            scope: CommandScope::Component(ComponentId(7)),
            payload: CommandPayload::None,
        }),
        None
    );
}

#[test]
fn host_actions_map_to_platform_execution_plans() {
    assert_eq!(
        main_host_execution_plan(MainHostAction::ToggleSearch),
        MainHostExecutionPlan::Search(MainSearchVisibilityRequest::Toggle)
    );
    assert_eq!(
        main_host_execution_plan(MainHostAction::OpenSettings),
        MainHostExecutionPlan::OpenSettings
    );
    assert_eq!(
        main_host_execution_plan(MainHostAction::HideWindow),
        MainHostExecutionPlan::HideWindow
    );
    assert_eq!(
        main_host_execution_plan(MainHostAction::CloseWindow),
        MainHostExecutionPlan::CloseWindow
    );
    assert_eq!(
        main_host_execution_plan(MainHostAction::InvokeMenuCommand(
            MainMenuCommandIntent::RowPaste
        )),
        MainHostExecutionPlan::InvokeMenuCommand(MainMenuCommandIntent::RowPaste)
    );
}

#[test]
fn main_host_execution_plan_kinds_are_explicit_porting_contract() {
    assert_eq!(
        REQUIRED_MAIN_HOST_EXECUTION_PLAN_KINDS,
        [
            MainHostExecutionPlanKind::Search,
            MainHostExecutionPlanKind::OpenSettings,
            MainHostExecutionPlanKind::HideWindow,
            MainHostExecutionPlanKind::CloseWindow,
            MainHostExecutionPlanKind::InvokeMenuCommand,
        ]
    );

    let plans = [
        main_host_execution_plan(MainHostAction::ToggleSearch),
        main_host_execution_plan(MainHostAction::OpenSettings),
        main_host_execution_plan(MainHostAction::HideWindow),
        main_host_execution_plan(MainHostAction::CloseWindow),
        main_host_execution_plan(MainHostAction::InvokeMenuCommand(
            MainMenuCommandIntent::RowPaste,
        )),
    ];
    let kinds: Vec<_> = plans.into_iter().map(MainHostExecutionPlan::kind).collect();

    assert_eq!(kinds, REQUIRED_MAIN_HOST_EXECUTION_PLAN_KINDS);
    assert!(REQUIRED_MAIN_HOST_EXECUTION_PLAN_KINDS
        .iter()
        .all(|kind| !kind.plan_name().is_empty()));
}

#[test]
fn menu_ids_map_to_platform_neutral_window_commands() {
    for id in [
        menu_ids::TRAY_TOGGLE,
        menu_ids::TRAY_LAN_TOGGLE,
        menu_ids::TRAY_CAPTURE_TOGGLE,
        menu_ids::TRAY_EXIT,
        menu_ids::ROW_PASTE,
        menu_ids::ROW_COPY,
        menu_ids::ROW_PIN,
        menu_ids::ROW_DELETE,
        menu_ids::ROW_DELETE_UNPINNED,
        menu_ids::ROW_TO_PHRASE,
        menu_ids::ROW_STICKER,
        menu_ids::ROW_SAVE_IMAGE,
        menu_ids::ROW_IMAGE_OCR,
        menu_ids::ROW_TEXT_TRANSLATE,
        menu_ids::ROW_QR_IMAGE,
        menu_ids::ROW_LAN_PUSH,
        menu_ids::ROW_OPEN_PATH,
        menu_ids::ROW_OPEN_FOLDER,
        menu_ids::ROW_COPY_PATH,
        menu_ids::ROW_GROUP_REMOVE,
        menu_ids::ROW_EDIT,
        menu_ids::ROW_QUICK_SEARCH,
        menu_ids::ROW_EXPORT_FILE,
        menu_ids::ROW_MAIL_MERGE,
        menu_ids::GROUP_FILTER_ALL,
        menu_ids::ROW_GROUP_BASE,
        menu_ids::ROW_GROUP_BASE + menu_ids::DYNAMIC_GROUP_LIMIT - 1,
        menu_ids::GROUP_FILTER_BASE,
        menu_ids::GROUP_FILTER_BASE + menu_ids::DYNAMIC_GROUP_LIMIT - 1,
    ] {
        assert_eq!(
            main_menu_command_for_id(id),
            Some(Command::window_with_payload(
                command_ids::INVOKE_MAIN_MENU_COMMAND,
                CommandPayload::ControlId(id as i64)
            ))
        );
    }

    assert_eq!(main_menu_command_for_id(1001), None);
    assert_eq!(
        main_menu_command_for_id(menu_ids::GROUP_FILTER_BASE + menu_ids::DYNAMIC_GROUP_LIMIT),
        None
    );
}

#[test]
fn dynamic_row_group_and_group_filter_menu_ids_do_not_overlap() {
    let row_group_range =
        menu_ids::ROW_GROUP_BASE..menu_ids::ROW_GROUP_BASE + menu_ids::DYNAMIC_GROUP_LIMIT;
    let group_filter_range =
        menu_ids::GROUP_FILTER_BASE..menu_ids::GROUP_FILTER_BASE + menu_ids::DYNAMIC_GROUP_LIMIT;

    assert!(!row_group_range.contains(&menu_ids::GROUP_FILTER_ALL));
    assert!(row_group_range.end <= menu_ids::GROUP_FILTER_ALL);
    assert!(menu_ids::GROUP_FILTER_ALL < group_filter_range.start);
    assert!(row_group_range.end <= group_filter_range.start);
    assert_eq!(main_row_group_selection_for_id(row_group_range.end), None);
    assert_eq!(
        main_menu_command_intent_for_id(row_group_range.end),
        Some(MainMenuCommandIntent::GroupFilterAll)
    );
}

#[test]
fn row_menu_actions_map_to_stable_menu_ids() {
    let cases = [
        (MainRowMenuAction::Copy, menu_ids::ROW_COPY),
        (MainRowMenuAction::Pin, menu_ids::ROW_PIN),
        (MainRowMenuAction::ToPhrase, menu_ids::ROW_TO_PHRASE),
        (
            MainRowMenuAction::RemoveFromGroup,
            menu_ids::ROW_GROUP_REMOVE,
        ),
        (MainRowMenuAction::Delete, menu_ids::ROW_DELETE),
        (
            MainRowMenuAction::DeleteUnpinned,
            menu_ids::ROW_DELETE_UNPINNED,
        ),
        (MainRowMenuAction::Sticker, menu_ids::ROW_STICKER),
        (MainRowMenuAction::SaveImage, menu_ids::ROW_SAVE_IMAGE),
        (MainRowMenuAction::ImageOcr, menu_ids::ROW_IMAGE_OCR),
        (MainRowMenuAction::ExportFile, menu_ids::ROW_EXPORT_FILE),
        (MainRowMenuAction::OpenPath, menu_ids::ROW_OPEN_PATH),
        (MainRowMenuAction::OpenFolder, menu_ids::ROW_OPEN_FOLDER),
        (MainRowMenuAction::CopyPath, menu_ids::ROW_COPY_PATH),
        (MainRowMenuAction::QrImage, menu_ids::ROW_QR_IMAGE),
        (MainRowMenuAction::MailMerge, menu_ids::ROW_MAIL_MERGE),
        (MainRowMenuAction::LanPush, menu_ids::ROW_LAN_PUSH),
        (MainRowMenuAction::Edit, menu_ids::ROW_EDIT),
        (MainRowMenuAction::QuickSearch, menu_ids::ROW_QUICK_SEARCH),
        (
            MainRowMenuAction::TextTranslate,
            menu_ids::ROW_TEXT_TRANSLATE,
        ),
    ];

    for (action, id) in cases {
        assert_eq!(main_row_menu_action_id(action), id);
        assert_eq!(main_row_menu_action_for_id(id), Some(action));
    }

    assert_eq!(
        main_row_menu_action_id(MainRowMenuAction::AddToGroup),
        menu_ids::ROW_GROUP_BASE
    );
    assert_eq!(main_row_menu_action_for_id(menu_ids::ROW_GROUP_BASE), None);
    assert_eq!(main_row_menu_action_for_id(menu_ids::ROW_PASTE), None);
    assert_eq!(
        main_row_menu_action_for_id(menu_ids::ROW_GROUP_BASE + 1),
        None
    );
    assert_eq!(
        main_row_menu_action_for_id(menu_ids::GROUP_FILTER_ALL),
        None
    );
}

#[test]
fn menu_ids_parse_to_platform_neutral_menu_intents() {
    assert_eq!(
        main_menu_command_intent_for_id(menu_ids::TRAY_TOGGLE),
        Some(MainMenuCommandIntent::Tray(
            MainTrayMenuAction::ToggleWindow
        ))
    );
    assert_eq!(
        main_menu_command_intent_for_id(menu_ids::TRAY_CAPTURE_TOGGLE),
        Some(MainMenuCommandIntent::Tray(
            MainTrayMenuAction::ToggleClipboardCapture
        ))
    );
    assert_eq!(
        main_menu_command_intent_for_id(menu_ids::ROW_PASTE),
        Some(MainMenuCommandIntent::RowPaste)
    );
    assert_eq!(
        main_menu_command_intent_for_id(menu_ids::ROW_COPY),
        Some(MainMenuCommandIntent::RowAction(MainRowMenuAction::Copy))
    );
    assert_eq!(
        main_menu_command_intent_for_id(menu_ids::ROW_GROUP_BASE),
        Some(MainMenuCommandIntent::AssignRowGroup { index: 0 })
    );
    assert_eq!(
        main_menu_command_intent_for_id(menu_ids::ROW_GROUP_BASE + 12),
        Some(MainMenuCommandIntent::AssignRowGroup { index: 12 })
    );
    assert_eq!(
        main_menu_command_intent_for_id(menu_ids::GROUP_FILTER_ALL),
        Some(MainMenuCommandIntent::GroupFilterAll)
    );
    assert_eq!(
        main_menu_command_intent_for_id(menu_ids::GROUP_FILTER_BASE),
        Some(MainMenuCommandIntent::GroupFilter { index: 0 })
    );
    assert_eq!(
        main_menu_command_intent_for_id(
            menu_ids::GROUP_FILTER_BASE + menu_ids::DYNAMIC_GROUP_LIMIT
        ),
        None
    );
    assert_eq!(main_menu_command_intent_for_id(1001), None);
}

#[test]
fn group_filter_menu_ids_parse_in_group_filter_context() {
    assert_eq!(main_group_filter_menu_all_id(), menu_ids::GROUP_FILTER_ALL);
    assert_eq!(
        main_group_filter_menu_group_id(7),
        menu_ids::GROUP_FILTER_BASE + 7
    );
    assert_eq!(
        main_group_filter_selection_for_id(menu_ids::GROUP_FILTER_ALL),
        Some(MainGroupFilterSelection::All)
    );
    assert_eq!(
        main_group_filter_selection_for_id(menu_ids::GROUP_FILTER_BASE),
        Some(MainGroupFilterSelection::Group { index: 0 })
    );
    assert_eq!(
        main_group_filter_selection_for_id(menu_ids::GROUP_FILTER_BASE + 12),
        Some(MainGroupFilterSelection::Group { index: 12 })
    );
    assert_eq!(
        main_group_filter_selection_for_id(
            menu_ids::GROUP_FILTER_BASE + menu_ids::DYNAMIC_GROUP_LIMIT
        ),
        None
    );
    assert_eq!(
        main_group_filter_selection_for_id(menu_ids::ROW_GROUP_BASE),
        None
    );
}

#[test]
fn row_group_menu_ids_parse_in_row_group_context() {
    assert_eq!(main_row_group_menu_group_id(0), menu_ids::ROW_GROUP_BASE);
    assert_eq!(
        main_row_group_menu_group_id(8),
        menu_ids::ROW_GROUP_BASE + 8
    );
    assert_eq!(
        main_row_group_selection_for_id(menu_ids::ROW_GROUP_BASE),
        Some(MainRowGroupSelection::Group { index: 0 })
    );
    assert_eq!(
        main_row_group_selection_for_id(menu_ids::ROW_GROUP_BASE + 8),
        Some(MainRowGroupSelection::Group { index: 8 })
    );
    assert_eq!(
        main_row_group_selection_for_id(menu_ids::ROW_GROUP_BASE + menu_ids::DYNAMIC_GROUP_LIMIT),
        None
    );
    assert_eq!(
        main_row_group_selection_for_id(menu_ids::GROUP_FILTER_BASE - 1),
        None
    );
}

#[test]
fn main_row_popup_entries_reuse_row_plan_and_group_submenu() {
    let groups = vec![
        ClipGroup {
            id: 7,
            category: 0,
            name: "Work".to_string(),
        },
        ClipGroup {
            id: 9,
            category: 0,
            name: "Temp".to_string(),
        },
    ];
    let group_entries = main_row_group_popup_entries(&groups, "(none)");
    assert_eq!(
        group_entries,
        vec![
            NativePopupMenuEntry::Command {
                id: menu_ids::ROW_GROUP_BASE,
                label: "Work".to_string(),
                enabled: true,
                checked: false,
            },
            NativePopupMenuEntry::Command {
                id: menu_ids::ROW_GROUP_BASE + 1,
                label: "Temp".to_string(),
                enabled: true,
                checked: false,
            },
        ]
    );

    let plan = main_row_menu_plan(MainRowMenuInput {
        selected_count: 1,
        has_unpinned: false,
        current_kind: ClipKind::Text,
        grouping_enabled: true,
        current_can_ocr: false,
        current_can_translate: true,
        current_is_excel: false,
        quick_search_enabled: true,
        qr_quick_enabled: false,
        super_mail_merge_enabled: false,
        lan_push_available: false,
    });
    let entries = main_row_popup_menu_entries(
        &plan,
        MainRowMenuLabelInput {
            selected_count: 1,
            has_unpinned: false,
            current_is_dir: false,
        },
        true,
        group_entries,
        |label| format!("native:{}", label),
    );

    assert!(entries.iter().any(|entry| {
        matches!(
            entry,
            NativePopupMenuEntry::Submenu {
                label,
                enabled: true,
                entries,
            } if label == "native:添加到分组"
                && entries.len() == 2
                && matches!(
                    entries[0],
                    NativePopupMenuEntry::Command { id: menu_ids::ROW_GROUP_BASE, .. }
                )
        )
    }));
    assert!(entries.iter().any(|entry| {
        matches!(
            entry,
            NativePopupMenuEntry::Command {
                id: menu_ids::ROW_DELETE_UNPINNED,
                enabled: false,
                ..
            }
        )
    }));
}

#[test]
fn main_group_filter_popup_entries_are_shared_by_group_and_vv_menus() {
    let groups = vec![
        ClipGroup {
            id: 7,
            category: 0,
            name: "Work".to_string(),
        },
        ClipGroup {
            id: 9,
            category: 0,
            name: "Temp".to_string(),
        },
    ];
    let plan = main_group_filter_menu_plan(9, &groups);
    let entries = main_group_filter_popup_entries(&plan, "All records");

    assert_eq!(
        entries,
        vec![
            NativePopupMenuEntry::Command {
                id: menu_ids::GROUP_FILTER_ALL,
                label: "All records".to_string(),
                enabled: true,
                checked: false,
            },
            NativePopupMenuEntry::Separator,
            NativePopupMenuEntry::Command {
                id: menu_ids::GROUP_FILTER_BASE,
                label: "Work".to_string(),
                enabled: true,
                checked: false,
            },
            NativePopupMenuEntry::Command {
                id: menu_ids::GROUP_FILTER_BASE + 1,
                label: "Temp".to_string(),
                enabled: true,
                checked: true,
            },
        ]
    );
}

#[test]
fn tray_menu_plan_describes_status_items_without_host_menu() {
    assert_eq!(
        main_tray_menu_plan(MainTrayMenuInput {
            clipboard_capture_enabled: true,
            lan_sync_enabled: false,
        }),
        vec![
            MainTrayMenuItem::Command {
                action: MainTrayMenuAction::ToggleWindow,
                text: MainTrayMenuText::ToggleWindow,
            },
            MainTrayMenuItem::Command {
                action: MainTrayMenuAction::ToggleClipboardCapture,
                text: MainTrayMenuText::DisableClipboardCapture,
            },
            MainTrayMenuItem::Command {
                action: MainTrayMenuAction::ToggleLanSync,
                text: MainTrayMenuText::LanSyncOff,
            },
            MainTrayMenuItem::Separator,
            MainTrayMenuItem::Command {
                action: MainTrayMenuAction::Exit,
                text: MainTrayMenuText::Exit,
            },
        ]
    );

    assert_eq!(
        main_tray_menu_plan(MainTrayMenuInput {
            clipboard_capture_enabled: false,
            lan_sync_enabled: true,
        })[1],
        MainTrayMenuItem::Command {
            action: MainTrayMenuAction::ToggleClipboardCapture,
            text: MainTrayMenuText::EnableClipboardCapture,
        }
    );
    assert_eq!(
        main_tray_menu_plan(MainTrayMenuInput {
            clipboard_capture_enabled: false,
            lan_sync_enabled: true,
        })[2],
        MainTrayMenuItem::Command {
            action: MainTrayMenuAction::ToggleLanSync,
            text: MainTrayMenuText::LanSyncOn,
        }
    );
}

#[test]
fn tray_action_plan_toggles_state_without_host_side_effects() {
    assert_eq!(
        main_tray_action_plan(MainTrayActionInput {
            action: MainTrayMenuAction::ToggleWindow,
            clipboard_capture_enabled: true,
            lan_sync_enabled: false,
        }),
        MainTrayActionPlan::ToggleWindow
    );
    assert_eq!(
        main_tray_action_plan(MainTrayActionInput {
            action: MainTrayMenuAction::ToggleClipboardCapture,
            clipboard_capture_enabled: true,
            lan_sync_enabled: false,
        }),
        MainTrayActionPlan::SetClipboardCapture { enabled: false }
    );
    assert_eq!(
        main_tray_action_plan(MainTrayActionInput {
            action: MainTrayMenuAction::ToggleLanSync,
            clipboard_capture_enabled: false,
            lan_sync_enabled: false,
        }),
        MainTrayActionPlan::SetLanSync { enabled: true }
    );
    assert_eq!(
        main_tray_action_plan(MainTrayActionInput {
            action: MainTrayMenuAction::Exit,
            clipboard_capture_enabled: false,
            lan_sync_enabled: true,
        }),
        MainTrayActionPlan::Exit
    );
}

#[test]
fn tray_actions_map_to_stable_status_host_commands() {
    assert_eq!(
        MainTrayMenuAction::ToggleWindow.command_id(),
        menu_ids::TRAY_TOGGLE
    );
    assert_eq!(
        MainTrayMenuAction::ToggleClipboardCapture.command_id(),
        menu_ids::TRAY_CAPTURE_TOGGLE
    );
    assert_eq!(
        MainTrayMenuAction::ToggleLanSync.command_id(),
        menu_ids::TRAY_LAN_TOGGLE
    );
    assert_eq!(MainTrayMenuAction::Exit.command_id(), menu_ids::TRAY_EXIT);
}

#[test]
fn status_item_host_operations_are_explicit_porting_contract() {
    assert_eq!(
        REQUIRED_STATUS_ITEM_HOST_OPERATIONS,
        [
            StatusItemHostOperation::Install,
            StatusItemHostOperation::Remove,
            StatusItemHostOperation::PresentMenu,
        ]
    );
    let names: Vec<_> = REQUIRED_STATUS_ITEM_HOST_OPERATIONS
        .iter()
        .map(|operation| operation.operation_name())
        .collect();
    assert_eq!(
        names,
        [
            "install_status_item",
            "remove_status_item",
            "present_status_menu"
        ]
    );
}

#[test]
fn native_popup_menu_host_operations_are_explicit_porting_contract() {
    assert_eq!(
        REQUIRED_NATIVE_POPUP_MENU_HOST_OPERATIONS,
        [NativePopupMenuHostOperation::PresentPopupMenu]
    );
    assert_eq!(
        REQUIRED_NATIVE_POPUP_MENU_HOST_OPERATIONS[0].operation_name(),
        "present_popup_menu"
    );
}

#[test]
fn native_transient_window_host_operations_are_explicit_porting_contract() {
    assert_eq!(
        REQUIRED_NATIVE_TRANSIENT_WINDOW_HOST_OPERATIONS,
        [
            NativeTransientWindowHostOperation::CreateTransientWindow,
            NativeTransientWindowHostOperation::PresentTransientWindow,
            NativeTransientWindowHostOperation::HideTransientWindow,
            NativeTransientWindowHostOperation::DestroyTransientWindow,
        ]
    );
    let names: Vec<_> = REQUIRED_NATIVE_TRANSIENT_WINDOW_HOST_OPERATIONS
        .iter()
        .map(|operation| operation.operation_name())
        .collect();
    assert_eq!(
        names,
        [
            "create_transient_window",
            "present_transient_window",
            "hide_transient_window",
            "destroy_transient_window"
        ]
    );
}

#[test]
fn native_ime_host_operations_are_explicit_porting_contract() {
    assert_eq!(
        REQUIRED_NATIVE_IME_HOST_OPERATIONS,
        [
            NativeImeHostOperation::CandidateAnchor,
            NativeImeHostOperation::CompositionAnchor,
            NativeImeHostOperation::HasDefaultImeWindow,
        ]
    );
    let names: Vec<_> = REQUIRED_NATIVE_IME_HOST_OPERATIONS
        .iter()
        .map(|operation| operation.operation_name())
        .collect();
    assert_eq!(
        names,
        [
            "candidate_anchor",
            "composition_anchor",
            "has_default_ime_window"
        ]
    );
}

#[test]
fn native_text_caret_host_operations_are_explicit_porting_contract() {
    assert_eq!(
        REQUIRED_NATIVE_TEXT_CARET_HOST_OPERATIONS,
        [
            NativeTextCaretHostOperation::AccessibleCaretAnchor,
            NativeTextCaretHostOperation::ThreadCaretAnchor,
            NativeTextCaretHostOperation::FocusRectAnchor,
            NativeTextCaretHostOperation::CursorAnchor,
            NativeTextCaretHostOperation::FocusHandleForTarget,
        ]
    );
    let names: Vec<_> = REQUIRED_NATIVE_TEXT_CARET_HOST_OPERATIONS
        .iter()
        .map(|operation| operation.operation_name())
        .collect();
    assert_eq!(
        names,
        [
            "accessible_caret_anchor",
            "thread_caret_anchor",
            "focus_rect_anchor",
            "cursor_anchor",
            "focus_handle_for_target",
        ]
    );
    assert!(NativeTextCaretAnchor::new(1, 2, 3).has_vertical_span());
    assert!(!NativeTextCaretAnchor::new(1, 2, 2).has_vertical_span());
}

#[test]
fn native_dialog_host_operations_are_explicit_porting_contract() {
    assert_eq!(
        REQUIRED_NATIVE_DIALOG_HOST_OPERATIONS,
        [
            NativeDialogHostOperation::ShowMessage,
            NativeDialogHostOperation::Confirm,
        ]
    );
    let names: Vec<_> = REQUIRED_NATIVE_DIALOG_HOST_OPERATIONS
        .iter()
        .map(|operation| operation.operation_name())
        .collect();
    assert_eq!(names, ["show_message", "confirm"]);
    assert_eq!(NativeDialogResponse::Cancel, NativeDialogResponse::Cancel);
    assert_eq!(
        NativeDialogButtons::YesNoCancel,
        NativeDialogButtons::YesNoCancel
    );
    assert_eq!(NativeDialogButtons::YesNo, NativeDialogButtons::YesNo);
}

#[test]
fn native_shell_open_host_operations_are_explicit_porting_contract() {
    assert_eq!(
        REQUIRED_NATIVE_SHELL_OPEN_HOST_OPERATIONS,
        [NativeShellOpenHostOperation::OpenPath]
    );
    assert_eq!(
        REQUIRED_NATIVE_SHELL_OPEN_HOST_OPERATIONS[0].operation_name(),
        "open_path"
    );
}

#[test]
fn native_window_identity_host_operations_are_explicit_porting_contract() {
    assert_eq!(
        REQUIRED_NATIVE_WINDOW_IDENTITY_HOST_OPERATIONS,
        [
            NativeWindowIdentityHostOperation::ProcessName,
            NativeWindowIdentityHostOperation::ClassName,
            NativeWindowIdentityHostOperation::RootHandle,
            NativeWindowIdentityHostOperation::ForegroundHandle,
            NativeWindowIdentityHostOperation::Exists,
            NativeWindowIdentityHostOperation::IsForeground,
            NativeWindowIdentityHostOperation::IsCurrentProcessWindow,
        ]
    );
    let names: Vec<_> = REQUIRED_NATIVE_WINDOW_IDENTITY_HOST_OPERATIONS
        .iter()
        .map(|operation| operation.operation_name())
        .collect();
    assert_eq!(
        names,
        [
            "process_name",
            "class_name",
            "root_handle",
            "foreground_handle",
            "exists",
            "is_foreground",
            "is_current_process_window",
        ]
    );
}

#[test]
fn native_window_identity_snapshot_keeps_target_checks_shared() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Handle(u64);

    struct Host;

    impl NativeWindowIdentityHost for Host {
        type Handle = Handle;

        fn process_name(&self, _handle: Self::Handle) -> String {
            "Notes".to_string()
        }

        fn class_name(&self, _handle: Self::Handle) -> String {
            "TextEditor".to_string()
        }

        fn root_handle(&self, _handle: Self::Handle) -> Self::Handle {
            Handle(1)
        }

        fn foreground_handle(&self) -> Self::Handle {
            Handle(1)
        }

        fn exists(&self, _handle: Self::Handle) -> bool {
            true
        }

        fn is_foreground(&self, _handle: Self::Handle) -> bool {
            false
        }

        fn is_current_process_window(&self, _handle: Self::Handle) -> bool {
            false
        }
    }

    let snapshot = native_window_identity_snapshot(&Host, Handle(7));

    assert_eq!(snapshot.handle, Handle(7));
    assert_eq!(snapshot.process_name, "Notes");
    assert_eq!(snapshot.class_name, "TextEditor");
    assert_eq!(snapshot.root_handle, Handle(1));
    assert_eq!(snapshot.foreground_handle, Handle(1));
    assert!(snapshot.exists);
    assert!(!snapshot.is_foreground);
    assert!(!snapshot.is_current_process_window);
    assert!(snapshot.is_external_existing_target());
    assert!(snapshot.foreground_matches_target());
    assert!(snapshot.can_restore_or_paste_to_target());
}

#[test]
fn native_paste_target_host_operations_are_explicit_porting_contract() {
    assert_eq!(
        REQUIRED_NATIVE_PASTE_TARGET_HOST_OPERATIONS,
        [
            NativePasteTargetHostOperation::ForcePasteTargetForeground,
            NativePasteTargetHostOperation::RestorePasteTargetFocus,
            NativePasteTargetHostOperation::SetPasteTargetText,
            NativePasteTargetHostOperation::PasteTargetTextInputCapabilities,
            NativePasteTargetHostOperation::PasteTargetFocusStatus,
            NativePasteTargetHostOperation::PasteTargetTextInputReady,
            NativePasteTargetHostOperation::SendPasteShortcut,
        ]
    );
    let names: Vec<_> = REQUIRED_NATIVE_PASTE_TARGET_HOST_OPERATIONS
        .iter()
        .map(|operation| operation.operation_name())
        .collect();
    assert_eq!(
        names,
        [
            "force_paste_target_foreground",
            "restore_paste_target_focus",
            "set_paste_target_text",
            "paste_target_text_input_capabilities",
            "paste_target_focus_status",
            "paste_target_text_input_ready",
            "send_paste_shortcut",
        ]
    );
    assert!(PasteTargetFocusStatus::Unknown.allows_paste_attempt());
    assert!(PasteTargetFocusStatus::NoActiveFocus.allows_paste_attempt());
    assert!(PasteTargetFocusStatus::InsideTarget.allows_paste_attempt());
    assert!(!PasteTargetFocusStatus::OutsideTarget.allows_paste_attempt());
}

#[test]
fn native_paste_target_activation_snapshot_keeps_vv_readiness_shared() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Handle(u64);

    #[derive(Default)]
    struct Host {
        calls: Vec<&'static str>,
    }

    impl NativePasteTargetHost for Host {
        type Handle = Handle;

        fn force_paste_target_foreground(&mut self, _target: Self::Handle) -> bool {
            self.calls.push("force_paste_target_foreground");
            true
        }

        fn restore_paste_target_focus(&mut self, _target: Self::Handle, _focus: Self::Handle) {
            self.calls.push("restore_paste_target_focus");
        }

        fn set_paste_target_text(&mut self, _target: Self::Handle, _text: &str) -> bool {
            self.calls.push("set_paste_target_text");
            true
        }

        fn paste_target_text_input_capabilities(
            &mut self,
            _target: Self::Handle,
        ) -> PasteTargetTextInputCapabilities {
            self.calls.push("paste_target_text_input_capabilities");
            PasteTargetTextInputCapabilities::text_input()
        }

        fn paste_target_focus_status(
            &mut self,
            _target: Self::Handle,
            _passthrough_focus: Self::Handle,
        ) -> PasteTargetFocusStatus {
            self.calls.push("paste_target_focus_status");
            PasteTargetFocusStatus::InsideTarget
        }

        fn paste_target_text_input_ready(&mut self, _target: Self::Handle) -> bool {
            self.calls.push("paste_target_text_input_ready");
            true
        }

        fn send_paste_shortcut(&mut self, _target: Self::Handle) -> bool {
            self.calls.push("send_paste_shortcut");
            true
        }
    }

    let mut host = Host::default();
    let snapshot = native_paste_target_activation_snapshot(&mut host, Handle(7), Handle(8));

    assert_eq!(snapshot.target, Handle(7));
    assert_eq!(snapshot.passthrough_focus, Handle(8));
    assert!(snapshot.foregrounded);
    assert_eq!(
        snapshot.text_input_capabilities,
        PasteTargetTextInputCapabilities::text_input()
    );
    assert_eq!(snapshot.focus_status, PasteTargetFocusStatus::InsideTarget);
    assert!(snapshot.text_input_ready);
    assert!(snapshot.can_directly_set_text());
    assert!(snapshot.can_send_paste_shortcut());
    assert_eq!(
        host.calls,
        vec![
            "force_paste_target_foreground",
            "paste_target_text_input_capabilities",
            "paste_target_focus_status",
            "paste_target_text_input_ready"
        ]
    );
}

#[test]
fn native_file_dialog_host_operations_are_explicit_porting_contract() {
    assert_eq!(
        REQUIRED_NATIVE_FILE_DIALOG_HOST_OPERATIONS,
        [NativeFileDialogHostOperation::PickFile]
    );
    assert_eq!(
        REQUIRED_NATIVE_FILE_DIALOG_HOST_OPERATIONS[0].operation_name(),
        "pick_file"
    );
    let request = NativeFileDialogRequest {
        title: "Choose",
        filter_name: "Wave Files",
        filter_pattern: "*.wav",
        current_path: "",
    };
    assert_eq!(request.filter_pattern, "*.wav");
}

#[test]
fn native_text_input_dialog_host_operations_are_explicit_porting_contract() {
    assert_eq!(
        REQUIRED_NATIVE_TEXT_INPUT_DIALOG_HOST_OPERATIONS,
        [NativeTextInputDialogHostOperation::PromptText]
    );
    assert_eq!(
        REQUIRED_NATIVE_TEXT_INPUT_DIALOG_HOST_OPERATIONS[0].operation_name(),
        "prompt_text"
    );
    let request = NativeTextInputDialogRequest {
        title: "Rename",
        label: "Name:",
        initial: "Old",
    };
    assert_eq!(request.initial, "Old");
}

#[test]
fn native_edit_text_dialog_host_operations_are_explicit_porting_contract() {
    assert_eq!(
        REQUIRED_NATIVE_EDIT_TEXT_DIALOG_HOST_OPERATIONS,
        [NativeEditTextDialogHostOperation::OpenEditText]
    );
    assert_eq!(
        REQUIRED_NATIVE_EDIT_TEXT_DIALOG_HOST_OPERATIONS[0].operation_name(),
        "open_edit_text"
    );
    let request = NativeEditTextDialogRequest {
        title: "编辑记录",
        initial_text: "旧内容",
        initial_size: Some(Size {
            width: 640,
            height: 500,
        }),
    };
    assert_eq!(request.initial_text, "旧内容");
    assert_eq!(request.initial_size.unwrap().width, 640);

    let mut saved = String::new();
    let mut save_handler = |text: &str| {
        saved = text.to_string();
        Ok(())
    };
    assert!(save_handler.save_text("新内容").is_ok());
    assert_eq!(saved, "新内容");

    let clean_close = native_host_edit_text_close_plan("same", "same");
    assert!(!clean_close.dirty);
    assert!(!clean_close.requires_unsaved_confirmation);
    let dirty_close = native_host_edit_text_close_plan("old", "new");
    assert!(dirty_close.dirty);
    assert!(dirty_close.requires_unsaved_confirmation);
}

#[test]
fn native_mail_merge_window_host_operations_are_explicit_porting_contract() {
    assert_eq!(
        REQUIRED_NATIVE_MAIL_MERGE_WINDOW_HOST_OPERATIONS,
        [NativeMailMergeWindowHostOperation::OpenMailMerge]
    );
    assert_eq!(
        REQUIRED_NATIVE_MAIL_MERGE_WINDOW_HOST_OPERATIONS[0].operation_name(),
        "open_mail_merge"
    );
    let request = NativeMailMergeWindowRequest {
        initial_excel_path: Some("data.xlsx"),
    };
    assert_eq!(request.initial_excel_path, Some("data.xlsx"));
}

#[test]
fn native_autostart_host_uses_trait_contract_without_parallel_required_operations() {
    #[derive(Default)]
    struct Host {
        enabled: bool,
    }

    impl NativeAutostartHost for Host {
        fn autostart_status(&self) -> NativeAutostartStatus {
            if self.enabled {
                NativeAutostartStatus::enabled_at("autostart")
            } else {
                NativeAutostartStatus::disabled()
            }
        }

        fn set_autostart_enabled(&mut self, enabled: bool) -> NativeAutostartApplyResult {
            self.enabled = enabled;
            NativeAutostartApplyResult::applied(enabled, self.autostart_status())
        }
    }

    let mut host = Host::default();
    let enabled = host.set_autostart_enabled(true);
    assert!(enabled.applied);
    assert_eq!(
        enabled.status.registration_path.as_deref(),
        Some("autostart")
    );
    let disabled = host.set_autostart_enabled(false);
    assert!(disabled.applied);
    assert!(!disabled.status.enabled);
    let source = include_str!("host_protocol.rs");
    assert!(source.contains("pub(crate) trait NativeAutostartHost"));
    assert!(!source.contains("REQUIRED_NATIVE_AUTOSTART_HOST_OPERATIONS"));
    assert!(!source.contains("NativeAutostartHostOperation"));
}

#[test]
fn native_main_window_host_operations_are_explicit_porting_contract() {
    assert_eq!(
        REQUIRED_NATIVE_MAIN_WINDOW_HOST_OPERATIONS,
        [
            NativeMainWindowHostOperation::CreateMainWindows,
            NativeMainWindowHostOperation::ApplyMainWindowAppearance,
            NativeMainWindowHostOperation::SetMainWindowAppIcon,
            NativeMainWindowHostOperation::HideMainWindow,
            NativeMainWindowHostOperation::PresentMainWindow,
            NativeMainWindowHostOperation::SetMainWindowBounds,
            NativeMainWindowHostOperation::ActivateMainWindow,
            NativeMainWindowHostOperation::ForegroundMainWindow,
            NativeMainWindowHostOperation::RestoreMainWindow,
            NativeMainWindowHostOperation::CloseMainWindow,
            NativeMainWindowHostOperation::SetMainWindowActivationPolicy,
            NativeMainWindowHostOperation::RequestMainWindowClose,
            NativeMainWindowHostOperation::DestroyMainWindow,
            NativeMainWindowHostOperation::CaptureMainPointer,
            NativeMainWindowHostOperation::ReleaseMainPointer,
            NativeMainWindowHostOperation::BeginMainWindowDrag,
            NativeMainWindowHostOperation::TrackMainPointerLeave,
            NativeMainWindowHostOperation::RequestMainWindowAreaRepaint,
            NativeMainWindowHostOperation::MainWindowLayoutDpi,
            NativeMainWindowHostOperation::MainWindowClientBounds,
            NativeMainWindowHostOperation::MainWindowBounds,
        ]
    );
    let names: Vec<_> = REQUIRED_NATIVE_MAIN_WINDOW_HOST_OPERATIONS
        .iter()
        .map(|operation| operation.operation_name())
        .collect();
    assert_eq!(
        names,
        [
            "create_main_windows",
            "apply_main_window_appearance",
            "set_main_window_app_icon",
            "hide_main_window",
            "present_main_window",
            "set_main_window_bounds",
            "activate_main_window",
            "foreground_main_window",
            "restore_main_window",
            "close_main_window",
            "set_main_window_activation_policy",
            "request_main_window_close",
            "destroy_main_window",
            "capture_main_pointer",
            "release_main_pointer",
            "begin_main_window_drag",
            "track_main_pointer_leave",
            "request_main_window_area_repaint",
            "main_window_layout_dpi",
            "main_window_client_bounds",
            "main_window_bounds"
        ]
    );

    let request = NativeMainWindowRequest {
        title: "ZSClip".to_string(),
        size: Size {
            width: 300,
            height: 614,
        },
        main_visible: true,
    };
    assert_eq!(request.title, "ZSClip");
    assert_eq!(request.size.height, 614);

    let handles = NativeMainWindowHandles {
        main: NativeWindowToken(1),
        quick: NativeWindowToken(2),
    };
    assert_eq!(
        NativeMainWindowPresentation::Created(handles),
        NativeMainWindowPresentation::Created(handles)
    );
}

#[test]
fn native_main_search_control_host_operations_are_explicit_porting_contract() {
    assert_eq!(
        REQUIRED_NATIVE_MAIN_SEARCH_CONTROL_HOST_OPERATIONS,
        [
            NativeMainSearchControlHostOperation::CreateSearchControl,
            NativeMainSearchControlHostOperation::ApplySearchStyle,
            NativeMainSearchControlHostOperation::ReleaseSearchStyleResource,
            NativeMainSearchControlHostOperation::SetSearchBounds,
            NativeMainSearchControlHostOperation::SetSearchVisible,
            NativeMainSearchControlHostOperation::SearchText,
            NativeMainSearchControlHostOperation::SetSearchText,
            NativeMainSearchControlHostOperation::FocusSearch,
        ]
    );
    let names: Vec<_> = REQUIRED_NATIVE_MAIN_SEARCH_CONTROL_HOST_OPERATIONS
        .iter()
        .map(|operation| operation.operation_name())
        .collect();
    assert_eq!(
        names,
        [
            "create_search_control",
            "apply_search_style",
            "release_search_style_resource",
            "set_search_bounds",
            "set_search_visible",
            "search_text",
            "set_search_text",
            "focus_search",
        ]
    );

    let request = NativeMainSearchControlRequest {
        owner: NativeWindowToken(1),
        id: 1001,
        bounds: UiRect::new(10, 20, 260, 48),
        visible: true,
    };
    assert_eq!(request.owner, NativeWindowToken(1));
    assert_eq!(request.id, 1001);
    assert!(request.visible);
    assert_eq!(
        NativeMainSearchControlPresentation::Created(NativeWindowToken(2)),
        NativeMainSearchControlPresentation::Created(NativeWindowToken(2))
    );
    let style = NativeMainSearchStyleRequest {
        handle: NativeWindowToken(2),
        font_family: "System".to_string(),
        font_px: 14,
        previous_resource: Some(NativeWindowToken(3)),
    };
    assert_eq!(style.handle, NativeWindowToken(2));
    assert_eq!(
        NativeMainSearchStylePresentation::Applied(Some(NativeWindowToken(4))),
        NativeMainSearchStylePresentation::Applied(Some(NativeWindowToken(4)))
    );
}

#[test]
fn native_settings_window_host_operations_are_explicit_porting_contract() {
    assert_eq!(
        REQUIRED_NATIVE_SETTINGS_WINDOW_HOST_OPERATIONS,
        [
            NativeSettingsWindowHostOperation::PresentSettingsWindow,
            NativeSettingsWindowHostOperation::SetSettingsWindowBounds,
            NativeSettingsWindowHostOperation::DestroySettingsWindow,
            NativeSettingsWindowHostOperation::FocusSettingsWindow,
            NativeSettingsWindowHostOperation::TrackSettingsPointerLeave,
            NativeSettingsWindowHostOperation::CaptureSettingsPointer,
            NativeSettingsWindowHostOperation::ReleaseSettingsPointer,
            NativeSettingsWindowHostOperation::RequestSettingsWindowRepaint,
            NativeSettingsWindowHostOperation::RequestSettingsWindowAreaRepaint,
            NativeSettingsWindowHostOperation::SettingsWindowLayoutDpi,
            NativeSettingsWindowHostOperation::SettingsWindowClientToScreen,
            NativeSettingsWindowHostOperation::SettingsWindowClientBounds,
            NativeSettingsWindowHostOperation::SettingsWindowBounds,
        ]
    );
    let names: Vec<_> = REQUIRED_NATIVE_SETTINGS_WINDOW_HOST_OPERATIONS
        .iter()
        .map(|operation| operation.operation_name())
        .collect();
    assert_eq!(
        names,
        [
            "present_settings_window",
            "set_settings_window_bounds",
            "destroy_settings_window",
            "focus_settings_window",
            "track_settings_pointer_leave",
            "capture_settings_pointer",
            "release_settings_pointer",
            "request_settings_window_repaint",
            "request_settings_window_area_repaint",
            "settings_window_layout_dpi",
            "settings_window_client_to_screen",
            "settings_window_client_bounds",
            "settings_window_bounds"
        ]
    );

    let request = NativeSettingsWindowRequest {
        owner: NativeWindowToken(1),
        existing: Some(NativeWindowToken(2)),
        bounds: UiRect::new(10, 20, 1110, 760),
    };
    assert_eq!(request.existing, Some(NativeWindowToken(2)));
    assert_eq!(
        NativeSettingsWindowPresentation::FocusedExisting(NativeWindowToken(2)),
        NativeSettingsWindowPresentation::FocusedExisting(NativeWindowToken(2))
    );
}

#[test]
fn native_settings_dropdown_host_operations_are_explicit_porting_contract() {
    assert_eq!(
        REQUIRED_NATIVE_SETTINGS_DROPDOWN_HOST_OPERATIONS,
        [
            NativeSettingsDropdownHostOperation::PresentSettingsDropdown,
            NativeSettingsDropdownHostOperation::DestroySettingsDropdown,
            NativeSettingsDropdownHostOperation::SettingsDropdownBounds,
        ]
    );
    let names: Vec<_> = REQUIRED_NATIVE_SETTINGS_DROPDOWN_HOST_OPERATIONS
        .iter()
        .map(|operation| operation.operation_name())
        .collect();
    assert_eq!(
        names,
        [
            "present_settings_dropdown",
            "destroy_settings_dropdown",
            "settings_dropdown_bounds"
        ]
    );

    let request = NativeSettingsDropdownRequest {
        owner: NativeWindowToken(1),
        control_id: 42,
        anchor: UiRect::new(10, 20, 110, 52),
        items: vec!["A".to_string(), "B".to_string()],
        selected: 1,
        width: 180,
    };
    assert_eq!(request.control_id, 42);
    assert_eq!(
        NativeSettingsDropdownPresentation::Created(NativeWindowToken(2)),
        NativeSettingsDropdownPresentation::Created(NativeWindowToken(2))
    );
}

#[test]
fn settings_group_text_input_requests_are_shared_prompt_models() {
    let add = settings_group_text_input_request(SettingsGroupTextInputKind::Add, "");
    assert_eq!(add.title, "新建分组");
    assert_eq!(add.label, "请输入分组名称：");
    assert_eq!(add.initial, "新分组");

    let rename = settings_group_text_input_request(SettingsGroupTextInputKind::Rename, "旧分组");
    assert_eq!(rename.title, "重命名分组");
    assert_eq!(rename.label, "请输入新名称：");
    assert_eq!(rename.initial, "旧分组");
}

#[test]
fn window_toggle_visibility_plan_preserves_edge_restore_fallback_order() {
    assert_eq!(
        main_window_toggle_visibility_plan(MainWindowVisibilityInput {
            main_visible: true,
            quick_visible: true,
            main_edge_hidden: true,
            quick_edge_hidden: true,
        }),
        MainWindowVisibilityPlan {
            steps: vec![
                MainWindowVisibilityStep::TryRestoreQuickEdge,
                MainWindowVisibilityStep::HideQuick,
                MainWindowVisibilityStep::TryRestoreMainEdge,
                MainWindowVisibilityStep::HideMain,
            ]
        }
    );

    assert_eq!(
        main_window_toggle_visibility_plan(MainWindowVisibilityInput {
            main_visible: false,
            quick_visible: false,
            main_edge_hidden: false,
            quick_edge_hidden: false,
        }),
        MainWindowVisibilityPlan {
            steps: vec![MainWindowVisibilityStep::ShowMain]
        }
    );
}

#[test]
fn hotkey_visibility_plan_prefers_quick_then_main_then_show_quick() {
    assert_eq!(
        main_window_hotkey_visibility_plan(MainWindowHotkeyVisibilityInput {
            main_visible: true,
            quick_visible: true,
            main_edge_hidden: true,
            quick_edge_hidden: true,
            plain_text_paste_mode: true,
        }),
        MainWindowHotkeyVisibilityPlan {
            steps: vec![
                MainWindowHotkeyVisibilityStep::TryRestoreQuickEdge,
                MainWindowHotkeyVisibilityStep::HideQuick,
            ]
        }
    );

    assert_eq!(
        main_window_hotkey_visibility_plan(MainWindowHotkeyVisibilityInput {
            main_visible: true,
            quick_visible: false,
            main_edge_hidden: true,
            quick_edge_hidden: false,
            plain_text_paste_mode: true,
        }),
        MainWindowHotkeyVisibilityPlan {
            steps: vec![
                MainWindowHotkeyVisibilityStep::TryRestoreMainEdge,
                MainWindowHotkeyVisibilityStep::HideMain,
            ]
        }
    );

    assert_eq!(
        main_window_hotkey_visibility_plan(MainWindowHotkeyVisibilityInput {
            main_visible: false,
            quick_visible: false,
            main_edge_hidden: false,
            quick_edge_hidden: false,
            plain_text_paste_mode: true,
        }),
        MainWindowHotkeyVisibilityPlan {
            steps: vec![MainWindowHotkeyVisibilityStep::ShowQuick {
                plain_text_paste_mode: true,
            }]
        }
    );
}

#[test]
fn window_position_plan_uses_mode_fallbacks_and_clamps_to_bounds() {
    let bounds = UiRect::new(0, 0, 800, 600);
    let base = MainWindowPositionInput {
        mode: MainWindowPositionMode::Center,
        by_hotkey: false,
        cursor_x: 100,
        cursor_y: 100,
        mouse_dx: 12,
        mouse_dy: 24,
        fixed_x: 40,
        fixed_y: 50,
        last_x: -1,
        last_y: -1,
        bounds,
        win_w: 200,
        win_h: 120,
    };

    assert_eq!(
        main_window_position_plan(base),
        MainWindowPositionPlan {
            x: 300,
            y: 160,
            width: 200,
            height: 120,
        }
    );

    assert_eq!(
        main_window_position_anchor(MainWindowPositionInput {
            by_hotkey: true,
            ..base
        }),
        MainWindowPositionAnchor { x: 112, y: 124 }
    );

    assert_eq!(
        main_window_position_plan(MainWindowPositionInput {
            mode: MainWindowPositionMode::Last,
            last_x: 700,
            last_y: 560,
            ..base
        }),
        MainWindowPositionPlan {
            x: 600,
            y: 480,
            width: 200,
            height: 120,
        }
    );
}

#[test]
fn edge_restore_and_remember_position_are_platform_neutral() {
    assert_eq!(
        main_edge_restore_position(MainEdgeRestorePositionInput {
            edge_auto_hide: true,
            edge_hidden_side: 1,
            edge_docked_left: 100,
            edge_docked_top: 50,
            edge_docked_right: 360,
            edge_docked_bottom: 520,
            edge_restore_x: 640,
            edge_restore_y: 80,
            last_window_x: 24,
            last_window_y: 32,
        }),
        Some(MainWindowPositionAnchor { x: 640, y: 80 })
    );

    assert_eq!(
        main_edge_restore_position(MainEdgeRestorePositionInput {
            edge_auto_hide: true,
            edge_hidden_side: -1,
            edge_docked_left: 0,
            edge_docked_top: 0,
            edge_docked_right: 0,
            edge_docked_bottom: 0,
            edge_restore_x: 640,
            edge_restore_y: 80,
            last_window_x: 24,
            last_window_y: 32,
        }),
        Some(MainWindowPositionAnchor { x: 24, y: 32 })
    );

    assert_eq!(
        main_edge_restore_position(MainEdgeRestorePositionInput {
            edge_auto_hide: false,
            edge_hidden_side: 1,
            edge_docked_left: 100,
            edge_docked_top: 50,
            edge_docked_right: 360,
            edge_docked_bottom: 520,
            edge_restore_x: 640,
            edge_restore_y: 80,
            last_window_x: 24,
            last_window_y: 32,
        }),
        None
    );

    assert_eq!(
        main_remember_window_position(MainRememberWindowPositionInput {
            edge_auto_hide: true,
            edge_hidden: true,
            edge_restore_x: 640,
            edge_restore_y: 80,
            window_left: 12,
            window_top: 18,
        }),
        MainWindowPositionAnchor { x: 640, y: 80 }
    );

    assert_eq!(
        main_remember_window_position(MainRememberWindowPositionInput {
            edge_auto_hide: true,
            edge_hidden: false,
            edge_restore_x: 640,
            edge_restore_y: 80,
            window_left: 12,
            window_top: 18,
        }),
        MainWindowPositionAnchor { x: 12, y: 18 }
    );
}

#[test]
fn show_window_state_plan_separates_main_and_quick_passthrough() {
    assert_eq!(
        main_show_window_state_plan(MainShowWindowStateInput {
            kind: MainShowWindowKind::Main,
            by_hotkey: true,
            edge_auto_hide: true,
            foreground_snapshot_available: true,
            plain_text_paste_mode: true,
        }),
        MainShowWindowStatePlan {
            reset_edge_hidden_state: true,
            dock_action: MainShowWindowDockAction::NoteMovedForEdgeHide,
            hotkey_passthrough: MainHotkeyPassthroughPlan::Clear,
            plain_text_paste_mode: false,
        }
    );

    assert_eq!(
        main_show_window_state_plan(MainShowWindowStateInput {
            kind: MainShowWindowKind::Quick,
            by_hotkey: true,
            edge_auto_hide: false,
            foreground_snapshot_available: true,
            plain_text_paste_mode: true,
        }),
        MainShowWindowStatePlan {
            reset_edge_hidden_state: true,
            dock_action: MainShowWindowDockAction::ClearEdgeDockState,
            hotkey_passthrough: MainHotkeyPassthroughPlan::UseForegroundSnapshot,
            plain_text_paste_mode: true,
        }
    );

    assert_eq!(
        main_show_window_state_plan(MainShowWindowStateInput {
            kind: MainShowWindowKind::Quick,
            by_hotkey: true,
            edge_auto_hide: false,
            foreground_snapshot_available: false,
            plain_text_paste_mode: false,
        })
        .hotkey_passthrough,
        MainHotkeyPassthroughPlan::Clear
    );
}

#[test]
fn hotkey_registration_plan_normalizes_platform_neutral_specs() {
    assert_eq!(
        main_hotkey_registration_plan(MainHotkeyRegistrationInput {
            enabled: false,
            already_registered: true,
            mod_label: "Ctrl",
            key_label: "C",
        }),
        MainHotkeyRegistrationPlan {
            unregister_existing: true,
            register: None,
        }
    );

    assert_eq!(
        main_hotkey_registration_plan(MainHotkeyRegistrationInput {
            enabled: true,
            already_registered: true,
            mod_label: " Ctrl+Shift ",
            key_label: " v ",
        }),
        MainHotkeyRegistrationPlan {
            unregister_existing: true,
            register: Some(MainHotkeySpec {
                modifiers: MainHotkeyModifiers {
                    ctrl: true,
                    alt: false,
                    shift: true,
                    meta: false,
                },
                key: MainHotkeyKey::Char('V'),
            }),
        }
    );

    assert_eq!(
        main_hotkey_registration_plan(MainHotkeyRegistrationInput {
            enabled: true,
            already_registered: false,
            mod_label: "unknown",
            key_label: "unknown",
        }),
        MainHotkeyRegistrationPlan {
            unregister_existing: false,
            register: Some(MainHotkeySpec {
                modifiers: MainHotkeyModifiers::meta(),
                key: MainHotkeyKey::Char('V'),
            }),
        }
    );
}

#[test]
fn show_prepare_plan_combines_shared_tab_and_search_state() {
    assert_eq!(
        main_show_prepare_plan(MainShowPrepareInput {
            shared_tab_changed: true,
            persistent_search_box: true,
        }),
        MainShowPreparePlan {
            clear_selection: true,
            reset_scroll: true,
            refilter: true,
            search_action: MainShowSearchAction::ShowPersistent,
        }
    );

    assert_eq!(
        main_show_prepare_plan(MainShowPrepareInput {
            shared_tab_changed: false,
            persistent_search_box: false,
        }),
        MainShowPreparePlan {
            clear_selection: false,
            reset_scroll: false,
            refilter: false,
            search_action: MainShowSearchAction::Reset,
        }
    );
}

#[test]
fn search_visibility_plan_describes_toggle_open_and_close() {
    assert_eq!(
        main_search_visibility_plan(MainSearchVisibilityInput {
            request: MainSearchVisibilityRequest::Toggle,
            search_on: false,
            search_text_empty: true,
            persistent_search_box: false,
            main_window_noactivate: true,
            quick_window: false,
        }),
        MainSearchVisibilityPlan {
            action: MainSearchVisibilityAction::Open,
            search_on: true,
            activate_window: true,
            stop_debounce_timer: false,
            clear_search_text: false,
            clear_selection: false,
            refilter: false,
            relayout: true,
            invalidate: true,
        }
    );

    assert_eq!(
        main_search_visibility_plan(MainSearchVisibilityInput {
            request: MainSearchVisibilityRequest::Close,
            search_on: true,
            search_text_empty: false,
            persistent_search_box: true,
            main_window_noactivate: false,
            quick_window: false,
        }),
        MainSearchVisibilityPlan {
            action: MainSearchVisibilityAction::ClosePersistent,
            search_on: true,
            activate_window: false,
            stop_debounce_timer: false,
            clear_search_text: false,
            clear_selection: false,
            refilter: false,
            relayout: true,
            invalidate: true,
        }
    );

    assert_eq!(
        main_search_visibility_plan(MainSearchVisibilityInput {
            request: MainSearchVisibilityRequest::Close,
            search_on: true,
            search_text_empty: false,
            persistent_search_box: false,
            main_window_noactivate: false,
            quick_window: false,
        }),
        MainSearchVisibilityPlan {
            action: MainSearchVisibilityAction::CloseReset,
            search_on: false,
            activate_window: false,
            stop_debounce_timer: true,
            clear_search_text: true,
            clear_selection: true,
            refilter: true,
            relayout: true,
            invalidate: true,
        }
    );
}

#[test]
fn settings_control_roles_map_to_platform_neutral_commands() {
    assert_eq!(
        settings_command_id_for_role(SettingsControlRole::Save),
        command_ids::SAVE_SETTINGS
    );
    assert_eq!(
        settings_command_id_for_role(SettingsControlRole::Close),
        command_ids::CLOSE_SETTINGS
    );
    assert_eq!(
        settings_command_for_control_role(SettingsControlRole::OpenConfig, 99),
        Command::window(command_ids::OPEN_SETTINGS_CONFIG)
    );
    assert_eq!(
        settings_command_for_control_role(SettingsControlRole::Dropdown, 6102),
        Command::window_with_payload(
            command_ids::OPEN_SETTINGS_DROPDOWN,
            CommandPayload::ControlId(6102)
        )
    );
    assert_eq!(
        settings_command_for_control_role(SettingsControlRole::Toggle, 7101),
        Command::window_with_payload(
            command_ids::TOGGLE_SETTINGS_CONTROL,
            CommandPayload::ControlId(7101)
        )
    );
}

#[test]
fn settings_actions_dispatch_to_platform_neutral_executor_domains() {
    #[derive(Default)]
    struct FakeExecutor {
        routes: Vec<(SettingsActionRoute, SettingsAction)>,
    }

    impl SettingsActionExecutor for FakeExecutor {
        type Context = usize;

        fn execute_sync(&mut self, context: &mut Self::Context, action: SettingsAction) -> bool {
            *context += 1;
            self.routes.push((SettingsActionRoute::Sync, action));
            true
        }

        fn execute_group(&mut self, context: &mut Self::Context, action: SettingsAction) -> bool {
            *context += 1;
            self.routes.push((SettingsActionRoute::Group, action));
            true
        }

        fn execute_platform(
            &mut self,
            context: &mut Self::Context,
            action: SettingsAction,
        ) -> bool {
            *context += 1;
            self.routes.push((SettingsActionRoute::Platform, action));
            true
        }
    }

    let mut executor = FakeExecutor::default();
    let mut calls = 0;
    for action in [
        SettingsAction::SyncWebDavNow,
        SettingsAction::AddGroup,
        SettingsAction::OpenSourceRepository,
    ] {
        assert!(dispatch_settings_action(&mut executor, &mut calls, action));
    }

    assert_eq!(calls, 3);
    assert_eq!(
        executor.routes,
        vec![
            (SettingsActionRoute::Sync, SettingsAction::SyncWebDavNow),
            (SettingsActionRoute::Group, SettingsAction::AddGroup),
            (
                SettingsActionRoute::Platform,
                SettingsAction::OpenSourceRepository,
            ),
        ]
    );
}

#[test]
fn settings_native_action_routes_resolve_to_shared_actions() {
    assert_eq!(
        settings_action_for_route("settings_sync", "sync_webdav_now"),
        Some(SettingsAction::SyncWebDavNow)
    );
    assert_eq!(
        settings_action_for_route("settings_sync", "open_lan_setup_page"),
        Some(SettingsAction::OpenLanSetupPage)
    );
    assert_eq!(
        settings_action_for_route("settings_group", "rename_group"),
        Some(SettingsAction::RenameGroup)
    );
    assert_eq!(
        settings_action_for_route("settings_platform", "open_source_repository"),
        Some(SettingsAction::OpenSourceRepository)
    );
    assert_eq!(
        settings_action_for_route("settings_platform", "restart_system_shell"),
        Some(SettingsAction::RestartSystemShell)
    );
    assert_eq!(settings_action_for_route("settings_sync", "missing"), None);
}

#[test]
fn settings_lan_sync_actions_have_shared_host_support_plan() {
    let plan = settings_lan_sync_action_support_plan(
        SettingsAction::RefreshLanDevices,
        "linux_native_host",
        "planned_not_implemented",
    )
    .expect("LAN settings action should have a shared host support plan");

    assert_eq!(plan.action, SettingsAction::RefreshLanDevices);
    assert_eq!(plan.action_name, "refresh_lan_devices");
    assert_eq!(plan.feature_name, SETTINGS_LAN_SYNC_FEATURE_NAME);
    assert_eq!(
        plan.runtime_boundary,
        SettingsLanSyncRuntimeBoundary::ServiceDiscovery
    );
    assert_eq!(plan.runtime_boundary_name, "service_discovery");
    assert_eq!(
        plan.required_host_capability_names,
        &["lan_service_lifecycle", "lan_udp_discovery"]
    );
    assert!(!plan.accepted);
    assert_eq!(
        plan.result_name,
        "zsclip.settings_sync.refresh_lan_devices.planned_not_implemented_on_linux_native_host"
    );
    assert_eq!(
        plan.missing_runtime_boundary,
        "platform LAN service lifecycle, UDP discovery, and device cache refresh"
    );
    let planned_action_names = SETTINGS_LAN_SYNC_ACTIONS
        .iter()
        .filter_map(|action| settings_lan_sync_action_name(*action))
        .collect::<Vec<_>>();
    assert_eq!(
        planned_action_names,
        vec![
            "refresh_lan_devices",
            "pair_lan_device",
            "accept_lan_pairing",
            "reject_lan_pairing",
            "copy_lan_pair_url",
            "copy_lan_setup_url",
            "open_lan_setup_page",
        ]
    );
    assert_eq!(
        settings_lan_sync_action_support_plan(
            SettingsAction::SyncWebDavNow,
            "linux_native_host",
            "ready_pending_smoke",
        ),
        None
    );
}

#[test]
fn settings_lan_mobile_link_projection_builds_pair_and_setup_urls() {
    let settings_json = serde_json::json!({
        "lan_sync_enabled": true,
        "lan_manual_host": " http://192.168.1.50/mobile/setup ",
        "lan_tcp_port": 38474,
    });
    let pair = settings_lan_mobile_link_projection_from_json(
        SettingsAction::CopyLanPairUrl,
        "macos_native_host",
        &settings_json,
    )
    .expect("pair link projection");

    assert!(pair.accepted);
    assert_eq!(pair.status, SettingsLanMobileLinkProjectionStatus::Ready);
    assert_eq!(pair.host.as_deref(), Some("192.168.1.50:38474"));
    assert_eq!(
        pair.setup_url.as_deref(),
        Some("http://192.168.1.50:38474/mobile/setup")
    );
    assert_eq!(
        pair.pair_url.as_deref(),
        Some("zsclip://pair?host=192.168.1.50%3A38474")
    );
    assert_eq!(
        pair.target_url.as_deref(),
        Some("zsclip://pair?host=192.168.1.50%3A38474")
    );
    assert_eq!(
        pair.result_name,
        "zsclip.settings_sync.copy_lan_pair_url.mobile_link_projected_on_macos_native_host"
    );

    let open = settings_lan_mobile_link_projection_from_json(
        SettingsAction::OpenLanSetupPage,
        "linux_native_host",
        &settings_json,
    )
    .expect("setup page projection");
    assert_eq!(
        open.target_url.as_deref(),
        Some("http://192.168.1.50:38474/mobile/setup")
    );

    let missing_host = settings_lan_mobile_link_projection_from_json(
        SettingsAction::CopyLanSetupUrl,
        "linux_native_host",
        &serde_json::json!({"lan_sync_enabled": true}),
    )
    .expect("missing host projection");
    assert!(!missing_host.accepted);
    assert_eq!(
        missing_host.status,
        SettingsLanMobileLinkProjectionStatus::MissingHost
    );
    assert_eq!(
        missing_host.result_name,
        "zsclip.settings_sync.copy_lan_setup_url.missing_lan_host_on_linux_native_host"
    );
}

#[test]
fn settings_lan_device_book_projection_summarizes_persisted_devices() {
    let desktop = settings_lan_device_projection(
        "desktop-1",
        "Desktop",
        "192.168.1.20",
        38473,
        1234,
        true,
        vec!["receive_clip".to_string(), "text".to_string()],
    );
    let phone = settings_lan_device_projection(
        "phone-1",
        "Phone",
        "192.168.1.30",
        0,
        5678,
        true,
        vec!["pull_only".to_string()],
    );
    let projection = settings_lan_device_book_projection("linux_native_host", vec![desktop, phone]);

    assert!(projection.accepted);
    assert_eq!(projection.action, SettingsAction::RefreshLanDevices);
    assert_eq!(projection.action_name, "refresh_lan_devices");
    assert_eq!(projection.device_count, 2);
    assert_eq!(projection.trusted_device_count, 2);
    assert_eq!(projection.receivable_device_count, 1);
    assert_eq!(projection.devices[0].endpoint, "192.168.1.20:38473");
    assert!(projection.devices[0].can_receive_clip);
    assert!(!projection.devices[1].can_receive_clip);
    assert_eq!(
        projection.result_name,
        "zsclip.settings_sync.refresh_lan_devices.device_book_projected_2_on_linux_native_host"
    );
}

#[test]
fn settings_lan_pair_request_projection_builds_protocol_body() {
    let settings_json = serde_json::json!({
        "lan_sync_enabled": true,
        "lan_manual_host": "192.168.1.88",
        "lan_device_id": "desktop-1",
        "lan_device_name": "Desktop",
        "lan_tcp_port": 38474,
    });
    let projection = settings_lan_pair_request_projection_from_json(
        SettingsAction::PairLanDevice,
        "macos_native_host",
        &settings_json,
    )
    .expect("pair request projection");

    assert!(projection.accepted);
    assert_eq!(
        projection.status,
        SettingsLanPairRequestProjectionStatus::Ready
    );
    assert_eq!(projection.host.as_deref(), Some("192.168.1.88:38474"));
    assert_eq!(projection.device_id.as_deref(), Some("desktop-1"));
    assert_eq!(projection.device_name.as_deref(), Some("Desktop"));
    assert_eq!(projection.tcp_port, 38474);
    assert_eq!(
        projection.result_name,
        "zsclip.settings_sync.pair_lan_device.pair_request_ready_on_macos_native_host"
    );
    let body = serde_json::from_str::<serde_json::Value>(
        projection.request_body_json.as_deref().unwrap_or_default(),
    )
    .expect("pair request body is json");
    assert_eq!(body["device_id"], "desktop-1");
    assert_eq!(body["name"], "Desktop");
    assert_eq!(body["tcp_port"], 38474);
    assert_eq!(body["capabilities"][0], "text");

    let missing_identity = settings_lan_pair_request_projection_from_json(
        SettingsAction::PairLanDevice,
        "linux_native_host",
        &serde_json::json!({
            "lan_sync_enabled": true,
            "lan_manual_host": "192.168.1.88",
        }),
    )
    .expect("missing identity pair projection");
    assert!(!missing_identity.accepted);
    assert_eq!(
        missing_identity.status,
        SettingsLanPairRequestProjectionStatus::MissingDeviceIdentity
    );
}

#[test]
fn settings_lan_pair_response_and_status_projection_parse_protocol_json() {
    let prefix = "zsclip.settings_sync.pair_lan_device.pair_request_ready_on_linux_native_host";
    let response = settings_lan_pair_request_response_projection(
        prefix,
        br#"{"pair_id":"pair-1","code":"123456","status":"pending"}"#,
    );
    assert!(response.accepted);
    assert_eq!(response.status, SettingsLanPairRequestResponseStatus::Sent);
    assert_eq!(response.pair_id.as_deref(), Some("pair-1"));
    assert_eq!(response.code.as_deref(), Some("123456"));
    assert_eq!(
        response.result_name,
        "zsclip.settings_sync.pair_lan_device.pair_request_ready_on_linux_native_host.sent"
    );

    let invalid = settings_lan_pair_request_response_projection(prefix, br#"{"status":"ok"}"#);
    assert!(!invalid.accepted);
    assert_eq!(
        invalid.status,
        SettingsLanPairRequestResponseStatus::InvalidResponse
    );

    let accepted = settings_lan_pair_status_projection(
        prefix,
        "192.168.1.88:38474",
        38473,
        9000,
        br#"{"status":"accepted","device_id":"desktop-2","name":"Desk 2","token":"tok","tcp_port":38475,"capabilities":["text","receive_clip"]}"#,
    );
    assert!(accepted.accepted);
    assert_eq!(
        accepted.status,
        SettingsLanPairStatusProjectionStatus::Accepted
    );
    let device = accepted
        .accepted_device
        .expect("accepted device projection");
    assert_eq!(device.device.device_id, "desktop-2");
    assert_eq!(device.device.name, "Desk 2");
    assert_eq!(device.device.endpoint, "192.168.1.88:38475");
    assert_eq!(device.token, "tok");
    assert_eq!(device.addr, "192.168.1.88");
    assert!(device.device.can_receive_clip);

    let pending = settings_lan_pair_status_projection(
        prefix,
        "192.168.1.88:38474",
        38473,
        0,
        br#"{"status":"pending"}"#,
    );
    assert!(!pending.accepted);
    assert_eq!(
        pending.status,
        SettingsLanPairStatusProjectionStatus::Pending
    );
}

#[test]
fn core_api_stays_platform_neutral() {
    let sources = [
        include_str!("../app_core.rs"),
        include_str!("components.rs"),
        include_str!("main_window.rs"),
        include_str!("../settings_model.rs"),
    ];
    let forbidden = [
        format!("{}{}", "windows", "_sys"),
        format!("{}{}", "HW", "ND"),
        format!("{}{}", "HD", "C"),
        format!("{}{}", "WP", "ARAM"),
        format!("{}{}", "LP", "ARAM"),
        format!("{}{}", "LR", "ESULT"),
        format!("{}{}", "WM", "_"),
        format!("{}{}", "VK", "_"),
        format!("{}{}", "IDC", "_SET_"),
        format!("{}{}", "crate::", "platform"),
        format!("{}{}", "crate::", "win_"),
        format!("{}{}", "Win", "UI"),
        format!("{}{}", "Seg", "oe"),
        format!("{}{}", "soft", "en"),
        format!("{}{}", "preserve", "_color"),
    ];
    for source in sources {
        for token in &forbidden {
            assert!(!source.contains(token), "{token}");
        }
    }
}

#[test]
fn zsui_framework_identity_is_explicit() {
    assert_eq!(ZSUI_FRAMEWORK_NAME, "ZSUI");
    assert!(ZSUI_FRAMEWORK_TAGLINE.contains("shared Rust UI logic"));
    assert!(ZSUI_FRAMEWORK_TAGLINE.contains("native platform hosts"));
}

#[test]
fn zsui_framework_manifest_is_single_reuse_entry_point() {
    let manifest = zsui_framework_manifest();

    assert_eq!(manifest.name, ZSUI_FRAMEWORK_NAME);
    assert_eq!(manifest.tagline, ZSUI_FRAMEWORK_TAGLINE);
    assert_eq!(manifest.api_version, APP_CORE_API_VERSION);
    assert_eq!(
        manifest
            .framework_layers
            .iter()
            .map(|layer| layer.layer_name())
            .collect::<Vec<_>>(),
        vec![
            "core_contracts",
            "layout_and_render_plans",
            "adapter_boundary",
            "native_host",
            "product_adapter"
        ]
    );
    assert_eq!(manifest.boundary_rules, zsui_framework_boundary_rules());
    assert_eq!(manifest.boundary_rules.len(), 5);
    assert!(manifest.boundary_rules.iter().any(|rule| rule.layer
        == ZsuiFrameworkLayer::AdapterBoundary
        && rule
            .allowed_modules
            .contains(&"src/macos_appkit_adapter.rs")
        && rule.allowed_modules.contains(&"src/linux_gtk_adapter.rs")
        && rule.must_not_own.contains(&"product command execution")
        && rule
            .must_not_own
            .contains(&"AppKit event-loop side effects")
        && rule.must_not_own.contains(&"GTK event-loop side effects")));
    assert!(manifest.boundary_rules.iter().any(|rule| rule.layer
        == ZsuiFrameworkLayer::NativeHost
        && rule.allowed_modules.contains(&"src/macos_native_host.rs")
        && rule.allowed_modules.contains(&"src/linux_native_host.rs")
        && rule.owns.contains(&"native windows")
        && rule.must_not_own.contains(&"row command semantics")));
    assert!(manifest.boundary_rules.iter().any(|rule| rule.layer
        == ZsuiFrameworkLayer::ProductAdapter
        && rule
            .allowed_modules
            .contains(&"src/zsclip_product_adapter.rs")
        && rule.owns.contains(&"product command execution")
        && rule.must_not_own.contains(&"native widget construction")));
    assert_eq!(
        manifest.native_feature_parity,
        zsui_native_feature_parity_statuses()
    );
    assert_eq!(
        manifest.user_feature_platform_statuses,
        zsui_user_feature_platform_statuses()
    );
    assert_eq!(
        manifest.user_feature_completion_summaries,
        zsui_user_feature_completion_summaries()
    );
    assert_eq!(
        manifest.user_feature_cross_platform_summaries,
        zsui_user_feature_cross_platform_summaries()
    );
    assert_eq!(
        manifest.native_ui_protocol_surfaces,
        native_ui_protocol_surfaces().to_vec()
    );
    assert_eq!(
        manifest.native_component_families,
        native_component_family_descriptors()
    );
    assert!(manifest.native_component_families.iter().any(|family| {
        family.surface_name == "settings_page"
            && family.action_family_name == "SettingsControl"
            && family.typed_spec_name == "NativeToggleSpec<NativeHostSettingsControlAction>"
            && family.spec_builder_name == "native_host_settings_toggle_specs"
    }));
    assert_eq!(
        manifest.native_ui_protocol_host_statuses,
        zsui_native_ui_protocol_host_statuses()
    );
    assert_eq!(manifest.native_feature_parity.len(), 75);
    assert_eq!(manifest.native_ui_protocol_host_statuses.len(), 15);
    assert!(manifest
        .native_ui_protocol_host_statuses
        .iter()
        .any(|status| status.platform == NativeUiPlatform::Windows
            && status.surface_name == "main_window"
            && status.source_coverage_verified
            && status.missing_protocol_builder_names.is_empty()
            && !status.target_smoke_verified
            && !status.system_complete));
    let required_total_control_features = [
        "right_click_edit_save",
        "right_click_copy",
        "right_click_paste",
        "right_click_delete",
        "right_click_pin",
        "right_click_group_assign_remove",
        "group_create_rename_delete_reorder_filter",
        "search_text_route",
        "vv_popup_select",
        "vv_paste",
        "settings_pages",
        "startup_autostart",
        "sync_webdav",
        "sync_lan",
        "status_menu",
        "dialog_input_confirm_edit",
        "clip_row_presentation_plan",
        "window_system_integration",
        "clipboard_text_payload",
        "clipboard_image_payload",
        "clipboard_file_path_payload",
    ];
    for platform in SUPPORTED_NATIVE_UI_PLATFORMS {
        for feature_name in required_total_control_features {
            let status = manifest
                .native_feature_parity
                .iter()
                .find(|status| status.platform == platform && status.feature_name == feature_name)
                .unwrap_or_else(|| {
                    panic!(
                        "missing native feature parity row for {} on {}",
                        feature_name,
                        platform.platform_name()
                    )
                });
            assert_eq!(
                status.support_status,
                ZsuiNativeFeatureSupportStatus::CodeLevelReadyPendingTargetSmoke
            );
            assert_eq!(
                status.support_status_name,
                status.support_status.status_name()
            );
            assert_eq!(
                status.code_level_ready,
                status.support_status.code_level_ready()
            );
            assert_eq!(
                status.target_smoke_required,
                status.support_status.target_smoke_required()
            );
            assert!(!status.target_smoke_verified);
            assert!(!status.system_complete);
            assert!(!status.support_status.is_explicitly_unsupported());
        }
    }
    let windows_copy = manifest
        .native_feature_parity
        .iter()
        .find(|status| {
            status.platform == NativeUiPlatform::Windows
                && status.feature_name == "right_click_copy"
        })
        .unwrap();
    assert!(windows_copy
        .missing_system_requirements
        .contains(&"Windows local release build and native smoke verification"));
    let macos_edit = manifest
        .native_feature_parity
        .iter()
        .find(|status| {
            status.platform == NativeUiPlatform::Macos
                && status.feature_name == "right_click_edit_save"
        })
        .unwrap();
    assert!(macos_edit.code_level_ready);
    assert!(macos_edit.target_smoke_required);
    assert!(!macos_edit.target_smoke_verified);
    assert!(!macos_edit.system_complete);
    assert!(macos_edit
        .missing_system_requirements
        .contains(&"target multiline native edit window, save refresh, and unsaved-change smoke"));
    let linux_vv_paste = manifest
        .native_feature_parity
        .iter()
        .find(|status| {
            status.platform == NativeUiPlatform::Linux && status.feature_name == "vv_paste"
        })
        .unwrap();
    assert!(linux_vv_paste.code_level_ready);
    assert!(linux_vv_paste
        .missing_system_requirements
        .contains(&"real OS paste shortcut delivery"));
    assert!(linux_vv_paste
        .missing_system_requirements
        .contains(&"target-window identity"));
    let identity_statuses = manifest
        .native_feature_parity
        .iter()
        .filter(|status| status.feature_name == "window_paste_target_identity")
        .collect::<Vec<_>>();
    assert_eq!(identity_statuses.len(), 3);
    assert!(identity_statuses
        .iter()
        .all(|status| status.code_level_ready && !status.system_complete));
    assert_eq!(
        manifest.native_platforms,
        vec![
            NativeUiPlatform::Windows,
            NativeUiPlatform::Macos,
            NativeUiPlatform::Linux
        ]
    );
    assert_eq!(
        manifest.native_toolkits,
        vec![
            NativeUiToolkit::Win32Gdi,
            NativeUiToolkit::AppKitSwiftUI,
            NativeUiToolkit::Gtk4Libadwaita
        ]
    );
    assert_eq!(
        manifest.native_backends,
        SUPPORTED_NATIVE_UI_BACKENDS.to_vec()
    );
    assert_eq!(
        manifest
            .native_platforms
            .iter()
            .map(|platform| platform.platform_name())
            .collect::<Vec<_>>(),
        vec!["windows", "macos", "linux"]
    );
    assert_eq!(
        manifest
            .native_toolkits
            .iter()
            .map(|toolkit| toolkit.toolkit_name())
            .collect::<Vec<_>>(),
        vec!["win32_gdi", "appkit_swiftui", "gtk4_libadwaita"]
    );
    assert_eq!(
        manifest.native_adapter_capabilities,
        REQUIRED_NATIVE_UI_ADAPTER_CAPABILITIES.to_vec()
    );
    assert_eq!(
        manifest
            .native_adapter_capabilities
            .iter()
            .map(|capability| capability.capability_name())
            .collect::<Vec<_>>(),
        vec![
            "main_window",
            "settings_window",
            "settings_dropdown",
            "input_dialog",
            "edit_dialog",
            "clipboard",
            "popup_menu",
            "status_item",
            "renderer",
            "text_layout",
            "main_search_control",
            "transient_window",
            "ime",
            "shell_open",
            "file_dialog",
            "paste_target",
            "window_identity",
            "main_execution_plan_bridge"
        ]
    );
    assert_eq!(
        native_ui_backend_for_platform(NativeUiPlatform::Windows)
            .unwrap()
            .adapter_boundary,
        "WindowsWin32AdapterBoundary"
    );
    assert_eq!(
        native_ui_backend_for_platform(NativeUiPlatform::Macos)
            .unwrap()
            .adapter_boundary,
        "MacosAppKitAdapterBoundary"
    );
    assert_eq!(
        native_ui_backend_for_platform(NativeUiPlatform::Linux)
            .unwrap()
            .adapter_boundary,
        "LinuxGtkAdapterBoundary"
    );
    assert_eq!(
        native_ui_backend_for_toolkit(NativeUiToolkit::Gtk4Libadwaita)
            .unwrap()
            .platform,
        NativeUiPlatform::Linux
    );
    assert!(native_ui_backend_for_platform(NativeUiPlatform::Windows)
        .unwrap()
        .status
        .is_native_runtime_ready());
    assert!(native_ui_backend_for_platform(NativeUiPlatform::Macos)
        .unwrap()
        .status
        .is_first_pass_native_host());
    assert!(native_ui_backend_for_platform(NativeUiPlatform::Linux)
        .unwrap()
        .status
        .is_first_pass_native_host());
    let current_backend = native_ui_backend_for_current_target().unwrap();
    #[cfg(target_os = "windows")]
    assert_eq!(current_backend.platform, NativeUiPlatform::Windows);
    #[cfg(target_os = "macos")]
    assert_eq!(current_backend.platform, NativeUiPlatform::Macos);
    #[cfg(target_os = "linux")]
    assert_eq!(current_backend.platform, NativeUiPlatform::Linux);
    assert_eq!(
        manifest
            .native_backends
            .iter()
            .map(|backend| {
                (
                    backend.platform,
                    backend.platform_name(),
                    backend.toolkit,
                    backend.toolkit_name(),
                    backend.status,
                    backend.status_name(),
                    backend.adapter_boundary,
                    backend.module_path,
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (
                NativeUiPlatform::Windows,
                "windows",
                NativeUiToolkit::Win32Gdi,
                "win32_gdi",
                NativeUiBackendStatus::NativeHostIntegrated,
                "native_host_integrated",
                "WindowsWin32AdapterBoundary",
                "src/windows_win32_adapter.rs"
            ),
            (
                NativeUiPlatform::Macos,
                "macos",
                NativeUiToolkit::AppKitSwiftUI,
                "appkit_swiftui",
                NativeUiBackendStatus::NativeHostFirstPass,
                "native_host_first_pass",
                "MacosAppKitAdapterBoundary",
                "src/macos_appkit_adapter.rs"
            ),
            (
                NativeUiPlatform::Linux,
                "linux",
                NativeUiToolkit::Gtk4Libadwaita,
                "gtk4_libadwaita",
                NativeUiBackendStatus::NativeHostFirstPass,
                "native_host_first_pass",
                "LinuxGtkAdapterBoundary",
                "src/linux_gtk_adapter.rs"
            )
        ]
    );
    assert_eq!(
        manifest.required_host_surfaces,
        REQUIRED_UI_HOST_SURFACES.to_vec()
    );
    assert_eq!(
        manifest.shared_non_host_protocols,
        SHARED_NON_HOST_UI_PROTOCOLS.to_vec()
    );
    assert_eq!(
        manifest.product_adapter.required_surfaces,
        REQUIRED_PRODUCT_ADAPTER_CONTRACT_SURFACES.to_vec()
    );
    assert_eq!(
        manifest
            .product_adapter
            .required_surfaces
            .iter()
            .map(|surface| surface.surface_name())
            .collect::<Vec<_>>(),
        vec![
            "product_identity",
            "product_state_model",
            "product_command_executor",
            "settings_model",
            "async_event_bridge",
            "ai_capability_catalog",
        ]
    );
    assert_eq!(
        manifest.product_adapter.ai.execution_routes,
        manifest.ai.execution_routes
    );
    assert_eq!(
        manifest.ai.total_capabilities,
        product_ai_capability_catalog().len()
    );
    assert_eq!(manifest.ai.llm_capabilities, 3);
    assert_eq!(manifest.ai.skill_capabilities, 2);
    assert_eq!(manifest.ai.product_adapter_capabilities, 2);
    assert_eq!(
        manifest.ai.providers,
        vec![
            ProductAiProviderKind::Llms,
            ProductAiProviderKind::Skills,
            ProductAiProviderKind::ProductAdapter
        ]
    );
    assert_eq!(
        manifest
            .ai
            .execution_routes
            .iter()
            .map(|route| route.capability_id)
            .collect::<Vec<_>>(),
        product_ai_capability_catalog()
            .iter()
            .map(|capability| capability.id)
            .collect::<Vec<_>>()
    );
    let matrix = native_ui_backend_capability_matrix();
    assert_eq!(matrix.len(), 3);
    assert!(matrix[0].native_runtime_ready());
    assert!(!matrix[1].native_runtime_ready());
    assert!(!matrix[1].scaffolded());
    assert!(matrix[1].backend.status.is_first_pass_native_host());
    assert!(!matrix[2].native_runtime_ready());
    assert!(!matrix[2].scaffolded());
    assert!(matrix[2].backend.status.is_first_pass_native_host());
    assert_eq!(
        matrix
            .iter()
            .map(|row| row.backend.platform_name())
            .collect::<Vec<_>>(),
        vec!["windows", "macos", "linux"]
    );
    assert_eq!(
        matrix[0].required_capabilities,
        REQUIRED_NATIVE_UI_ADAPTER_CAPABILITIES.to_vec()
    );
    assert!(matrix[0]
        .required_capability_names()
        .contains(&"main_execution_plan_bridge"));
    assert_eq!(
        native_ui_backend_capability_matrix_for_platform(NativeUiPlatform::Linux)
            .unwrap()
            .backend
            .toolkit_name(),
        "gtk4_libadwaita"
    );
}

#[test]
fn native_feature_matrix_has_explicit_platform_support_statuses() {
    let statuses = zsui_native_feature_parity_statuses();
    let payload_features = [
        "clipboard_text_payload",
        "clipboard_image_payload",
        "clipboard_file_path_payload",
    ];

    for platform in SUPPORTED_NATIVE_UI_PLATFORMS {
        let platform_statuses = statuses
            .iter()
            .filter(|status| status.platform == platform)
            .collect::<Vec<_>>();
        assert_eq!(platform_statuses.len(), 25);
        assert!(platform_statuses.iter().all(|status| {
            status.platform_name == platform.platform_name()
                && status.support_status_name == status.support_status.status_name()
                && status.code_level_ready == status.support_status.code_level_ready()
                && status.target_smoke_required == status.support_status.target_smoke_required()
                && status.target_smoke_verified == status.support_status.target_smoke_verified()
                && status.system_complete == status.support_status.system_complete()
        }));

        for feature_name in payload_features {
            let status = platform_statuses
                .iter()
                .find(|status| status.feature_name == feature_name)
                .unwrap_or_else(|| {
                    panic!(
                        "missing clipboard payload feature row for {} on {}",
                        feature_name,
                        platform.platform_name()
                    )
                });
            assert_eq!(
                status.support_status,
                ZsuiNativeFeatureSupportStatus::CodeLevelReadyPendingTargetSmoke
            );
            assert!(status.target_smoke_required);
            assert!(!status.system_complete);
        }

        let row_presentation = platform_statuses
            .iter()
            .find(|status| status.feature_name == "clip_row_presentation_plan")
            .unwrap_or_else(|| {
                panic!(
                    "missing row presentation feature row for {}",
                    platform.platform_name()
                )
            });
        assert_eq!(
            row_presentation.support_status,
            ZsuiNativeFeatureSupportStatus::CodeLevelReadyPendingTargetSmoke
        );
        assert!(row_presentation
            .missing_system_requirements
            .contains(&"host source proves row labels, kind icons, pin badges, and accessibility text consume NativeHostClipRowPresentation"));

        let window_system = platform_statuses
            .iter()
            .find(|status| status.feature_name == "window_system_integration")
            .unwrap_or_else(|| {
                panic!(
                    "missing window system integration row for {}",
                    platform.platform_name()
                )
            });
        assert_eq!(
            window_system.support_status,
            ZsuiNativeFeatureSupportStatus::CodeLevelReadyPendingTargetSmoke
        );
        assert!(window_system
            .missing_system_requirements
            .contains(&"target always-on-top smoke"));
        assert!(window_system
            .missing_system_requirements
            .contains(&"target DPI scale factor smoke"));
        if platform == NativeUiPlatform::Linux {
            assert!(window_system.missing_system_requirements.contains(
                &"target GTK X11 command backend smoke for keep-above and cursor-follow; Wayland layer-shell follow-up if compositor blocks window moves"
            ));
        }

        let startup_autostart = platform_statuses
            .iter()
            .find(|status| status.feature_name == "startup_autostart")
            .unwrap_or_else(|| {
                panic!(
                    "missing startup autostart row for {}",
                    platform.platform_name()
                )
            });
        assert_eq!(
            startup_autostart.support_status,
            ZsuiNativeFeatureSupportStatus::CodeLevelReadyPendingTargetSmoke
        );
        assert!(startup_autostart
            .missing_system_requirements
            .contains(&"target startup/autostart install smoke"));
        assert!(startup_autostart
            .missing_system_requirements
            .contains(&"native settings auto_start control toggles platform autostart host"));
    }

    assert!(!statuses
        .iter()
        .any(|status| status.feature_name == "clipboard_text_image_file_paths"));
}

#[test]
fn native_feature_matrix_marks_cross_platform_sync_truthfully() {
    for platform in SUPPORTED_NATIVE_UI_PLATFORMS {
        let webdav = zsui_native_feature_status_for(platform, "sync_webdav")
            .unwrap_or_else(|| panic!("webdav sync row for {}", platform.platform_name()));
        assert_eq!(
            webdav.support_status,
            ZsuiNativeFeatureSupportStatus::CodeLevelReadyPendingTargetSmoke
        );
        assert!(webdav.code_level_ready);
        assert!(webdav.target_smoke_required);
    }

    let windows_lan = zsui_native_feature_status_for(NativeUiPlatform::Windows, "sync_lan")
        .expect("windows LAN sync row");
    assert_eq!(
        windows_lan.support_status,
        ZsuiNativeFeatureSupportStatus::CodeLevelReadyPendingTargetSmoke
    );
    assert!(windows_lan.code_level_ready);

    for platform in [NativeUiPlatform::Macos, NativeUiPlatform::Linux] {
        let status = zsui_native_feature_status_for(platform, "sync_lan")
            .unwrap_or_else(|| panic!("LAN sync row for {}", platform.platform_name()));
        assert_eq!(
            status.support_status,
            ZsuiNativeFeatureSupportStatus::CodeLevelReadyPendingTargetSmoke
        );
        assert!(status.code_level_ready);
        assert!(status.target_smoke_required);
        assert!(!status
            .missing_system_requirements
            .contains(&SETTINGS_LAN_SYNC_RUNTIME_GAP));
    }
}

#[test]
fn lan_sync_capability_matrix_splits_partial_platform_progress() {
    let matrix = zsui_lan_sync_capability_matrix();
    assert_eq!(matrix.len(), SUPPORTED_NATIVE_UI_PLATFORMS.len() * 9);

    let windows = zsui_lan_sync_capabilities_for_platform(NativeUiPlatform::Windows);
    assert_eq!(windows.len(), 9);
    assert_eq!(
        windows
            .iter()
            .filter(|status| status.code_level_ready)
            .count(),
        9
    );
    assert!(windows.iter().all(|status| !status.blocks_sync_lan_release));

    for platform in [NativeUiPlatform::Macos, NativeUiPlatform::Linux] {
        let statuses = zsui_lan_sync_capabilities_for_platform(platform);
        assert_eq!(statuses.len(), 9);
        let ready = statuses
            .iter()
            .filter(|status| status.code_level_ready)
            .map(|status| status.capability_name)
            .collect::<Vec<_>>();
        assert_eq!(
            ready,
            vec![
                "mobile_link_projection",
                "device_book_projection",
                "manual_pair_request",
                "pair_status_accepted_device_save",
                "pair_approval_prompt",
                "service_discovery_runtime",
                "background_clip_sync_loop",
                "image_payload_transfer",
                "file_payload_transfer"
            ]
        );
        let blockers = statuses
            .iter()
            .filter(|status| status.blocks_sync_lan_release)
            .map(|status| status.capability_name)
            .collect::<Vec<_>>();
        assert!(blockers.is_empty());
        let accepted_save = statuses
            .iter()
            .find(|status| status.capability_name == "pair_status_accepted_device_save")
            .unwrap();
        assert!(accepted_save
            .evidence_names
            .contains(&"upsert_lan_device_in_store"));
        assert_eq!(
            accepted_save.support_status,
            ZsuiNativeFeatureSupportStatus::CodeLevelReadyPendingTargetSmoke
        );
        let file_transfer = statuses
            .iter()
            .find(|status| status.capability_name == "file_payload_transfer")
            .unwrap();
        assert_eq!(
            file_transfer.support_status,
            ZsuiNativeFeatureSupportStatus::CodeLevelReadyPendingTargetSmoke
        );
        assert!(file_transfer
            .evidence_names
            .contains(&"execute_lan_file_payload_transfer_once"));
        assert!(file_transfer.target_smoke_required);
    }

    let manifest = zsui_framework_manifest();
    assert_eq!(manifest.lan_sync_capability_matrix, matrix);
    let context = zsui_agent_context();
    assert_eq!(context.lan_sync_capability_matrix, matrix);
}

#[test]
fn window_system_capability_matrix_splits_linux_backend_gaps() {
    let matrix = zsui_window_system_capability_matrix();
    assert_eq!(matrix.len(), SUPPORTED_NATIVE_UI_PLATFORMS.len() * 5);

    for platform in [NativeUiPlatform::Windows, NativeUiPlatform::Macos] {
        let statuses = zsui_window_system_capabilities_for_platform(platform);
        assert_eq!(statuses.len(), 5);
        assert!(statuses.iter().all(|status| status.code_level_ready));
        assert!(statuses
            .iter()
            .all(|status| !status.partial_code_level_ready));
        assert!(statuses.iter().all(|status| status.target_smoke_required));
        assert!(statuses
            .iter()
            .all(|status| status.missing_backend_requirements.is_empty()));
        assert!(statuses
            .iter()
            .all(|status| status.implemented_backend_names.is_empty()));
    }

    let linux = zsui_window_system_capabilities_for_platform(NativeUiPlatform::Linux);
    assert_eq!(linux.len(), 5);
    let ready = linux
        .iter()
        .filter(|status| status.code_level_ready)
        .map(|status| status.capability_name)
        .collect::<Vec<_>>();
    assert_eq!(
        ready,
        vec![
            "window_show_hide_restore",
            "always_on_top",
            "dark_theme_detection",
            "dpi_scale_factor",
            "cursor_follow_positioning"
        ]
    );
    let missing = linux
        .iter()
        .filter(|status| !status.code_level_ready)
        .map(|status| status.capability_name)
        .collect::<Vec<_>>();
    assert!(missing.is_empty());
    let partial = linux
        .iter()
        .filter(|status| status.partial_code_level_ready)
        .map(|status| status.capability_name)
        .collect::<Vec<_>>();
    assert!(partial.is_empty());

    let always_on_top = linux
        .iter()
        .find(|status| status.capability_name == "always_on_top")
        .expect("Linux always-on-top capability row");
    assert_eq!(
        always_on_top.support_status_name,
        ZsuiNativeFeatureSupportStatus::CodeLevelReadyPendingTargetSmoke.status_name()
    );
    assert!(always_on_top.code_level_ready);
    assert!(!always_on_top.partial_code_level_ready);
    assert!(always_on_top
        .implemented_backend_names
        .contains(&"x11_command_tools"));
    assert!(always_on_top.missing_backend_requirements.is_empty());

    let cursor_follow = linux
        .iter()
        .find(|status| status.capability_name == "cursor_follow_positioning")
        .expect("Linux cursor-follow capability row");
    assert_eq!(
        cursor_follow.support_status_name,
        ZsuiNativeFeatureSupportStatus::CodeLevelReadyPendingTargetSmoke.status_name()
    );
    assert!(cursor_follow.code_level_ready);
    assert!(!cursor_follow.partial_code_level_ready);
    assert!(cursor_follow
        .implemented_backend_names
        .contains(&"x11_command_tools"));
    assert!(cursor_follow.missing_backend_requirements.is_empty());

    let manifest = zsui_framework_manifest();
    assert_eq!(manifest.window_system_capability_matrix, matrix);
    let context = zsui_agent_context();
    assert_eq!(context.window_system_capability_matrix, matrix);
}

#[test]
fn window_system_backend_work_items_clear_after_linux_x11_backend_lands() {
    let work_items = zsui_window_system_backend_work_items();
    assert!(work_items.is_empty());

    let manifest = zsui_framework_manifest();
    assert_eq!(manifest.window_system_backend_work_items, work_items);
    let context = zsui_agent_context();
    assert_eq!(context.window_system_backend_work_items, work_items);
}

#[test]
fn clipboard_mvp_matrix_groups_cross_platform_clipboard_loop_blockers() {
    let matrix = zsui_clipboard_mvp_feature_matrix();
    let p0_features = [
        "main_window_show_hide_and_restore",
        "clipboard_history_list_and_search",
        "row_context_copy_paste_delete_pin",
        "row_edit_save",
        "group_filter_and_assignment",
        "vv_popup_select_and_paste",
        "settings_open_edit_save",
    ];

    for platform in SUPPORTED_NATIVE_UI_PLATFORMS {
        let platform_rows = matrix
            .iter()
            .filter(|row| row.platform == platform)
            .collect::<Vec<_>>();
        assert_eq!(platform_rows.len(), 12);
        assert!(platform_rows
            .iter()
            .all(|row| row.platform_name == platform.platform_name()));

        for feature_name in p0_features {
            let row = platform_rows
                .iter()
                .find(|row| row.feature_name == feature_name)
                .unwrap_or_else(|| panic!("missing P0 MVP row {feature_name}"));
            assert_eq!(row.phase, ZsuiClipboardMvpPhase::P0SameClipboardLoop);
            assert_eq!(row.phase_name, "p0_same_clipboard_loop");
            assert!(row.blocks_cross_platform_mvp);
            assert!(row.code_level_ready);
            assert!(row.target_smoke_required);
            assert!(!row.system_complete);
            assert!(row.next_missing_requirement.is_some());
        }
    }

    let linux_vv = matrix
        .iter()
        .find(|row| {
            row.platform == NativeUiPlatform::Linux
                && row.feature_name == "vv_popup_select_and_paste"
        })
        .expect("linux VV MVP row must exist");
    assert_eq!(
        linux_vv.required_native_feature_names,
        vec!["vv_popup_select", "vv_paste"]
    );
    assert!(linux_vv
        .missing_system_requirements
        .contains(&"real OS paste shortcut delivery"));

    let windows_history = matrix
        .iter()
        .find(|row| {
            row.platform == NativeUiPlatform::Windows
                && row.feature_name == "clipboard_history_list_and_search"
        })
        .expect("windows history/search MVP row must exist");
    assert!(windows_history
        .required_native_feature_names
        .contains(&"clip_row_presentation_plan"));

    let linux_webdav = matrix
        .iter()
        .find(|row| row.platform == NativeUiPlatform::Linux && row.feature_name == "sync_webdav")
        .expect("linux WebDAV sync MVP row must exist");
    assert!(linux_webdav.code_level_ready);
    assert!(linux_webdav.target_smoke_required);

    let linux_settings = matrix
        .iter()
        .find(|row| {
            row.platform == NativeUiPlatform::Linux && row.feature_name == "settings_open_edit_save"
        })
        .expect("linux settings MVP row must exist");
    assert!(linux_settings
        .required_native_feature_names
        .contains(&"startup_autostart"));

    let linux_lan = matrix
        .iter()
        .find(|row| row.platform == NativeUiPlatform::Linux && row.feature_name == "sync_lan")
        .expect("linux LAN sync MVP row must exist");
    assert!(linux_lan.code_level_ready);
    assert!(linux_lan.target_smoke_required);
    assert_eq!(
        linux_lan.next_missing_requirement,
        Some("target settings sync toggle smoke")
    );
}

#[test]
fn zsui_reuse_readiness_report_names_platform_ai_and_product_boundaries() {
    let report = zsui_reuse_readiness_report();

    assert_eq!(report.platform_names, vec!["windows", "macos", "linux"]);
    assert_eq!(report.native_runtime_ready_platforms, vec!["windows"]);
    assert_eq!(
        report.first_pass_native_host_platforms,
        vec!["macos", "linux"]
    );
    assert!(report.scaffold_platforms.is_empty());
    assert_eq!(
        report.product_adapter_surface_names,
        vec![
            "product_identity",
            "product_state_model",
            "product_command_executor",
            "settings_model",
            "async_event_bridge",
            "ai_capability_catalog"
        ]
    );
    assert_eq!(
        report.ai_provider_names,
        vec!["llms", "skills", "product_adapter"]
    );
    assert_eq!(
        report.ai_executor_boundary_names,
        vec!["llm_executor", "skill_registry", "product_adapter_tools"]
    );
    assert_eq!(
        report.product_adapter_task_names,
        vec![
            "provide_product_identity",
            "project_product_state",
            "execute_product_commands",
            "bind_settings_model",
            "bridge_async_events",
            "publish_ai_catalog",
            "connect_llm_executor",
            "connect_skill_registry",
            "connect_product_ai_tools"
        ]
    );
    assert!(report
        .native_adapter_capability_names
        .contains(&"settings_window"));
    assert!(report
        .native_adapter_capability_names
        .contains(&"main_execution_plan_bridge"));
    assert_eq!(report.adapter_parity, None);
}

#[test]
fn zsui_reuse_bootstrap_plan_combines_platform_product_and_ai_requirements() {
    let windows = zsui_reuse_bootstrap_plan(NativeUiPlatform::Windows).unwrap();
    let macos = zsui_reuse_bootstrap_plan(NativeUiPlatform::Macos).unwrap();
    let linux = zsui_reuse_bootstrap_plan(NativeUiPlatform::Linux).unwrap();

    assert_eq!(windows.platform, NativeUiPlatform::Windows);
    assert_eq!(windows.platform_name, "windows");
    assert_eq!(windows.toolkit_name, "win32_gdi");
    assert_eq!(
        windows.backend_status,
        NativeUiBackendStatus::NativeHostIntegrated
    );
    assert_eq!(windows.backend_status_name, "native_host_integrated");
    assert!(windows.native_runtime_ready());
    assert!(!windows.scaffolded());
    assert_eq!(windows.adapter_boundary, "WindowsWin32AdapterBoundary");
    assert_eq!(windows.adapter_module_path, "src/windows_win32_adapter.rs");
    assert!(windows
        .native_adapter_capability_names
        .contains(&"main_window"));
    assert!(windows
        .native_adapter_capability_names
        .contains(&"main_execution_plan_bridge"));
    assert_eq!(
        windows.product_adapter_surface_names,
        vec![
            "product_identity",
            "product_state_model",
            "product_command_executor",
            "settings_model",
            "async_event_bridge",
            "ai_capability_catalog"
        ]
    );
    assert_eq!(
        windows.product_adapter_task_names,
        vec![
            "provide_product_identity",
            "project_product_state",
            "execute_product_commands",
            "bind_settings_model",
            "bridge_async_events",
            "publish_ai_catalog",
            "connect_llm_executor",
            "connect_skill_registry",
            "connect_product_ai_tools"
        ]
    );
    assert_eq!(
        windows.ai_provider_names,
        vec!["llms", "skills", "product_adapter"]
    );
    assert_eq!(
        windows.ai_executor_boundary_names,
        vec!["llm_executor", "skill_registry", "product_adapter_tools"]
    );
    assert_eq!(
        windows.native_runtime_gate_names,
        vec![
            "native_event_loop",
            "native_window_surfaces",
            "native_control_mapping",
            "native_renderer",
            "native_clipboard_services",
            "native_dialog_services",
            "native_settings_surfaces",
            "ai_action_presentation"
        ]
    );
    assert!(windows.missing_native_runtime_gate_names.is_empty());
    assert_eq!(windows.next_native_runtime_gate_name, None);
    assert_eq!(windows.native_runtime_gate_completion.total_gate_count, 8);
    assert_eq!(
        windows.native_runtime_gate_completion.completed_gate_count,
        8
    );
    assert_eq!(
        windows.native_runtime_gate_completion.completed_gate_names,
        windows.native_runtime_gate_names
    );
    assert_eq!(windows.native_runtime_gate_completion.missing_gate_count, 0);
    assert_eq!(
        windows.native_runtime_gate_completion.completion_percent,
        100
    );
    assert_eq!(
        windows.native_runtime_gate_completion.missing_gate_names,
        Vec::<&'static str>::new()
    );
    assert_eq!(windows.native_runtime_gate_completion.next_gate_name, None);
    assert_eq!(windows.native_runtime_gate_plans.len(), 8);
    assert_eq!(windows.native_runtime_gate_binding_plans.len(), 8);
    let windows_event_loop_gate = windows
        .native_runtime_gate_plans
        .iter()
        .find(|gate| gate.gate_name == "native_event_loop")
        .unwrap();
    assert_eq!(
        windows_event_loop_gate.required_adapter_capability_names,
        vec!["main_window", "status_item", "main_execution_plan_bridge"]
    );
    let windows_event_loop_binding_gate = windows
        .native_runtime_gate_binding_plans
        .iter()
        .find(|gate| gate.gate_name == "native_event_loop")
        .unwrap();
    assert_eq!(
        windows_event_loop_binding_gate.required_platform_binding_names,
        vec![
            "win32_main_window_pair",
            "shell_notify_icon_status_item",
            "shared_main_execution_plan_bridge"
        ]
    );
    let windows_renderer_gate = windows
        .native_runtime_gate_plans
        .iter()
        .find(|gate| gate.gate_name == "native_renderer")
        .unwrap();
    assert_eq!(
        windows_renderer_gate.required_adapter_capability_names,
        vec!["renderer", "text_layout"]
    );
    assert!(windows_renderer_gate
        .required_product_adapter_task_names
        .is_empty());
    let windows_renderer_binding_gate = windows
        .native_runtime_gate_binding_plans
        .iter()
        .find(|gate| gate.gate_name == "native_renderer")
        .unwrap();
    assert_eq!(
        windows_renderer_binding_gate.required_platform_binding_names,
        vec!["gdi_renderer", "gdi_text_layout"]
    );
    let windows_ai_gate = windows
        .native_runtime_gate_plans
        .iter()
        .find(|gate| gate.gate_name == "ai_action_presentation")
        .unwrap();
    assert_eq!(
        windows_ai_gate.required_adapter_capability_names,
        vec![
            "popup_menu",
            "settings_window",
            "main_execution_plan_bridge"
        ]
    );
    assert_eq!(
        windows_ai_gate.required_product_adapter_task_names,
        vec![
            "publish_ai_catalog",
            "connect_llm_executor",
            "connect_skill_registry",
            "connect_product_ai_tools"
        ]
    );
    assert_eq!(
        windows_ai_gate.required_ai_executor_boundary_names,
        vec!["llm_executor", "skill_registry", "product_adapter_tools"]
    );
    let windows_ai_binding_gate = windows
        .native_runtime_gate_binding_plans
        .iter()
        .find(|gate| gate.gate_name == "ai_action_presentation")
        .unwrap();
    assert_eq!(
        windows_ai_binding_gate.required_platform_binding_names,
        vec![
            "win32_popup_menu_host",
            "win32_settings_window",
            "shared_main_execution_plan_bridge"
        ]
    );
    assert_eq!(
        windows_ai_binding_gate.required_product_adapter_task_names,
        windows_ai_gate.required_product_adapter_task_names
    );

    assert_eq!(macos.platform_name, "macos");
    assert_eq!(macos.toolkit_name, "appkit_swiftui");
    assert_eq!(macos.adapter_boundary, "MacosAppKitAdapterBoundary");
    assert!(!macos.native_runtime_ready());
    assert!(!macos.scaffolded());
    assert!(macos.backend_status.is_first_pass_native_host());
    assert_eq!(
        macos.native_runtime_gate_completion.completed_gate_names,
        vec![
            "native_event_loop",
            "native_window_surfaces",
            "native_control_mapping",
            "native_renderer",
            "native_clipboard_services",
            "native_dialog_services",
            "native_settings_surfaces",
            "ai_action_presentation"
        ]
    );
    assert!(macos.missing_native_runtime_gate_names.is_empty());
    assert_eq!(macos.next_native_runtime_gate_name, None);
    assert_eq!(macos.native_runtime_gate_completion.total_gate_count, 8);
    assert_eq!(macos.native_runtime_gate_completion.completed_gate_count, 8);
    assert_eq!(macos.native_runtime_gate_completion.missing_gate_count, 0);
    assert_eq!(macos.native_runtime_gate_completion.completion_percent, 100);
    assert_eq!(macos.native_runtime_gate_completion.next_gate_name, None);
    assert_eq!(
        macos.native_runtime_gate_plans[0].required_adapter_capability_names,
        vec!["main_window", "status_item", "main_execution_plan_bridge"]
    );
    assert_eq!(
        macos.native_runtime_gate_binding_plans[0].required_platform_binding_names,
        vec![
            "ns_window_pair",
            "ns_status_item_bridge",
            "shared_main_execution_plan_bridge"
        ]
    );
    assert_eq!(linux.platform_name, "linux");
    assert_eq!(linux.toolkit_name, "gtk4_libadwaita");
    assert_eq!(linux.adapter_boundary, "LinuxGtkAdapterBoundary");
    assert!(!linux.native_runtime_ready());
    assert!(!linux.scaffolded());
    assert!(linux.backend_status.is_first_pass_native_host());
    assert_eq!(
        linux.native_runtime_gate_completion.completed_gate_names,
        vec![
            "native_event_loop",
            "native_window_surfaces",
            "native_control_mapping",
            "native_renderer",
            "native_clipboard_services",
            "native_dialog_services",
            "native_settings_surfaces",
            "ai_action_presentation"
        ]
    );
    assert!(linux.missing_native_runtime_gate_names.is_empty());
    assert_eq!(linux.next_native_runtime_gate_name, None);
    assert_eq!(linux.native_runtime_gate_completion.total_gate_count, 8);
    assert_eq!(linux.native_runtime_gate_completion.completed_gate_count, 8);
    assert_eq!(linux.native_runtime_gate_completion.missing_gate_count, 0);
    assert_eq!(linux.native_runtime_gate_completion.completion_percent, 100);
    assert!(linux
        .native_runtime_gate_completion
        .missing_gate_names
        .is_empty());
    let linux_renderer_binding_gate = linux
        .native_runtime_gate_binding_plans
        .iter()
        .find(|gate| gate.gate_name == "native_renderer")
        .unwrap();
    assert_eq!(
        linux_renderer_binding_gate.required_platform_binding_names,
        vec!["gtk_snapshot_renderer", "pango_text_layout"]
    );
}

#[test]
fn native_ui_adapter_reuse_packages_report_three_platform_parity() {
    let packages = vec![
        crate::windows_win32_adapter::WindowsWin32AdapterBoundary::default_from_core_contract()
            .reuse_package(),
        crate::macos_appkit_adapter::MacosAppKitAdapterBoundary::default_from_macos_contract()
            .reuse_package(),
        crate::linux_gtk_adapter::LinuxGtkAdapterBoundary::default_from_linux_contract()
            .reuse_package(),
    ];
    for package in &packages {
        for gate in &package.bootstrap.native_runtime_gate_binding_plans {
            assert!(gate
                .required_platform_binding_names
                .iter()
                .all(|binding_name| package.binding_plan.has_binding_name(binding_name)));
        }
    }
    assert!(packages[1]
        .bootstrap
        .native_runtime_gate_binding_plans
        .iter()
        .find(|gate| gate.gate_name == "native_renderer")
        .unwrap()
        .required_platform_binding_names
        .contains(&"core_graphics_renderer"));
    assert!(packages[2]
        .bootstrap
        .native_runtime_gate_binding_plans
        .iter()
        .find(|gate| gate.gate_name == "native_clipboard_services")
        .unwrap()
        .required_platform_binding_names
        .contains(&"gdk_clipboard_bridge"));
    let gate_summaries = zsui_adapter_reuse_package_gate_binding_summaries(&packages);
    assert_eq!(
        gate_summaries
            .iter()
            .map(|summary| (
                summary.platform_name,
                summary.toolkit_name,
                summary.status_name,
                summary.adapter_boundary,
                summary.completion_percent,
                summary.next_gate_name,
                summary.all_gate_bindings_present_in_adapter,
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                "windows",
                "win32_gdi",
                "native_host_integrated",
                "WindowsWin32AdapterBoundary",
                100,
                None,
                true
            ),
            (
                "macos",
                "appkit_swiftui",
                "native_host_first_pass",
                "MacosAppKitAdapterBoundary",
                100,
                None,
                true
            ),
            (
                "linux",
                "gtk4_libadwaita",
                "native_host_first_pass",
                "LinuxGtkAdapterBoundary",
                100,
                None,
                true
            )
        ]
    );
    assert_eq!(gate_summaries[0].gate_names.len(), 8);
    assert_eq!(
        gate_summaries[0].gate_binding_counts,
        vec![3, 6, 5, 2, 3, 5, 2, 3]
    );
    assert!(gate_summaries[0].missing_gate_names.is_empty());
    assert!(gate_summaries[1].missing_gate_names.is_empty());
    let work_items = zsui_adapter_reuse_package_porting_work_items(&packages);
    assert!(work_items.is_empty());
    let report = native_ui_adapter_parity_report(&packages);

    assert_eq!(report.platform_names, vec!["windows", "macos", "linux"]);
    assert_eq!(
        report.toolkit_names,
        vec!["win32_gdi", "appkit_swiftui", "gtk4_libadwaita"]
    );
    assert_eq!(
        report.status_names,
        vec![
            "native_host_integrated",
            "native_host_first_pass",
            "native_host_first_pass"
        ]
    );
    assert_eq!(
        report.adapter_boundaries,
        vec![
            "WindowsWin32AdapterBoundary",
            "MacosAppKitAdapterBoundary",
            "LinuxGtkAdapterBoundary"
        ]
    );
    assert_eq!(report.binding_counts, vec![27, 27, 27]);
    assert_eq!(report.main_execution_plan_counts, vec![5, 5, 5]);
    assert_eq!(report.shared_non_host_protocol_counts, vec![3, 3, 3]);
    assert_eq!(report.native_runtime_ready_platforms, vec!["windows"]);
    assert_eq!(
        report.first_pass_native_host_platforms,
        vec!["macos", "linux"]
    );
    assert!(report.scaffold_platforms.is_empty());
    assert!(report.all_binding_counts_match_manifest);
    assert!(report.all_main_execution_plan_counts_match);
    assert!(report.all_shared_non_host_protocol_counts_match);

    let readiness = zsui_reuse_readiness_report_with_adapter_parity(&packages);
    let parity = readiness.adapter_parity.unwrap();
    assert_eq!(parity.platform_names, vec!["windows", "macos", "linux"]);
    assert_eq!(parity.binding_counts, vec![27, 27, 27]);
    assert!(parity.all_binding_counts_match_manifest);
}

#[test]
fn zsui_agent_context_summarizes_readiness_parity_and_ai_routes() {
    let packages = vec![
        crate::windows_win32_adapter::WindowsWin32AdapterBoundary::default_from_core_contract()
            .reuse_package(),
        crate::macos_appkit_adapter::MacosAppKitAdapterBoundary::default_from_macos_contract()
            .reuse_package(),
        crate::linux_gtk_adapter::LinuxGtkAdapterBoundary::default_from_linux_contract()
            .reuse_package(),
    ];
    let context = zsui_agent_context_with_adapter_parity(&packages);

    assert_eq!(context.framework_name, ZSUI_FRAMEWORK_NAME);
    assert_eq!(context.api_version, APP_CORE_API_VERSION);
    assert_eq!(context.framework_layers, zsui_framework_layers());
    assert_eq!(context.boundary_rules, zsui_framework_boundary_rules());
    assert_eq!(
        context.native_feature_parity,
        zsui_native_feature_parity_statuses()
    );
    assert_eq!(
        context.clipboard_mvp_feature_matrix,
        zsui_clipboard_mvp_feature_matrix()
    );
    assert_eq!(
        context.user_feature_platform_statuses,
        zsui_user_feature_platform_statuses()
    );
    assert_eq!(
        context.user_feature_completion_summaries,
        zsui_user_feature_completion_summaries()
    );
    assert_eq!(
        context.user_feature_cross_platform_summaries,
        zsui_user_feature_cross_platform_summaries()
    );
    assert_eq!(
        context.user_feature_work_items,
        zsui_user_feature_work_items()
    );
    assert!(context
        .clipboard_mvp_feature_matrix
        .iter()
        .any(
            |row| row.feature_name == "row_context_copy_paste_delete_pin"
                && row.phase == ZsuiClipboardMvpPhase::P0SameClipboardLoop
                && row.blocks_cross_platform_mvp
        ));
    assert_eq!(
        context.native_ui_protocol_surfaces,
        native_ui_protocol_surfaces().to_vec()
    );
    assert_eq!(
        context.native_component_families,
        native_component_family_descriptors()
    );
    assert!(context.native_component_families.iter().any(|family| {
        family.surface_name == "dynamic_controls"
            && family.action_family_name == "ClipRow"
            && family.typed_spec_name == "NativeClipRowSpec"
            && family.dynamic
    }));
    assert_eq!(
        context.native_ui_protocol_host_statuses,
        zsui_native_ui_protocol_host_statuses()
    );
    assert!(context
        .native_ui_protocol_host_statuses
        .iter()
        .any(|status| status.platform == NativeUiPlatform::Windows
            && status.surface_name == "settings_page"
            && status.source_coverage_verified
            && status.missing_protocol_builder_names.is_empty()
            && !status.target_smoke_verified
            && !status.system_complete));
    assert!(context
        .native_ui_protocol_host_statuses
        .iter()
        .any(|status| status.platform == NativeUiPlatform::Macos
            && status.surface_name == "menu"
            && status
                .protocol_builder_names
                .contains(&"native_host_status_menu_item_specs")
            && !status.system_complete));
    assert!(context
        .native_ui_protocol_surfaces
        .iter()
        .any(
            |surface| surface.kind == NativeUiProtocolSurfaceKind::SettingsPage
                && surface
                    .protocol_builder_names
                    .contains(&"native_host_settings_group_button_specs")
                && surface.action_family_names.contains(&"SettingsGroup")
        ));
    assert!(context
        .native_feature_parity
        .iter()
        .any(|status| status.platform == NativeUiPlatform::Linux
            && status.feature_name == "status_menu"
            && status.code_level_ready
            && status
                .missing_system_requirements
                .contains(&"target StatusNotifierHost smoke")
            && !status.system_complete));
    assert!(context
        .boundary_rules
        .iter()
        .any(|rule| rule.layer == ZsuiFrameworkLayer::CoreContracts
            && rule.owner_name == "app_core_contracts"
            && rule.must_not_own.contains(&"ZSClip database access")));
    assert!(context.boundary_rules.iter().any(|rule| rule.layer
        == ZsuiFrameworkLayer::LayoutAndRenderPlans
        && rule.owns.contains(&"row and VV action plans")
        && rule.must_not_own.contains(&"native clipboard writes")));
    assert_eq!(
        context.readiness.platform_names,
        vec!["windows", "macos", "linux"]
    );
    assert_eq!(context.platform_bootstrap.len(), 3);
    assert_eq!(
        context
            .platform_bootstrap
            .iter()
            .map(|platform| (
                platform.platform_name,
                platform.toolkit_name,
                platform.backend_status_name,
                platform.adapter_boundary,
                platform.adapter_module_path,
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                "windows",
                "win32_gdi",
                "native_host_integrated",
                "WindowsWin32AdapterBoundary",
                "src/windows_win32_adapter.rs"
            ),
            (
                "macos",
                "appkit_swiftui",
                "native_host_first_pass",
                "MacosAppKitAdapterBoundary",
                "src/macos_appkit_adapter.rs"
            ),
            (
                "linux",
                "gtk4_libadwaita",
                "native_host_first_pass",
                "LinuxGtkAdapterBoundary",
                "src/linux_gtk_adapter.rs"
            )
        ]
    );
    assert!(context.platform_bootstrap[0]
        .native_adapter_capability_names
        .contains(&"main_window"));
    assert_eq!(context.platform_runtime_gates.len(), 3);
    assert_eq!(
        context
            .platform_runtime_gates
            .iter()
            .map(|gate| (
                gate.platform_name,
                gate.toolkit_name,
                gate.backend_status_name,
                gate.native_runtime_ready,
                gate.next_gate_name,
            ))
            .collect::<Vec<_>>(),
        vec![
            ("windows", "win32_gdi", "native_host_integrated", true, None),
            (
                "macos",
                "appkit_swiftui",
                "native_host_first_pass",
                false,
                None
            ),
            (
                "linux",
                "gtk4_libadwaita",
                "native_host_first_pass",
                false,
                None
            )
        ]
    );
    assert_eq!(
        context.platform_runtime_gates[0].gate_names,
        vec![
            "native_event_loop",
            "native_window_surfaces",
            "native_control_mapping",
            "native_renderer",
            "native_clipboard_services",
            "native_dialog_services",
            "native_settings_surfaces",
            "ai_action_presentation"
        ]
    );
    assert_eq!(context.platform_runtime_gates[0].gate_plans.len(), 8);
    assert_eq!(
        context.platform_runtime_gates[0].gate_binding_plans.len(),
        8
    );
    assert_eq!(
        context.platform_runtime_gates[0]
            .completion
            .completion_percent,
        100
    );
    assert_eq!(
        context.platform_runtime_gates[1]
            .completion
            .completion_percent,
        100
    );
    assert_eq!(
        context.platform_runtime_gates[1]
            .completion
            .completed_gate_names,
        vec![
            "native_event_loop",
            "native_window_surfaces",
            "native_control_mapping",
            "native_renderer",
            "native_clipboard_services",
            "native_dialog_services",
            "native_settings_surfaces",
            "ai_action_presentation"
        ]
    );
    assert_eq!(
        context.platform_runtime_gates[1].completion.next_gate_name,
        None
    );
    assert_eq!(
        context.platform_runtime_gates[2]
            .completion
            .missing_gate_count,
        0
    );
    assert_eq!(
        context.platform_runtime_gates[1].gate_plans[0].required_adapter_capability_names,
        vec!["main_window", "status_item", "main_execution_plan_bridge"]
    );
    assert_eq!(
        context.platform_runtime_gates[1].gate_binding_plans[0].required_platform_binding_names,
        vec![
            "ns_window_pair",
            "ns_status_item_bridge",
            "shared_main_execution_plan_bridge"
        ]
    );
    assert_eq!(
        context.platform_runtime_gates[2].gate_plans[7].required_ai_executor_boundary_names,
        vec!["llm_executor", "skill_registry", "product_adapter_tools"]
    );
    assert_eq!(
        context.platform_runtime_gates[2].gate_binding_plans[7].required_platform_binding_names,
        vec![
            "gtk_popover_menu",
            "adw_preferences_window",
            "shared_main_execution_plan_bridge"
        ]
    );
    assert!(context.platform_runtime_gates[0]
        .missing_gate_names
        .is_empty());
    assert!(context.platform_runtime_gates[1]
        .missing_gate_names
        .is_empty());
    assert!(context.platform_runtime_gates[2]
        .missing_gate_names
        .is_empty());
    assert!(context.porting_work_items.is_empty());
    assert_eq!(
        context.reusable_app_blueprint.rust_ui_language_name,
        "zsui_rust_ui_contract"
    );
    assert_eq!(
        context.reusable_app_blueprint.native_platform_names,
        vec!["windows", "macos", "linux"]
    );
    assert_eq!(
        context
            .reusable_app_blueprint
            .native_runtime_driver_operation_names,
        vec![
            "start_runtime",
            "dispatch_ui_command",
            "poll_application_event",
            "request_shutdown"
        ]
    );
    assert_eq!(
        context.reusable_app_blueprint.runtime_harness_stage_names,
        vec![
            "start_runtime",
            "dispatch_command",
            "bridge_application_event",
            "execute_ai_invocation",
            "request_shutdown"
        ]
    );
    assert_eq!(
        context.reusable_app_blueprint.reusable_feature_names,
        vec![
            "native_app_entry",
            "window_surfaces",
            "control_mapping",
            "renderer_text_layout",
            "system_services",
            "settings_surfaces",
            "ai_action_surfaces"
        ]
    );
    assert_eq!(
        context.reusable_app_blueprint.product_adapter_surface_names,
        vec![
            "product_identity",
            "product_state_model",
            "product_command_executor",
            "settings_model",
            "async_event_bridge",
            "ai_capability_catalog"
        ]
    );
    assert_eq!(
        context.reusable_app_blueprint.product_adapter_method_names,
        vec![
            "product_identity",
            "project_product_state",
            "execute_product_command",
            "bind_settings_model",
            "bridge_async_event",
            "publish_ai_catalog",
            "execute_ai_plan"
        ]
    );
    assert_eq!(
        context
            .reusable_app_blueprint
            .product_function_flows
            .iter()
            .map(|flow| flow.flow_name)
            .collect::<Vec<_>>(),
        vec![
            "app_bootstrap",
            "state_projection",
            "user_command",
            "settings_sync",
            "async_event",
            "ai_action"
        ]
    );
    let command_flow = context
        .reusable_app_blueprint
        .product_function_flows
        .iter()
        .find(|flow| flow.flow_name == "user_command")
        .unwrap();
    assert_eq!(
        command_flow.required_surface_names,
        vec!["product_command_executor"]
    );
    assert_eq!(
        command_flow.required_task_names,
        vec!["execute_product_commands"]
    );
    let ai_flow = context
        .reusable_app_blueprint
        .product_function_flows
        .iter()
        .find(|flow| flow.flow_name == "ai_action")
        .unwrap();
    assert_eq!(
        ai_flow.required_surface_names,
        vec!["ai_capability_catalog", "product_command_executor"]
    );
    assert_eq!(
        ai_flow.required_ai_executor_boundary_names,
        vec!["llm_executor", "skill_registry", "product_adapter_tools"]
    );
    assert_eq!(
        context
            .reusable_app_blueprint
            .product_execution_pipeline
            .iter()
            .map(|stage| stage.stage_name)
            .collect::<Vec<_>>(),
        vec![
            "receive_ui_intent",
            "project_product_state",
            "execute_product_command",
            "bridge_async_event",
            "route_ai_action",
            "project_ui_update"
        ]
    );
    let execute_stage = context
        .reusable_app_blueprint
        .product_execution_pipeline
        .iter()
        .find(|stage| stage.stage_name == "execute_product_command")
        .unwrap();
    assert_eq!(
        execute_stage.required_surface_names,
        vec!["product_command_executor"]
    );
    assert_eq!(
        execute_stage.required_task_names,
        vec!["execute_product_commands"]
    );
    let route_ai_stage = context
        .reusable_app_blueprint
        .product_execution_pipeline
        .iter()
        .find(|stage| stage.stage_name == "route_ai_action")
        .unwrap();
    assert_eq!(
        route_ai_stage.required_surface_names,
        vec!["ai_capability_catalog", "product_command_executor"]
    );
    assert_eq!(
        route_ai_stage.required_task_names,
        vec![
            "publish_ai_catalog",
            "connect_llm_executor",
            "connect_skill_registry",
            "connect_product_ai_tools"
        ]
    );
    assert_eq!(
        route_ai_stage.required_ai_executor_boundary_names,
        vec!["llm_executor", "skill_registry", "product_adapter_tools"]
    );
    assert_eq!(
        context.reusable_app_blueprint.ai_executor_boundary_names,
        vec!["llm_executor", "skill_registry", "product_adapter_tools"]
    );
    assert_eq!(context.reusable_app_blueprint.feature_statuses.len(), 21);
    let windows_entry = context
        .reusable_app_blueprint
        .feature_statuses
        .iter()
        .find(|status| {
            status.platform_name == "windows" && status.feature_name == "native_app_entry"
        })
        .unwrap();
    assert!(windows_entry.runtime_ready);
    assert_eq!(windows_entry.runtime_status_name, "native_host_integrated");
    assert_eq!(
        windows_entry.required_platform_binding_names,
        vec![
            "win32_main_window_pair",
            "shell_notify_icon_status_item",
            "shared_main_execution_plan_bridge"
        ]
    );
    let macos_system_services = context
        .reusable_app_blueprint
        .feature_statuses
        .iter()
        .find(|status| status.platform_name == "macos" && status.feature_name == "system_services")
        .unwrap();
    assert!(!macos_system_services.runtime_ready);
    assert_eq!(
        macos_system_services.blocking_runtime_gate_names,
        Vec::<&'static str>::new()
    );
    assert!(macos_system_services
        .required_platform_binding_names
        .contains(&"ns_pasteboard_bridge"));
    assert!(macos_system_services
        .required_platform_binding_names
        .contains(&"ns_open_panel"));
    let linux_ai_surfaces = context
        .reusable_app_blueprint
        .feature_statuses
        .iter()
        .find(|status| {
            status.platform_name == "linux" && status.feature_name == "ai_action_surfaces"
        })
        .unwrap();
    assert_eq!(
        linux_ai_surfaces.required_platform_binding_names,
        vec![
            "gtk_popover_menu",
            "adw_preferences_window",
            "shared_main_execution_plan_bridge"
        ]
    );
    assert_eq!(
        linux_ai_surfaces.required_product_adapter_task_names,
        vec![
            "publish_ai_catalog",
            "connect_llm_executor",
            "connect_skill_registry",
            "connect_product_ai_tools"
        ]
    );
    assert_eq!(
        linux_ai_surfaces.required_ai_executor_boundary_names,
        vec!["llm_executor", "skill_registry", "product_adapter_tools"]
    );
    assert_eq!(context.integration_steps.len(), 7);
    assert_eq!(
        context
            .integration_steps
            .iter()
            .map(|step| (step.step_name, step.owner_name))
            .collect::<Vec<_>>(),
        vec![
            ("select_native_adapter", "native_adapter"),
            ("verify_adapter_capability_parity", "native_adapter"),
            ("implement_product_adapter_surfaces", "product_adapter"),
            ("complete_product_adapter_tasks", "product_adapter"),
            ("connect_llm_executor", "ai_executor"),
            ("connect_skill_registry", "ai_executor"),
            ("connect_product_ai_tools", "ai_executor")
        ]
    );
    assert_eq!(
        context.integration_steps[0].required_names,
        vec!["windows", "macos", "linux"]
    );
    assert!(context.integration_steps[1]
        .required_names
        .contains(&"main_execution_plan_bridge"));
    assert!(context.integration_steps[2]
        .required_names
        .contains(&"ai_capability_catalog"));
    assert!(context.integration_steps[3]
        .required_names
        .contains(&"publish_ai_catalog"));
    assert_eq!(
        context.integration_steps[4].required_names,
        vec!["llm_executor"]
    );
    assert_eq!(
        context.integration_steps[5].required_names,
        vec!["skill_registry"]
    );
    assert_eq!(
        context.integration_steps[6].required_names,
        vec!["product_adapter_tools"]
    );
    assert_eq!(
        context
            .readiness
            .adapter_parity
            .as_ref()
            .unwrap()
            .binding_counts,
        vec![27, 27, 27]
    );
    assert_eq!(
        context.ai_routes.len(),
        product_ai_capability_catalog().len()
    );
    let ocr = context
        .ai_routes
        .iter()
        .find(|route| route.capability_id == "clipboard.product.ocr")
        .unwrap();

    assert_eq!(ocr.provider_name, "product_adapter");
    assert_eq!(ocr.executor_boundary_name, "product_adapter_tools");
    assert_eq!(ocr.executor_task_name, "connect_product_ai_tools");
    assert_eq!(ocr.action_name, "ocr_image");
    assert_eq!(ocr.surface_name, "row_context_menu");
    assert_eq!(
        ocr.input_context_names,
        vec![
            "user_prompt",
            "selected_image",
            "selected_file_path",
            "clipboard_item_ids"
        ]
    );
    assert_eq!(ocr.result_name, "clipboard_text");

    let base_context = zsui_agent_context();
    assert_eq!(base_context.readiness.adapter_parity, None);
    assert!(base_context.porting_work_items.is_empty());
    assert_eq!(
        base_context.reusable_app_blueprint.reusable_feature_names,
        context.reusable_app_blueprint.reusable_feature_names
    );
    assert_eq!(
        base_context
            .integration_steps
            .iter()
            .map(|step| step.step_name)
            .collect::<Vec<_>>(),
        vec![
            "select_native_adapter",
            "verify_adapter_capability_parity",
            "implement_product_adapter_surfaces",
            "complete_product_adapter_tasks",
            "connect_llm_executor",
            "connect_skill_registry",
            "connect_product_ai_tools"
        ]
    );
    assert_eq!(
        base_context
            .ai_routes
            .iter()
            .map(|route| route.executor_boundary_name)
            .collect::<Vec<_>>(),
        vec![
            "llm_executor",
            "llm_executor",
            "llm_executor",
            "skill_registry",
            "skill_registry",
            "product_adapter_tools",
            "product_adapter_tools"
        ]
    );
}

#[test]
fn product_adapter_integration_contract_names_reusable_boundaries() {
    let contract = product_adapter_integration_contract();
    let checklist = product_adapter_reuse_checklist();

    assert_eq!(
        contract.required_surfaces,
        vec![
            ProductAdapterContractSurface::ProductIdentity,
            ProductAdapterContractSurface::ProductStateModel,
            ProductAdapterContractSurface::ProductCommandExecutor,
            ProductAdapterContractSurface::SettingsModel,
            ProductAdapterContractSurface::AsyncEventBridge,
            ProductAdapterContractSurface::AiCapabilityCatalog,
        ]
    );
    assert_eq!(
        contract.ai.total_capabilities,
        product_ai_capability_catalog().len()
    );
    assert_eq!(contract.ai.providers.len(), 3);
    assert_eq!(
        contract.ai.executor_boundaries,
        vec![
            ProductAiExecutorBoundary::LlmExecutor,
            ProductAiExecutorBoundary::SkillRegistry,
            ProductAiExecutorBoundary::ProductAdapterTools
        ]
    );
    assert_eq!(
        REQUIRED_PRODUCT_ADAPTER_INTEGRATION_TASKS,
        [
            ProductAdapterIntegrationTask::ProvideProductIdentity,
            ProductAdapterIntegrationTask::ProjectProductState,
            ProductAdapterIntegrationTask::ExecuteProductCommands,
            ProductAdapterIntegrationTask::BindSettingsModel,
            ProductAdapterIntegrationTask::BridgeAsyncEvents,
            ProductAdapterIntegrationTask::PublishAiCatalog,
            ProductAdapterIntegrationTask::ConnectLlmExecutor,
            ProductAdapterIntegrationTask::ConnectSkillRegistry,
            ProductAdapterIntegrationTask::ConnectProductAiTools,
        ]
    );
    assert_eq!(
        checklist.task_names,
        vec![
            "provide_product_identity",
            "project_product_state",
            "execute_product_commands",
            "bind_settings_model",
            "bridge_async_events",
            "publish_ai_catalog",
            "connect_llm_executor",
            "connect_skill_registry",
            "connect_product_ai_tools"
        ]
    );
    assert_eq!(
        checklist.surface_names,
        vec![
            "product_identity",
            "product_state_model",
            "product_command_executor",
            "settings_model",
            "async_event_bridge",
            "ai_capability_catalog"
        ]
    );
    assert_eq!(
        checklist.ai_provider_names,
        vec!["llms", "skills", "product_adapter"]
    );
    assert_eq!(
        checklist.ai_executor_boundary_names,
        vec!["llm_executor", "skill_registry", "product_adapter_tools"]
    );
    assert_eq!(
        checklist.ai_route_ids,
        product_ai_capability_catalog()
            .iter()
            .map(|capability| capability.id)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        required_product_adapter_host_method_names(),
        vec![
            "product_identity",
            "project_product_state",
            "execute_product_command",
            "bind_settings_model",
            "bridge_async_event",
            "publish_ai_catalog",
            "execute_ai_plan"
        ]
    );
}

#[test]
fn product_adapter_host_trait_executes_reusable_function_path() {
    #[derive(Default)]
    struct RecordingProductAdapter {
        commands: Vec<&'static str>,
        settings_revision: u64,
        events: Vec<&'static str>,
        ai_actions: Vec<&'static str>,
    }

    impl ProductAdapterHost for RecordingProductAdapter {
        fn product_identity(&self) -> ProductAdapterIdentity {
            ProductAdapterIdentity {
                product_id: "demo.tool".to_string(),
                display_name: "Demo Tool".to_string(),
            }
        }

        fn project_product_state(&self) -> ProductAdapterProjectedState {
            ProductAdapterProjectedState {
                state_name: "ready".to_string(),
                revision: 7,
                native_clip_items: native_host_default_clip_list_projection(),
            }
        }

        fn execute_product_command(&mut self, command: Command) -> ProductAdapterCommandResult {
            self.commands.push(command.id.0);
            ProductAdapterCommandResult {
                accepted: true,
                result_name: "command_executed".to_string(),
            }
        }

        fn bind_settings_model(&mut self, settings: ProductAdapterSettingsSnapshot) {
            self.settings_revision = settings.revision;
        }

        fn bridge_async_event(
            &mut self,
            event: ApplicationEvent,
        ) -> ProductAdapterAsyncBridgeResult {
            let event_name = match event {
                ApplicationEvent::CloudSyncReady => "cloud_sync_ready",
                _ => "other_event",
            };
            self.events.push(event_name);
            ProductAdapterAsyncBridgeResult {
                bridged: true,
                event_name: event_name.to_string(),
            }
        }

        fn publish_ai_catalog(&self) -> Vec<ProductAiCapability> {
            product_ai_capability_catalog()
                .iter()
                .map(|descriptor| descriptor.capability())
                .collect()
        }

        fn execute_ai_plan(&mut self, plan: ProductAiExecutionPlan) -> ProductAdapterCommandResult {
            self.ai_actions.push(plan.action_name());
            ProductAdapterCommandResult {
                accepted: true,
                result_name: plan.result_name().to_string(),
            }
        }
    }

    let mut adapter = RecordingProductAdapter::default();
    assert_eq!(adapter.product_identity().product_id, "demo.tool");
    assert_eq!(adapter.project_product_state().state_name, "ready");

    let command_result = adapter.execute_product_command(Command::window(CommandId("tool.run")));
    assert!(command_result.accepted);
    assert_eq!(adapter.commands, vec!["tool.run"]);

    adapter.bind_settings_model(ProductAdapterSettingsSnapshot {
        profile_name: "default".to_string(),
        revision: 3,
    });
    assert_eq!(adapter.settings_revision, 3);

    let bridged = adapter.bridge_async_event(ApplicationEvent::CloudSyncReady);
    assert!(bridged.bridged);
    assert_eq!(adapter.events, vec!["cloud_sync_ready"]);
    assert_eq!(
        adapter.publish_ai_catalog().len(),
        product_ai_capability_catalog().len()
    );

    let plan = product_ai_execution_plan(ProductAiInvocation {
        capability_id: "clipboard.clean".to_string(),
        input_text: "clean me".to_string(),
        context_item_ids: vec![1],
    })
    .unwrap();
    let ai_result = adapter.execute_ai_plan(plan);
    assert!(ai_result.accepted);
    assert_eq!(ai_result.result_name, "clipboard_text");
    assert_eq!(adapter.ai_actions, vec!["clean_text"]);
}

#[test]
fn native_runtime_driver_trait_executes_platform_entry_path() {
    #[derive(Default)]
    struct RecordingRuntimeDriver {
        started: bool,
        shutdown_requested: bool,
        commands: Vec<&'static str>,
        events: Vec<ApplicationEvent>,
    }

    impl NativeRuntimeDriver for RecordingRuntimeDriver {
        type WindowHandle = NativeWindowToken;

        fn start_runtime(
            &mut self,
            request: NativeRuntimeStartupRequest,
        ) -> NativeRuntimeStartupResult<Self::WindowHandle> {
            assert_eq!(request.app_name, "Demo Tool");
            assert_eq!(request.main_window.title, "Demo");
            assert_eq!(request.status_item_tooltip, Some("Demo Tool".to_string()));
            self.started = true;
            NativeRuntimeStartupResult::Started(NativeMainWindowHandles {
                main: NativeWindowToken(1),
                quick: NativeWindowToken(2),
            })
        }

        fn dispatch_ui_command(&mut self, command: Command) {
            self.commands.push(command.id.0);
            self.events.push(ApplicationEvent::ItemsPageReady);
        }

        fn poll_application_event(&mut self) -> Option<ApplicationEvent> {
            self.events.pop()
        }

        fn request_shutdown(&mut self) {
            self.shutdown_requested = true;
        }
    }

    assert_eq!(
        required_native_runtime_driver_operation_names(),
        vec![
            "start_runtime",
            "dispatch_ui_command",
            "poll_application_event",
            "request_shutdown"
        ]
    );

    let mut driver = RecordingRuntimeDriver::default();
    let startup = driver.start_runtime(NativeRuntimeStartupRequest {
        app_name: "Demo Tool".to_string(),
        main_window: NativeMainWindowRequest {
            title: "Demo".to_string(),
            size: Size {
                width: 320,
                height: 240,
            },
            main_visible: true,
        },
        status_item_tooltip: Some("Demo Tool".to_string()),
    });
    assert_eq!(
        startup,
        NativeRuntimeStartupResult::Started(NativeMainWindowHandles {
            main: NativeWindowToken(1),
            quick: NativeWindowToken(2)
        })
    );
    assert!(driver.started);

    driver.dispatch_ui_command(Command::window(CommandId("demo.open")));
    assert_eq!(driver.commands, vec!["demo.open"]);
    assert_eq!(
        driver.poll_application_event(),
        Some(ApplicationEvent::ItemsPageReady)
    );

    driver.request_shutdown();
    assert!(driver.shutdown_requested);
}

#[test]
fn reusable_runtime_harness_connects_native_driver_and_product_adapter() {
    #[derive(Default)]
    struct HarnessDriver {
        started: bool,
        shutdown_requested: bool,
        commands: Vec<&'static str>,
        events: Vec<ApplicationEvent>,
    }

    impl NativeRuntimeDriver for HarnessDriver {
        type WindowHandle = NativeWindowToken;

        fn start_runtime(
            &mut self,
            request: NativeRuntimeStartupRequest,
        ) -> NativeRuntimeStartupResult<Self::WindowHandle> {
            assert_eq!(request.app_name, "Harness Tool");
            self.started = true;
            NativeRuntimeStartupResult::Started(NativeMainWindowHandles {
                main: NativeWindowToken(11),
                quick: NativeWindowToken(12),
            })
        }

        fn dispatch_ui_command(&mut self, command: Command) {
            self.commands.push(command.id.0);
            self.events.push(ApplicationEvent::ItemsPageReady);
        }

        fn poll_application_event(&mut self) -> Option<ApplicationEvent> {
            self.events.pop()
        }

        fn request_shutdown(&mut self) {
            self.shutdown_requested = true;
        }
    }

    #[derive(Default)]
    struct HarnessProductAdapter {
        commands: Vec<&'static str>,
        bridged_events: Vec<&'static str>,
        ai_results: Vec<&'static str>,
    }

    impl ProductAdapterHost for HarnessProductAdapter {
        fn product_identity(&self) -> ProductAdapterIdentity {
            ProductAdapterIdentity {
                product_id: "harness.tool".to_string(),
                display_name: "Harness Tool".to_string(),
            }
        }

        fn project_product_state(&self) -> ProductAdapterProjectedState {
            ProductAdapterProjectedState {
                state_name: "ready".to_string(),
                revision: 1,
                native_clip_items: native_host_default_clip_list_projection(),
            }
        }

        fn execute_product_command(&mut self, command: Command) -> ProductAdapterCommandResult {
            self.commands.push(command.id.0);
            ProductAdapterCommandResult {
                accepted: true,
                result_name: "command_executed".to_string(),
            }
        }

        fn bind_settings_model(&mut self, _settings: ProductAdapterSettingsSnapshot) {}

        fn bridge_async_event(
            &mut self,
            event: ApplicationEvent,
        ) -> ProductAdapterAsyncBridgeResult {
            let event_name = match event {
                ApplicationEvent::ItemsPageReady => "items_page_ready",
                _ => "other_event",
            };
            self.bridged_events.push(event_name);
            ProductAdapterAsyncBridgeResult {
                bridged: true,
                event_name: event_name.to_string(),
            }
        }

        fn publish_ai_catalog(&self) -> Vec<ProductAiCapability> {
            product_ai_capability_catalog()
                .iter()
                .map(|descriptor| descriptor.capability())
                .collect()
        }

        fn execute_ai_plan(&mut self, plan: ProductAiExecutionPlan) -> ProductAdapterCommandResult {
            self.ai_results.push(plan.result_name());
            ProductAdapterCommandResult {
                accepted: true,
                result_name: plan.result_name().to_string(),
            }
        }
    }

    assert_eq!(
        zsui_reusable_runtime_harness_stage_names(),
        vec![
            "start_runtime",
            "dispatch_command",
            "bridge_application_event",
            "execute_ai_invocation",
            "request_shutdown"
        ]
    );

    let mut harness =
        ZsuiReusableRuntimeHarness::new(HarnessDriver::default(), HarnessProductAdapter::default());
    let startup = harness.start(NativeRuntimeStartupRequest {
        app_name: "Harness Tool".to_string(),
        main_window: NativeMainWindowRequest {
            title: "Harness".to_string(),
            size: Size {
                width: 360,
                height: 240,
            },
            main_visible: true,
        },
        status_item_tooltip: Some("Harness Tool".to_string()),
    });
    assert_eq!(
        startup,
        NativeRuntimeStartupResult::Started(NativeMainWindowHandles {
            main: NativeWindowToken(11),
            quick: NativeWindowToken(12)
        })
    );

    let command_result = harness.dispatch_command(Command::window(CommandId("harness.run")));
    assert!(command_result.accepted);
    assert_eq!(harness.driver().commands, vec!["harness.run"]);
    assert_eq!(harness.product().commands, vec!["harness.run"]);

    let bridge = harness.poll_and_bridge_event().unwrap();
    assert!(bridge.bridged);
    assert_eq!(bridge.event_name, "items_page_ready");
    assert_eq!(harness.product().bridged_events, vec!["items_page_ready"]);

    let ai_result = harness
        .execute_ai_invocation(ProductAiInvocation {
            capability_id: "clipboard.clean".to_string(),
            input_text: "clean".to_string(),
            context_item_ids: vec![5],
        })
        .unwrap();
    assert_eq!(ai_result.result_name, "clipboard_text");
    assert_eq!(harness.product().ai_results, vec!["clipboard_text"]);

    harness.request_shutdown();
    assert!(harness.driver().shutdown_requested);
}

#[test]
fn zsui_layers_separate_foundation_from_product_adapter() {
    assert!(ZsuiLayer::CoreContracts.is_reusable_foundation());
    assert!(ZsuiLayer::LayoutModel.is_reusable_foundation());
    assert!(ZsuiLayer::RenderProtocol.is_reusable_foundation());
    assert!(ZsuiLayer::NativeHost.is_reusable_foundation());
    assert!(!ZsuiLayer::ProductAdapter.is_reusable_foundation());
}

#[test]
fn semantic_style_keeps_native_visuals_in_platform_resolver() {
    let style = SemanticTextStyle::body();

    assert_eq!(style.role, TextRole::Body);
    assert_eq!(style.color, ColorRole::PrimaryText);
    assert_eq!(style.weight, TextWeight::Regular);
    assert_eq!(style.horizontal_align, HorizontalAlign::Start);
    assert_eq!(style.vertical_align, VerticalAlign::Center);
    assert_eq!(style.wrap, TextWrap::NoWrap);
    assert!(style.ellipsis);
}

#[test]
fn native_style_host_operations_are_explicit_porting_contract() {
    assert_eq!(
        REQUIRED_NATIVE_STYLE_HOST_OPERATIONS,
        [NativeStyleHostOperation::ResolveTextStyle]
    );
    assert_eq!(
        REQUIRED_NATIVE_STYLE_HOST_OPERATIONS[0].operation_name(),
        "resolve_text_style"
    );
}

#[test]
fn settings_component_kinds_describe_intent_without_platform_classes() {
    assert_eq!(
        SettingsComponentKind::Label.family(),
        NativeControlFamily::StaticText
    );
    assert_eq!(
        SettingsComponentKind::TextInput.family(),
        NativeControlFamily::TextInput
    );
    assert_eq!(
        SettingsComponentKind::Toggle.family(),
        NativeControlFamily::Action
    );
    assert_eq!(
        SettingsComponentKind::Dropdown.family(),
        NativeControlFamily::Action
    );
    assert!(SettingsComponentKind::Button.is_action());
    assert!(SettingsComponentKind::AccentButton.is_action());
}

#[test]
fn native_control_mapper_operations_are_explicit_porting_contract() {
    assert_eq!(
        REQUIRED_NATIVE_CONTROL_MAPPER_OPERATIONS,
        [NativeControlMapperOperation::ClassName]
    );
    assert_eq!(
        REQUIRED_NATIVE_CONTROL_MAPPER_OPERATIONS[0].operation_name(),
        "class_name"
    );
}

#[test]
fn text_layout_host_operations_are_explicit_porting_contract() {
    assert_eq!(
        REQUIRED_TEXT_LAYOUT_HOST_OPERATIONS,
        [
            TextLayoutHostOperation::Measure,
            TextLayoutHostOperation::LayoutRuns,
        ]
    );
    let names: Vec<_> = REQUIRED_TEXT_LAYOUT_HOST_OPERATIONS
        .iter()
        .map(|operation| operation.operation_name())
        .collect();
    assert_eq!(names, ["measure", "layout_runs"]);
}

#[test]
fn renderer_host_operations_are_explicit_porting_contract() {
    assert_eq!(
        REQUIRED_RENDERER_HOST_OPERATIONS,
        [
            RendererHostOperation::FillRect,
            RendererHostOperation::StrokeRect,
            RendererHostOperation::DrawText,
            RendererHostOperation::PushClip,
            RendererHostOperation::PopClip,
        ]
    );
    let names: Vec<_> = REQUIRED_RENDERER_HOST_OPERATIONS
        .iter()
        .map(|operation| operation.operation_name())
        .collect();
    assert_eq!(
        names,
        [
            "fill_rect",
            "stroke_rect",
            "draw_text",
            "push_clip",
            "pop_clip"
        ]
    );
}

#[test]
fn settings_control_spec_carries_native_control_intent() {
    let bounds = UiRect::new(10, 20, 110, 52);
    let button = SettingsControlSpec::action(SettingsComponentKind::Button, 42, "保存", bounds);

    assert_eq!(button.kind, SettingsComponentKind::Button);
    assert_eq!(button.id, Some(42));
    assert_eq!(button.text, "保存");
    assert_eq!(button.width(), 100);
    assert_eq!(button.height(), 32);

    let label = SettingsControlSpec::label("标题", bounds);
    assert_eq!(label.kind, SettingsComponentKind::Label);
    assert_eq!(label.id, None);

    let input = SettingsControlSpec::text_input(7, "abc", bounds);
    assert_eq!(input.kind, SettingsComponentKind::TextInput);
    assert_eq!(input.id, Some(7));
}

#[test]
fn native_settings_control_host_consumes_control_specs() {
    #[derive(Default)]
    struct FakeHost {
        created: Vec<SettingsControlSpec>,
        destroyed: Vec<usize>,
        visible: Vec<(usize, bool)>,
        enabled: Vec<(usize, bool)>,
        bounds: Vec<(usize, UiRect)>,
        text: Vec<(usize, String)>,
        repainted: Vec<usize>,
    }

    impl NativeSettingsControlHost for FakeHost {
        type Handle = usize;

        fn create_control(&mut self, spec: &SettingsControlSpec) -> Self::Handle {
            self.created.push(spec.clone());
            self.created.len()
        }

        fn destroy_control(&mut self, handle: Self::Handle) {
            self.destroyed.push(handle);
        }

        fn control_exists(&self, handle: Self::Handle) -> bool {
            handle > 0 && handle <= self.created.len() && !self.destroyed.contains(&handle)
        }

        fn set_control_visible(&mut self, handle: Self::Handle, visible: bool) {
            self.visible.push((handle, visible));
        }

        fn set_control_enabled(&mut self, handle: Self::Handle, enabled: bool) {
            self.enabled.push((handle, enabled));
        }

        fn set_control_bounds(&mut self, handle: Self::Handle, bounds: UiRect) {
            self.bounds.push((handle, bounds));
        }

        fn control_at_point(&self, point: Point) -> Option<Self::Handle> {
            self.bounds
                .iter()
                .rev()
                .find(|(handle, bounds)| {
                    self.control_exists(*handle) && bounds.contains(point.x, point.y)
                })
                .map(|(handle, _)| *handle)
        }

        fn control_screen_bounds(&self, handle: Self::Handle) -> Option<UiRect> {
            self.bounds
                .iter()
                .rev()
                .find(|(bounds_handle, _)| *bounds_handle == handle)
                .map(|(_, bounds)| *bounds)
        }

        fn control_text(&self, handle: Self::Handle) -> String {
            self.text
                .iter()
                .rev()
                .find(|(text_handle, _)| *text_handle == handle)
                .map(|(_, text)| text.clone())
                .unwrap_or_default()
        }

        fn set_control_text(&mut self, handle: Self::Handle, text: &str) {
            self.text.push((handle, text.to_string()));
        }

        fn request_control_repaint(&mut self, handle: Self::Handle) -> bool {
            self.repainted.push(handle);
            true
        }
    }

    let mut host = FakeHost::default();
    let spec = SettingsControlSpec::action(
        SettingsComponentKind::Dropdown,
        9,
        "同步方案",
        UiRect::new(1, 2, 201, 34),
    );

    assert_eq!(host.create_control(&spec), 1);
    assert!(host.control_exists(1));
    host.set_control_visible(1, false);
    host.set_control_enabled(1, false);
    host.set_control_bounds(1, UiRect::new(3, 4, 203, 36));
    assert_eq!(host.control_at_point(Point { x: 10, y: 20 }), Some(1));
    assert_eq!(
        host.control_screen_bounds(1),
        Some(UiRect::new(3, 4, 203, 36))
    );
    host.set_control_text(1, "WebDAV");
    assert_eq!(host.control_text(1), "WebDAV");
    assert!(host.request_control_repaint(1));
    host.destroy_control(1);
    assert!(!host.control_exists(1));

    assert_eq!(host.created, vec![spec]);
    assert_eq!(host.visible, vec![(1, false)]);
    assert_eq!(host.enabled, vec![(1, false)]);
    assert_eq!(host.bounds, vec![(1, UiRect::new(3, 4, 203, 36))]);
    assert_eq!(host.text, vec![(1, "WebDAV".to_string())]);
    assert_eq!(host.repainted, vec![1]);
    assert_eq!(host.destroyed, vec![1]);
}

#[test]
fn settings_control_host_operations_are_explicit_porting_contract() {
    assert_eq!(
        REQUIRED_SETTINGS_CONTROL_HOST_OPERATIONS,
        [
            SettingsControlHostOperation::CreateControl,
            SettingsControlHostOperation::DestroyControl,
            SettingsControlHostOperation::ControlExists,
            SettingsControlHostOperation::SetControlVisible,
            SettingsControlHostOperation::SetControlEnabled,
            SettingsControlHostOperation::SetControlBounds,
            SettingsControlHostOperation::ControlAtPoint,
            SettingsControlHostOperation::ControlScreenBounds,
            SettingsControlHostOperation::ControlText,
            SettingsControlHostOperation::SetControlText,
            SettingsControlHostOperation::RequestControlRepaint,
        ]
    );

    let names: Vec<_> = REQUIRED_SETTINGS_CONTROL_HOST_OPERATIONS
        .iter()
        .map(|operation| operation.operation_name())
        .collect();
    assert_eq!(
        names,
        [
            "create_control",
            "destroy_control",
            "control_exists",
            "set_control_visible",
            "set_control_enabled",
            "set_control_bounds",
            "control_at_point",
            "control_screen_bounds",
            "control_text",
            "set_control_text",
            "request_control_repaint",
        ]
    );
}

#[test]
fn clipboard_host_contract_is_the_trait_not_a_parallel_operation_registry() {
    let source =
        std::fs::read_to_string("src/app_core/host_protocol.rs").expect("host protocol source");

    assert!(source.contains("pub(crate) trait ClipboardHost"));
    for method_name in [
        "fn read_text()",
        "fn write_text(text: &str)",
        "fn read_image_rgba()",
        "fn write_image_rgba(bytes: &[u8], width: usize, height: usize)",
        "fn read_file_paths()",
        "fn write_file_paths(paths: &[String])",
        "fn sequence_number()",
        "fn write_text_ignored_by_monitors(text: &str)",
        "fn should_ignore_capture_by_named_format()",
    ] {
        assert!(
            source.contains(method_name),
            "ClipboardHost trait must declare {method_name}"
        );
    }
    assert!(!source.contains("enum ClipboardHostOperation"));
    assert!(!source.contains("REQUIRED_CLIPBOARD_HOST_OPERATIONS"));
}

#[test]
fn clipboard_monitor_poll_result_bridges_only_external_changes() {
    let mut state = ClipboardMonitorState::default();

    assert_eq!(
        clipboard_monitor_poll_result_for_sequence(&mut state, true, 10, false),
        ClipboardMonitorPollResult::Baseline { sequence: 10 }
    );
    assert_eq!(state.last_sequence(), Some(10));
    assert_eq!(
        clipboard_monitor_poll_result_for_sequence(&mut state, true, 10, false),
        ClipboardMonitorPollResult::Unchanged { sequence: 10 }
    );

    let changed = clipboard_monitor_poll_result_for_sequence(&mut state, true, 11, false);
    assert_eq!(
        changed,
        ClipboardMonitorPollResult::Changed { sequence: 11 }
    );
    assert!(changed.should_bridge_application_event());
    assert_eq!(changed.sequence(), 11);

    let ignored = clipboard_monitor_poll_result_for_sequence(&mut state, true, 12, true);
    assert_eq!(
        ignored,
        ClipboardMonitorPollResult::IgnoredSelfWrite { sequence: 12 }
    );
    assert!(!ignored.should_bridge_application_event());

    assert_eq!(
        clipboard_monitor_poll_result_for_sequence(&mut state, false, 13, false),
        ClipboardMonitorPollResult::Disabled { sequence: 13 }
    );
    assert_eq!(state.last_sequence(), Some(13));
    assert_eq!(
        clipboard_monitor_poll_result_for_sequence(&mut state, true, 13, false),
        ClipboardMonitorPollResult::Unchanged { sequence: 13 }
    );
}

#[test]
fn main_timer_ids_map_to_platform_neutral_tasks() {
    let ids = MainTimerIds {
        startup_recovery: 11,
        vv_watch: 12,
        vv_show: 13,
        paste: 14,
        search_debounce: 15,
        hidden_reclaim: 16,
        clipboard_retry: 17,
        dpi_fit: 18,
        scroll_fade: 19,
        edge_auto_hide: 20,
        outside_hide: 21,
        cloud_sync: 22,
    };

    assert_eq!(
        main_timer_task_for_id(11, ids),
        Some(MainTimerTask::StartupRecovery)
    );
    assert_eq!(
        main_timer_task_for_id(12, ids),
        Some(MainTimerTask::VvWatch)
    );
    assert_eq!(main_timer_task_for_id(13, ids), Some(MainTimerTask::VvShow));
    assert_eq!(main_timer_task_for_id(14, ids), Some(MainTimerTask::Paste));
    assert_eq!(
        main_timer_task_for_id(15, ids),
        Some(MainTimerTask::SearchDebounce)
    );
    assert_eq!(
        main_timer_task_for_id(16, ids),
        Some(MainTimerTask::HiddenReclaim)
    );
    assert_eq!(
        main_timer_task_for_id(17, ids),
        Some(MainTimerTask::ClipboardRetry)
    );
    assert_eq!(main_timer_task_for_id(18, ids), Some(MainTimerTask::DpiFit));
    assert_eq!(
        main_timer_task_for_id(19, ids),
        Some(MainTimerTask::ScrollFade)
    );
    assert_eq!(
        main_timer_task_for_id(20, ids),
        Some(MainTimerTask::EdgeAutoHide)
    );
    assert_eq!(
        main_timer_task_for_id(21, ids),
        Some(MainTimerTask::OutsideHide)
    );
    assert_eq!(
        main_timer_task_for_id(22, ids),
        Some(MainTimerTask::CloudSync)
    );
    assert_eq!(main_timer_task_for_id(999, ids), None);
}

#[test]
fn main_async_events_are_plain_platform_neutral_payloads() {
    let image = ImageThumbnail {
        bytes: vec![255, 0, 0, 255],
        width: 1,
        height: 1,
    };
    let paste = ImagePasteReadyResult {
        image: Some((vec![1, 2, 3, 4], 1, 1)),
        target: NativeWindowToken(42),
        hide_main: true,
        backspaces: 2,
    };
    let text = TextOperationReadyResult {
        text: Some("hello".to_string()),
        error: None,
    };

    assert_eq!(
        MainAsyncEvent::ImagePaste(paste.clone()),
        MainAsyncEvent::ImagePaste(paste)
    );
    assert_eq!(
        MainAsyncEvent::ImageOcr(text.clone()),
        MainAsyncEvent::ImageOcr(text)
    );
    assert_eq!(
        MainAsyncEvent::ImageThumbnail(ImageThumbReadyResult {
            item_id: 7,
            image: Some(image.clone()),
        }),
        MainAsyncEvent::ImageThumbnail(ImageThumbReadyResult {
            item_id: 7,
            image: Some(image),
        })
    );
}

#[test]
fn ui_event_protocol_allows_product_specific_application_events() {
    let zsclip_event: UiEvent = UiEvent::Application(ApplicationEvent::CloudSyncReady);
    assert_eq!(
        zsclip_event,
        UiEvent::Application(ApplicationEvent::CloudSyncReady)
    );

    let external_event: event_protocol::UiEvent<&'static str> =
        event_protocol::UiEvent::Application("external.app.ready");
    assert_eq!(
        external_event,
        event_protocol::UiEvent::Application("external.app.ready")
    );
}

#[test]
fn required_ui_host_surfaces_are_explicit_porting_contract() {
    assert_eq!(
        REQUIRED_UI_HOST_SURFACES,
        [
            UiHostSurface::MainWindow,
            UiHostSurface::SettingsWindow,
            UiHostSurface::SettingsDropdown,
            UiHostSurface::InputDialog,
            UiHostSurface::EditDialog,
        ]
    );
    assert_eq!(
        REQUIRED_UI_HOST_SURFACES
            .iter()
            .map(|surface| surface.adapter_name())
            .collect::<Vec<_>>(),
        vec![
            "main_window_host_event_from_message",
            "settings_window_host_event_from_message",
            "dropdown_window_host_event_from_message",
            "input_dialog_host_event_from_message",
            "edit_dialog_host_event_from_message",
        ]
    );
}

#[test]
fn product_ai_capabilities_stay_in_product_adapter_layer() {
    let capabilities = product_ai_capability_catalog();
    let manifest = product_ai_integration_manifest();
    let invocation = ProductAiInvocation {
        capability_id: "clipboard.clean".to_string(),
        input_text: " raw text ".to_string(),
        context_item_ids: vec![7, 9],
    };
    let clean = product_ai_capability_descriptor("clipboard.clean").unwrap();
    let translate = product_ai_capability_descriptor("clipboard.skill.translate").unwrap();
    let ocr = product_ai_capability_descriptor("clipboard.product.ocr").unwrap();
    let configure = product_ai_capability_descriptor("clipboard.product.configure_ai").unwrap();

    assert_eq!(capabilities.len(), 7);
    assert_eq!(manifest.total_capabilities, capabilities.len());
    assert_eq!(manifest.llm_capabilities, 3);
    assert_eq!(manifest.skill_capabilities, 2);
    assert_eq!(manifest.product_adapter_capabilities, 2);
    assert_eq!(
        manifest.providers,
        vec![
            ProductAiProviderKind::Llms,
            ProductAiProviderKind::Skills,
            ProductAiProviderKind::ProductAdapter
        ]
    );
    assert_eq!(
        manifest
            .providers
            .iter()
            .map(|provider| provider.provider_name())
            .collect::<Vec<_>>(),
        vec!["llms", "skills", "product_adapter"]
    );
    assert_eq!(
        manifest.executor_boundaries,
        vec![
            ProductAiExecutorBoundary::LlmExecutor,
            ProductAiExecutorBoundary::SkillRegistry,
            ProductAiExecutorBoundary::ProductAdapterTools
        ]
    );
    assert_eq!(
        manifest
            .executor_boundaries
            .iter()
            .map(|boundary| boundary.boundary_name())
            .collect::<Vec<_>>(),
        vec!["llm_executor", "skill_registry", "product_adapter_tools"]
    );
    assert_eq!(
        manifest.actions,
        vec![
            ProductAiActionKind::CleanText,
            ProductAiActionKind::SummarizeItems,
            ProductAiActionKind::ExplainItem,
            ProductAiActionKind::TranslateText,
            ProductAiActionKind::InvokeSkill,
            ProductAiActionKind::OcrImage,
            ProductAiActionKind::ConfigureProvider
        ]
    );
    assert_eq!(
        manifest
            .actions
            .iter()
            .map(|action| action.action_name())
            .collect::<Vec<_>>(),
        vec![
            "clean_text",
            "summarize_items",
            "explain_item",
            "translate_text",
            "invoke_skill",
            "ocr_image",
            "configure_provider"
        ]
    );
    assert_eq!(
        manifest.surfaces,
        vec![
            ProductAiUiSurface::RowContextMenu,
            ProductAiUiSurface::MainWindow,
            ProductAiUiSurface::SettingsPluginPage
        ]
    );
    assert_eq!(
        manifest
            .surfaces
            .iter()
            .map(|surface| surface.surface_name())
            .collect::<Vec<_>>(),
        vec!["row_context_menu", "main_window", "settings_plugin_page"]
    );
    assert_eq!(
        manifest.contexts,
        vec![
            ProductAiContextKind::UserPrompt,
            ProductAiContextKind::SelectedText,
            ProductAiContextKind::ClipboardItemIds,
            ProductAiContextKind::SelectedFilePath,
            ProductAiContextKind::SelectedImage,
            ProductAiContextKind::SettingsProfile
        ]
    );
    assert_eq!(
        manifest
            .contexts
            .iter()
            .map(|context| context.context_name())
            .collect::<Vec<_>>(),
        vec![
            "user_prompt",
            "selected_text",
            "clipboard_item_ids",
            "selected_file_path",
            "selected_image",
            "settings_profile"
        ]
    );
    assert_eq!(
        manifest.results,
        vec![
            ProductAiResultKind::ClipboardText,
            ProductAiResultKind::Text,
            ProductAiResultKind::ProductCommand,
            ProductAiResultKind::SettingsMutation
        ]
    );
    assert_eq!(
        manifest
            .results
            .iter()
            .map(|result| result.result_name())
            .collect::<Vec<_>>(),
        vec![
            "clipboard_text",
            "text",
            "product_command",
            "settings_mutation"
        ]
    );
    assert_eq!(manifest.execution_routes.len(), capabilities.len());
    let ocr_route = manifest
        .execution_routes
        .iter()
        .find(|route| route.capability_id == "clipboard.product.ocr")
        .unwrap();
    assert_eq!(ocr_route.provider_name(), "product_adapter");
    assert_eq!(
        ocr_route.executor_boundary,
        ProductAiExecutorBoundary::ProductAdapterTools
    );
    assert_eq!(ocr_route.executor_boundary_name(), "product_adapter_tools");
    assert_eq!(ocr_route.executor_task_name(), "connect_product_ai_tools");
    assert_eq!(ocr_route.action_name(), "ocr_image");
    assert_eq!(ocr_route.surface_name(), "row_context_menu");
    assert_eq!(ocr_route.result_name(), "clipboard_text");
    assert_eq!(ocr_route.surface, ProductAiUiSurface::RowContextMenu);
    assert_eq!(
        ocr_route.input_contexts,
        vec![
            ProductAiContextKind::UserPrompt,
            ProductAiContextKind::SelectedImage,
            ProductAiContextKind::SelectedFilePath,
            ProductAiContextKind::ClipboardItemIds
        ]
    );
    assert_eq!(
        ocr_route.input_context_names(),
        vec![
            "user_prompt",
            "selected_image",
            "selected_file_path",
            "clipboard_item_ids"
        ]
    );
    assert_eq!(
        manifest
            .execution_routes
            .iter()
            .map(|route| (
                route.capability_id,
                route.provider,
                route.executor_boundary,
                route.action,
                route.result
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                "clipboard.clean",
                ProductAiProviderKind::Llms,
                ProductAiExecutorBoundary::LlmExecutor,
                ProductAiActionKind::CleanText,
                ProductAiResultKind::ClipboardText
            ),
            (
                "clipboard.summarize",
                ProductAiProviderKind::Llms,
                ProductAiExecutorBoundary::LlmExecutor,
                ProductAiActionKind::SummarizeItems,
                ProductAiResultKind::Text
            ),
            (
                "clipboard.explain",
                ProductAiProviderKind::Llms,
                ProductAiExecutorBoundary::LlmExecutor,
                ProductAiActionKind::ExplainItem,
                ProductAiResultKind::Text
            ),
            (
                "clipboard.skill.translate",
                ProductAiProviderKind::Skills,
                ProductAiExecutorBoundary::SkillRegistry,
                ProductAiActionKind::TranslateText,
                ProductAiResultKind::ClipboardText
            ),
            (
                "clipboard.skill.run",
                ProductAiProviderKind::Skills,
                ProductAiExecutorBoundary::SkillRegistry,
                ProductAiActionKind::InvokeSkill,
                ProductAiResultKind::ProductCommand
            ),
            (
                "clipboard.product.ocr",
                ProductAiProviderKind::ProductAdapter,
                ProductAiExecutorBoundary::ProductAdapterTools,
                ProductAiActionKind::OcrImage,
                ProductAiResultKind::ClipboardText
            ),
            (
                "clipboard.product.configure_ai",
                ProductAiProviderKind::ProductAdapter,
                ProductAiExecutorBoundary::ProductAdapterTools,
                ProductAiActionKind::ConfigureProvider,
                ProductAiResultKind::SettingsMutation
            )
        ]
    );
    assert_eq!(clean.capability().provider, ProductAiProviderKind::Llms);
    assert_eq!(translate.provider, ProductAiProviderKind::Skills);
    assert_eq!(ocr.provider, ProductAiProviderKind::ProductAdapter);
    assert_eq!(clean.action, ProductAiActionKind::CleanText);
    assert_eq!(translate.action, ProductAiActionKind::TranslateText);
    assert_eq!(ocr.action, ProductAiActionKind::OcrImage);
    assert_eq!(clean.surface, ProductAiUiSurface::RowContextMenu);
    assert_eq!(configure.surface, ProductAiUiSurface::SettingsPluginPage);
    assert!(clean.accepts_context(ProductAiContextKind::SelectedText));
    assert!(!clean.accepts_context(ProductAiContextKind::SelectedImage));
    assert!(ocr.accepts_context(ProductAiContextKind::SelectedImage));
    assert_eq!(clean.result, ProductAiResultKind::ClipboardText);
    assert_eq!(configure.result, ProductAiResultKind::SettingsMutation);
    assert_eq!(
        product_ai_capabilities_for_surface(ProductAiUiSurface::SettingsPluginPage)
            .into_iter()
            .map(|capability| capability.id)
            .collect::<Vec<_>>(),
        vec!["clipboard.product.configure_ai"]
    );
    assert_eq!(
        product_ai_capabilities_for_context(
            ProductAiUiSurface::RowContextMenu,
            ProductAiContextKind::SelectedImage
        )
        .into_iter()
        .map(|capability| capability.id)
        .collect::<Vec<_>>(),
        vec!["clipboard.product.ocr"]
    );
    assert_eq!(
        product_ai_capability_for_action(
            ProductAiUiSurface::RowContextMenu,
            ProductAiActionKind::TranslateText
        )
        .unwrap()
        .id,
        "clipboard.skill.translate"
    );
    assert_eq!(invocation.capability_id, "clipboard.clean");
    assert_eq!(invocation.context_item_ids, vec![7, 9]);
}

#[test]
fn product_ai_execution_plan_routes_invocation_to_provider_family() {
    let llm = product_ai_execution_plan(ProductAiInvocation {
        capability_id: "clipboard.clean".to_string(),
        input_text: "trim this".to_string(),
        context_item_ids: vec![1],
    })
    .unwrap();
    let skill = product_ai_execution_plan(ProductAiInvocation {
        capability_id: "clipboard.skill.translate".to_string(),
        input_text: "hello".to_string(),
        context_item_ids: vec![2],
    })
    .unwrap();
    let product = product_ai_execution_plan(ProductAiInvocation {
        capability_id: "clipboard.product.ocr".to_string(),
        input_text: "".to_string(),
        context_item_ids: vec![3],
    })
    .unwrap();

    assert_eq!(llm.provider, ProductAiProviderKind::Llms);
    assert_eq!(llm.provider_name(), "llms");
    assert_eq!(
        llm.executor_boundary,
        ProductAiExecutorBoundary::LlmExecutor
    );
    assert_eq!(llm.executor_boundary_name(), "llm_executor");
    assert_eq!(llm.executor_task_name(), "connect_llm_executor");
    assert_eq!(llm.action, ProductAiActionKind::CleanText);
    assert_eq!(llm.action_name(), "clean_text");
    assert_eq!(llm.surface, ProductAiUiSurface::RowContextMenu);
    assert_eq!(llm.surface_name(), "row_context_menu");
    assert_eq!(
        llm.input_contexts,
        vec![
            ProductAiContextKind::UserPrompt,
            ProductAiContextKind::SelectedText,
            ProductAiContextKind::ClipboardItemIds
        ]
    );
    assert_eq!(
        llm.input_context_names(),
        vec!["user_prompt", "selected_text", "clipboard_item_ids"]
    );
    assert_eq!(llm.result, ProductAiResultKind::ClipboardText);
    assert_eq!(llm.result_name(), "clipboard_text");
    assert_eq!(llm.invocation.context_item_ids, vec![1]);
    assert_eq!(skill.provider, ProductAiProviderKind::Skills);
    assert_eq!(
        skill.executor_boundary,
        ProductAiExecutorBoundary::SkillRegistry
    );
    assert_eq!(skill.executor_boundary_name(), "skill_registry");
    assert_eq!(skill.executor_task_name(), "connect_skill_registry");
    assert_eq!(skill.action, ProductAiActionKind::TranslateText);
    assert_eq!(product.provider, ProductAiProviderKind::ProductAdapter);
    assert_eq!(
        product.executor_boundary,
        ProductAiExecutorBoundary::ProductAdapterTools
    );
    assert_eq!(product.executor_boundary_name(), "product_adapter_tools");
    assert_eq!(product.executor_task_name(), "connect_product_ai_tools");
    assert_eq!(product.action, ProductAiActionKind::OcrImage);
    assert_eq!(product.surface, ProductAiUiSurface::RowContextMenu);
    assert!(product
        .input_contexts
        .contains(&ProductAiContextKind::SelectedImage));
    assert_eq!(
        product_ai_execution_plan(ProductAiInvocation {
            capability_id: "clipboard.unknown".to_string(),
            input_text: "x".to_string(),
            context_item_ids: vec![],
        }),
        None
    );
}

#[test]
fn main_row_ai_menu_actions_resolve_to_product_ai_catalog() {
    let image_ocr = MainRowMenuAction::ImageOcr.ai_action_kind().unwrap();
    let text_translate = MainRowMenuAction::TextTranslate.ai_action_kind().unwrap();
    let image_capability =
        product_ai_capability_for_action(ProductAiUiSurface::RowContextMenu, image_ocr).unwrap();
    let translate_capability =
        product_ai_capability_for_action(ProductAiUiSurface::RowContextMenu, text_translate)
            .unwrap();

    assert_eq!(image_capability.id, "clipboard.product.ocr");
    assert!(image_capability.accepts_context(ProductAiContextKind::SelectedImage));
    assert_eq!(translate_capability.id, "clipboard.skill.translate");
    assert!(translate_capability.accepts_context(ProductAiContextKind::SelectedText));
    assert_eq!(MainRowMenuAction::Copy.ai_action_kind(), None);
}

#[test]
fn main_row_ai_capability_plan_describes_selected_context_for_ai() {
    fn item(id: i64, kind: ClipKind) -> ClipItem {
        ClipItem {
            id,
            kind,
            preview: format!("preview {id}"),
            text: matches!(kind, ClipKind::Text | ClipKind::Phrase).then(|| format!("text {id}")),
            source_app: "test".to_string(),
            file_paths: (kind == ClipKind::Files).then(|| vec![format!("C:/tmp/{id}.png")]),
            image_bytes: None,
            image_path: (kind == ClipKind::Image).then(|| format!("C:/tmp/{id}.png")),
            image_width: if kind == ClipKind::Image { 20 } else { 0 },
            image_height: if kind == ClipKind::Image { 20 } else { 0 },
            pinned: false,
            group_id: 0,
            created_at: "2026-01-01".to_string(),
        }
    }

    let text = item(7, ClipKind::Text);
    let text_plan = main_row_ai_capability_plan(Some(&text), &[]).unwrap();
    assert_eq!(text_plan.contexts, vec![ProductAiContextKind::SelectedText]);
    assert_eq!(text_plan.target_item_ids, vec![7]);
    assert_eq!(
        text_plan
            .capabilities
            .iter()
            .map(|capability| capability.capability_id)
            .collect::<Vec<_>>(),
        vec![
            "clipboard.clean",
            "clipboard.explain",
            "clipboard.skill.translate"
        ]
    );
    assert_eq!(
        text_plan.capabilities[0],
        MainRowAiCapabilityPresentation {
            capability_id: "clipboard.clean",
            label: "Clean text",
            provider: ProductAiProviderKind::Llms,
            action: ProductAiActionKind::CleanText,
            result: ProductAiResultKind::ClipboardText,
        }
    );

    let image = item(9, ClipKind::Image);
    let image_plan = main_row_ai_capability_plan(Some(&image), &[]).unwrap();
    assert_eq!(
        image_plan.contexts,
        vec![ProductAiContextKind::SelectedImage]
    );
    assert_eq!(
        image_plan
            .capabilities
            .iter()
            .map(|capability| capability.capability_id)
            .collect::<Vec<_>>(),
        vec!["clipboard.product.ocr"]
    );

    let file = item(11, ClipKind::Files);
    let file_plan = main_row_ai_capability_plan(Some(&file), &[]).unwrap();
    assert_eq!(
        file_plan.contexts,
        vec![ProductAiContextKind::SelectedFilePath]
    );
    assert_eq!(
        file_plan.capabilities[0].capability_id,
        "clipboard.product.ocr"
    );

    let selected = [item(2, ClipKind::Image), text.clone()];
    let mixed_plan = main_row_ai_capability_plan(Some(&file), &selected).unwrap();
    assert_eq!(
        mixed_plan.contexts,
        vec![
            ProductAiContextKind::SelectedImage,
            ProductAiContextKind::SelectedText
        ]
    );
    assert_eq!(mixed_plan.target_item_ids, vec![2, 7]);
    assert_eq!(
        mixed_plan
            .capabilities
            .iter()
            .map(|capability| capability.capability_id)
            .collect::<Vec<_>>(),
        vec![
            "clipboard.product.ocr",
            "clipboard.clean",
            "clipboard.explain",
            "clipboard.skill.translate"
        ]
    );
}

#[test]
fn main_row_ai_invocation_uses_capability_plan_targets() {
    let item = ClipItem {
        id: 42,
        kind: ClipKind::Text,
        preview: "selected".to_string(),
        text: Some("selected text".to_string()),
        source_app: "test".to_string(),
        file_paths: None,
        image_bytes: None,
        image_path: None,
        image_width: 0,
        image_height: 0,
        pinned: false,
        group_id: 0,
        created_at: "2026-01-01".to_string(),
    };
    let plan = main_row_ai_capability_plan(Some(&item), &[]).unwrap();
    let invocation = main_row_ai_invocation(&plan, "clipboard.clean", "make it concise").unwrap();

    assert_eq!(
        invocation,
        ProductAiInvocation {
            capability_id: "clipboard.clean".to_string(),
            input_text: "make it concise".to_string(),
            context_item_ids: vec![42],
        }
    );
    assert_eq!(
        main_row_ai_invocation(&plan, "clipboard.product.ocr", "read it"),
        None
    );
}

#[test]
fn settings_timer_ids_map_to_platform_neutral_tasks() {
    let ids = SettingsTimerIds {
        hide_scrollbar: 4,
        clear_save_hint: 8,
        dpi_fit: 15,
    };

    assert_eq!(
        settings_timer_task_for_id(4, ids),
        Some(SettingsTimerTask::HideScrollbar)
    );
    assert_eq!(
        settings_timer_task_for_id(8, ids),
        Some(SettingsTimerTask::ClearSaveHint)
    );
    assert_eq!(
        settings_timer_task_for_id(15, ids),
        Some(SettingsTimerTask::DpiFit)
    );
    assert_eq!(settings_timer_task_for_id(1, ids), None);
    assert_eq!(settings_timer_task_for_id(usize::MAX, ids), None);
}

#[test]
fn dpi_compensation_math_stays_platform_neutral() {
    assert_eq!(dpi_compensated_size(800, 600, 96, 144), (533, 400));
    assert_eq!(dpi_compensated_size(800, 600, 144, 96), (1200, 900));
    assert_eq!(dpi_compensated_size(0, -5, 0, 0), (1, 1));

    let bounds = UiRect::new(0, 0, 700, 500);
    let mut state = DpiCompensationState::default();
    let current = UiRect::new(100, 100, 900, 700);

    assert_eq!(state.resize_plan(current, bounds, 96, 2), None);
    let plan = state.resize_plan(current, bounds, 144, 2).unwrap();
    assert_eq!(
        (plan.x, plan.y, plan.width, plan.height),
        (167, 100, 533, 400)
    );
    assert_eq!(plan.monitor_dpi, 144);

    state.finish_resize(144);
    assert_eq!(
        state.resize_plan(UiRect::new(100, 100, 633, 500), bounds, 144, 2),
        None
    );
}
