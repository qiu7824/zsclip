package com.zsclip.qqime;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.DialogInterface;
import android.content.Intent;
import android.os.Build;
import android.os.Bundle;
import android.text.InputType;
import android.view.View;
import android.widget.Button;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;
import android.widget.Spinner;
import android.widget.ArrayAdapter;
import android.widget.AdapterView;

/** Settings hosted inside the input method APK. */
public final class BridgeActivity extends Activity implements LanBridge.Listener {
    private LanBridge.Engine engine;
    private EditText address;
    private TextView device;
    private TextView status;
    private TextView code;
    private Button pair;
    private Button cancel;
    private Button push;
    private Button pull;
    private Button forget;
    private Button authorizeCloud;
    private Button discover;
    private TextView discoveryStatus;
    private LinearLayout discoveredDevices;
    private LanDiscovery discovery;
    private int discoveryCount;
    private TextView cloudConfirmation;
    private TextView cloudAvailability;
    private AlertDialog cloudDialog;
    private boolean destroyed;
    private Spinner syncMode;
    private TextView effectiveMode;
    private boolean updatingMode;

    @Override public void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setTitle("ZSClip 剪贴板同步");
        LinearLayout content = new LinearLayout(this);
        content.setOrientation(LinearLayout.VERTICAL);
        int padding = dp(20);
        content.setPadding(padding, padding, padding, padding);
        ScrollView scroll = new ScrollView(this);
        scroll.setFillViewport(true);
        scroll.addView(content);
        setContentView(scroll);
        heading(content, "ZSClip 剪贴板同步");
        note(content, "连接同一 Wi-Fi，在电脑 ZSClip 中开启“局域网同步”。点击下方发现的电脑，在电脑核对配对码并允许一次，即可手动发送；自动同步可在下方选择方向，无需扫码。", 15);
        if (Build.VERSION.SDK_INT < 23) {
            note(content, "此功能需要 Android 6.0 或更高版本，以安全保存配对凭据。", 17);
            return;
        }
        device = note(content, "", 15);
        discover = button(content, "查找附近的 ZSClip 电脑", new View.OnClickListener() {
            public void onClick(View view) { discoverComputers(); }
        });
        discoveryStatus = note(content, "", 14);
        discoveredDevices = new LinearLayout(this);
        discoveredDevices.setOrientation(LinearLayout.VERTICAL);
        content.addView(discoveredDevices, wide());
        note(content, "也可手动输入电脑同步页面显示的地址：", 14);
        address = new EditText(this);
        address.setSingleLine(true);
        address.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_URI);
        address.setHint("电脑 IP，例如 192.168.1.42；自定义端口可加 :38473");
        if (Build.VERSION.SDK_INT >= 26) address.setImportantForAutofill(View.IMPORTANT_FOR_AUTOFILL_NO);
        content.addView(address, wide());
        pair = button(content, "请求配对", new View.OnClickListener() { public void onClick(View view) {
            engine.pair(address.getText().toString());
        }});
        code = note(content, "", 23);
        code.setTextIsSelectable(true);
        cancel = button(content, "取消配对", new View.OnClickListener() { public void onClick(View view) { engine.cancelPair(true); }});
        heading(content, "同步方式");
        syncMode = new Spinner(this);
        syncMode.setAdapter(new ArrayAdapter<String>(this, android.R.layout.simple_spinner_dropdown_item, SyncMode.LABELS));
        content.addView(syncMode, wide());
        syncMode.setOnItemSelectedListener(new AdapterView.OnItemSelectedListener() {
            public void onItemSelected(AdapterView<?> parent, View view, int position, long id) {
                if (!updatingMode && engine != null && engine.ready) engine.setMode(SyncMode.VALUES[position]);
            }
            public void onNothingSelected(AdapterView<?> parent) { }
        });
        effectiveMode = note(content, "", 14);
        push = button(content, "手机剪贴板 → 电脑", new View.OnClickListener() { public void onClick(View view) { engine.push(); }});
        pull = button(content, "电脑最新文本 → 手机剪贴板", new View.OnClickListener() { public void onClick(View view) { engine.pull(); }});
        note(content, "长按输入法剪贴板条目，点击“发送到电脑”即可发送该项。仅手动模式不监听剪贴板、不轮询电脑；手机→电脑只在复制后发送。自动同步需输入法运行、屏幕点亮且解锁。实际方向同时受电脑设置限制；手动发送和接收不改变自动模式。敏感内容跳过同步，单条最多 262144 字符、1 MB。", 14);
        note(content, "局域网使用 HTTP 传输，请仅在可信 Wi-Fi 或私有 VPN 中使用。此入口支持文本，电脑和手机均需在线。", 14);
        heading(content, "QQ 云剪贴板");
        cloudAvailability = note(content, QqLoginCapability.notice(this), 15);
        note(content, "先在电脑 ZSClip 中点击“连接 QQ 云剪贴板”，再点击下方按钮并核对两端指纹。发送授权内容后，还需在电脑核对手机显示的确认码并保存。完成后，电脑可直接上传到当前 QQ 账号的官方云剪贴板，手机无需保持在线。", 14);
        authorizeCloud = button(content, "授权电脑使用 QQ 云剪贴板", new View.OnClickListener() {
            public void onClick(View view) { engine.requestCloudAuthorization(); }
        });
        cloudConfirmation = note(content, "", 19);
        cloudConfirmation.setTextIsSelectable(true);
        cloudConfirmation.setAccessibilityLiveRegion(View.ACCESSIBILITY_LIVE_REGION_POLITE);
        status = note(content, "", 16);
        status.setAccessibilityLiveRegion(View.ACCESSIBILITY_LIVE_REGION_POLITE);
        forget = button(content, "断开此电脑", new View.OnClickListener() { public void onClick(View view) {
            new AlertDialog.Builder(BridgeActivity.this).setTitle("断开电脑")
                    .setMessage("清除手机中的配对凭据并关闭自动同步。重新连接需要电脑允许配对。")
                    .setNegativeButton("保留", null)
                    .setPositiveButton("断开", new DialogInterface.OnClickListener() {
                        public void onClick(DialogInterface dialog, int which) { engine.forget(); }
                    }).show();
        }});
        engine = LanBridge.attach(this, this);
        address.setText(engine.displayAddress());
        onChanged();
        discoverComputers();
    }

    private void discoverComputers() {
        if (discovery != null) discovery.cancel();
        discoveredDevices.removeAllViews();
        discoveryCount = 0;
        discover.setEnabled(false);
        discoveryStatus.setText("正在通过局域网广播和本子网地址查找电脑，约需 6 秒…");
        final LanDiscovery scan = AndroidLanNetwork.discovery(this, address.getText().toString());
        discovery = scan;
        scan.start(new LanDiscovery.Listener() {
            public void found(final LanDiscovery.Device found) {
                runOnUiThread(new Runnable() { public void run() {
                    if (destroyed || discovery != scan) return;
                    discoveryCount++;
                    Button choice = button(discoveredDevices, found.name + "\n" + found.address(), new View.OnClickListener() {
                        public void onClick(View view) {
                            if (engine == null || engine.busy || engine.pairingNow || engine.cloudAuthorizationNow) return;
                            address.setText(found.address());
                            engine.pair(found.address());
                        }
                    });
                    choice.setContentDescription("配对 " + found.name + "，" + found.address());
                    discoveryStatus.setText("已找到 " + discoveryCount + " 台电脑，点击请求配对。");
                }});
            }
            public void finished(final boolean failed) {
                runOnUiThread(new Runnable() { public void run() {
                    if (destroyed || discovery != scan) return;
                    discover.setEnabled(true);
                    if (discoveryCount == 0) discoveryStatus.setText(scan.diagnostics()
                            + "\n请确认电脑已开启局域网同步并允许防火墙连接；路由器的访客网络或客户端隔离会阻止两端互通。可重新查找，或输入电脑同步页面显示的 IP 直接请求配对。");
                }});
            }
        });
    }

    @Override public void onChanged() {
        if (engine == null || destroyed || isFinishing()) return;
        device.setText(engine.deviceLabel());
        if (engine.ready && address.length() == 0 && !address.hasFocus()) address.setText(engine.displayAddress());
        status.setText(engine.status);
        cloudConfirmation.setVisibility(engine.cloudConfirmationCode.isEmpty() ? View.GONE : View.VISIBLE);
        cloudConfirmation.setText(engine.cloudConfirmationCode.isEmpty() ? "" : "授权内容已送达，待电脑确认\n确认码  "
                + QqCloudAuthorization.displayConfirmationCode(engine.cloudConfirmationCode)
                + "\n请在电脑逐组核对确认码，完全一致后保存。若不一致，请在电脑取消本次连接。");
        code.setText(engine.pairCode.isEmpty() ? "" : "配对码  " + engine.pairCode);
        code.setVisibility(engine.pairCode.isEmpty() ? View.GONE : View.VISIBLE);
        cancel.setVisibility(engine.pairingNow ? View.VISIBLE : View.GONE);
        boolean idle = engine.ready && !engine.busy && !engine.pairingNow && !engine.cloudAuthorizationNow;
        updatingMode = true;
        syncMode.setSelection(SyncMode.index(engine.mode));
        updatingMode = false;
        syncMode.setEnabled(idle && engine.hasPairing());
        effectiveMode.setText("当前自动方向：" + SyncMode.label(engine.effectiveMode())
                + "\n电脑允许：" + SyncMode.label(engine.serverMode));
        pair.setEnabled(idle);
        address.setEnabled(idle);
        push.setEnabled(idle && engine.hasPairing());
        pull.setEnabled(idle && engine.hasPairing());
        forget.setEnabled(idle && engine.hasPairing());
        cloudAvailability.setText(QqLoginCapability.notice(this));
        authorizeCloud.setEnabled(idle && engine.hasPairing());
    }

    @Override public void onCloudConfirmation(final QqCloudAuthorization.Request request) {
        if (destroyed || isFinishing() || engine == null) return;
        dismissCloudDialog();
        cloudDialog = new AlertDialog.Builder(this).setTitle("授权此电脑访问 QQ 云剪贴板")
                .setMessage(engine.deviceLabel() + "\n\n授权指纹：" + request.displayFingerprint()
                        + "\n\n请与电脑显示的指纹逐组核对，完全一致后再授权。"
                        + "\n\n授权内容将加密传给此电脑。发送后，请在电脑核对手机显示的确认码并保存；完成后电脑可直接访问当前 QQ 账号的官方云剪贴板，手机离线后仍可使用，直到授权失效或撤销。"
                        + "\n\n可在电脑断开 QQ 云剪贴板并清除授权；断开局域网配对不会自动撤销云授权。")
                .setNegativeButton("取消", new DialogInterface.OnClickListener() {
                    public void onClick(DialogInterface dialog, int which) { engine.cancelCloudAuthorization(true); }
                })
                .setPositiveButton("指纹一致，授权", new DialogInterface.OnClickListener() {
                    public void onClick(DialogInterface dialog, int which) {
                        if (!destroyed && !isFinishing()) engine.confirmCloudAuthorization(request);
                    }
                }).create();
        cloudDialog.setOnCancelListener(new DialogInterface.OnCancelListener() {
            public void onCancel(DialogInterface dialog) { engine.cancelCloudAuthorization(true); }
        });
        cloudDialog.show();
    }

    @Override public void onCloudLoginRequired() {
        if (destroyed || isFinishing()) return;
        dismissCloudDialog();
        final Intent login = QqCloudAuthorization.loginIntent(this);
        AlertDialog.Builder builder = new AlertDialog.Builder(this).setTitle("请先登录输入法账号")
                .setMessage(QqLoginCapability.loginRequiredMessage(this) + "\n\n若要使用原 QQ 账号云记录，手机号需已在该输入法账号中完成绑定。")
                .setNegativeButton("关闭", null);
        if (login != null) builder.setPositiveButton("打开账号登录页", new DialogInterface.OnClickListener() {
            public void onClick(DialogInterface dialog, int which) {
                try { startActivity(login); }
                catch (RuntimeException unavailable) {
                    if (!destroyed && !isFinishing()) {
                        cloudDialog = new AlertDialog.Builder(BridgeActivity.this).setTitle("QQ 账号页暂不可用")
                                .setMessage("请从 QQ 输入法首页打开账号设置，登录后重新授权电脑。")
                                .setPositiveButton("关闭", null).show();
                    }
                }
            }
        });
        cloudDialog = builder.show();
    }

    private void dismissCloudDialog() {
        if (cloudDialog != null) { cloudDialog.dismiss(); cloudDialog = null; }
    }

    @Override protected void onResume() {
        super.onResume();
        if (engine != null) { engine.refreshPolicy(); onChanged(); }
    }

    @Override protected void onDestroy() {
        destroyed = true;
        if (discovery != null) discovery.cancel();
        dismissCloudDialog();
        if (engine != null) LanBridge.detach(engine, this);
        super.onDestroy();
    }

    private void heading(LinearLayout parent, String text) {
        TextView title = note(parent, text, 21);
        title.setTypeface(null, android.graphics.Typeface.BOLD);
        title.setPadding(0, dp(14), 0, dp(8));
    }

    private TextView note(LinearLayout parent, String text, int size) {
        TextView view = new TextView(this);
        view.setText(text);
        view.setTextSize(size);
        view.setPadding(0, dp(7), 0, dp(7));
        parent.addView(view, wide());
        return view;
    }

    private Button button(LinearLayout parent, String text, View.OnClickListener listener) {
        Button view = new Button(this);
        view.setText(text);
        view.setAllCaps(false);
        view.setOnClickListener(listener);
        parent.addView(view, wide());
        return view;
    }

    private LinearLayout.LayoutParams wide() {
        return new LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT);
    }

    private int dp(int value) { return Math.round(value * getResources().getDisplayMetrics().density); }
}
