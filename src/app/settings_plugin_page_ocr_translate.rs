use super::prelude::*;
use crate::win_system_params::{IDC_SET_TRANSLATE_APP_ID, IDC_SET_TRANSLATE_SECRET};

pub(super) unsafe fn settings_create_plugin_ocr_translate_page(
    st: &mut SettingsWndState,
    b: &SettingsPageBuilder,
    sec1: SettingsFormSectionLayout,
    sec2: SettingsFormSectionLayout,
    line_h: i32,
) {
    st.lb_ocr_title = b.label(
        st,
        tr("识别来源：", "Provider:"),
        sec1.left(),
        sec1.label_y(0, line_h),
        sec1.label_w(),
        line_h,
    );
    st.cb_ocr_provider = b.dropdown(
        st,
        tr("关闭", "Off"),
        IDC_SET_OCR_PROVIDER,
        sec1.field_x(),
        sec1.row_y(0),
        settings_scale(220),
    );
    if !st.cb_ocr_provider.is_null() {
        st.ownerdraw_ctrls.push(st.cb_ocr_provider);
    }
    st.lb_ocr_status = null_mut();
    st.lb_ocr_primary = b.label(
        st,
        tr("API Key：", "API Key:"),
        sec1.left(),
        sec1.label_y(2, line_h),
        sec1.label_w(),
        line_h,
    );
    st.ed_ocr_cloud_url = b.edit(
        st,
        "",
        IDC_SET_OCR_CLOUD_URL,
        sec1.field_x(),
        sec1.row_y(2),
        sec1.field_w(),
    );
    st.lb_ocr_secondary = b.label(
        st,
        tr("Secret Key：", "Secret Key:"),
        sec1.left(),
        sec1.label_y(3, line_h),
        sec1.label_w(),
        line_h,
    );
    st.ed_ocr_cloud_token = b.password_edit(
        st,
        "",
        IDC_SET_OCR_CLOUD_TOKEN,
        sec1.field_x(),
        sec1.row_y(3),
        sec1.field_w(),
    );
    st.btn_ocr_detect = b.button(
        st,
        tr("自动检测微信目录", "Auto-detect WeChat directory"),
        IDC_SET_OCR_WECHAT_DETECT,
        sec1.left(),
        sec1.row_y(3),
        settings_scale(180),
    );
    if !st.btn_ocr_detect.is_null() {
        st.ownerdraw_ctrls.push(st.btn_ocr_detect);
    }

    st.lb_translate_title = b.label(
        st,
        tr("翻译来源：", "Provider:"),
        sec2.left(),
        sec2.label_y(0, line_h),
        sec2.label_w(),
        line_h,
    );
    st.cb_translate_provider = b.dropdown(
        st,
        tr("关闭", "Off"),
        IDC_SET_TRANSLATE_PROVIDER,
        sec2.field_x(),
        sec2.row_y(0),
        settings_scale(220),
    );
    if !st.cb_translate_provider.is_null() {
        st.ownerdraw_ctrls.push(st.cb_translate_provider);
    }
    st.lb_translate_status = null_mut();
    st.lb_translate_primary = b.label(
        st,
        tr("APP ID：", "APP ID:"),
        sec2.left(),
        sec2.label_y(2, line_h),
        sec2.label_w(),
        line_h,
    );
    st.ed_translate_app_id = b.edit(
        st,
        "",
        IDC_SET_TRANSLATE_APP_ID,
        sec2.field_x(),
        sec2.row_y(2),
        sec2.field_w(),
    );
    st.lb_translate_secondary = b.label(
        st,
        tr("密钥：", "Secret:"),
        sec2.left(),
        sec2.label_y(3, line_h),
        sec2.label_w(),
        line_h,
    );
    st.ed_translate_secret = b.password_edit(
        st,
        "",
        IDC_SET_TRANSLATE_SECRET,
        sec2.field_x(),
        sec2.row_y(3),
        sec2.field_w(),
    );
    st.lb_translate_target = b.label(
        st,
        tr("目标语言：", "Target language:"),
        sec2.left(),
        sec2.label_y(4, line_h),
        sec2.label_w(),
        line_h,
    );
    st.cb_translate_target = b.dropdown(
        st,
        tr("简体中文", "Simplified Chinese"),
        IDC_SET_TRANSLATE_TARGET,
        sec2.field_x(),
        sec2.row_y(4),
        settings_scale(180),
    );
    if !st.cb_translate_target.is_null() {
        st.ownerdraw_ctrls.push(st.cb_translate_target);
    }
}
