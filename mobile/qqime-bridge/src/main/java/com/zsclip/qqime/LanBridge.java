package com.zsclip.qqime;

import android.app.KeyguardManager;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Context;
import android.inputmethodservice.InputMethodService;
import android.os.Build;
import android.os.Handler;
import android.os.Looper;
import android.os.PowerManager;
import android.os.SystemClock;
import android.provider.Settings;
import android.text.InputType;
import android.view.inputmethod.EditorInfo;
import android.widget.Toast;
import org.json.JSONObject;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;
import java.lang.ref.WeakReference;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.RejectedExecutionException;

/** In-process, opt-in text synchronization for the input method. */
public final class LanBridge {
    interface Listener {
        void onChanged();
        void onCloudConfirmation(QqCloudAuthorization.Request request);
        void onCloudLoginRequired();
    }
    interface Work<T> { T run() throws Exception; }
    interface Done<T> { void accept(T value, Exception error); }
    private static final Handler MAIN = new Handler(Looper.getMainLooper());
    private static Engine instance;
    private static boolean imeRunning;
    private static WeakReference<InputMethodService> activeIme = new WeakReference<InputMethodService>(null);

    public static void start(final InputMethodService service) {
        onMain(new Runnable() { public void run() {
            try {
                if (Build.VERSION.SDK_INT < 23) return;
                imeRunning = true;
                activeIme = new WeakReference<InputMethodService>(service);
                Engine engine = obtain(service);
                engine.refreshPolicy();
                engine.armAuto(0);
            } catch (Exception ignored) { /* Synchronization must not prevent keyboard startup. */ }
        }});
    }

    public static void stop() {
        onMain(new Runnable() { public void run() {
            imeRunning = false;
            activeIme.clear();
            if (instance != null) {
                instance.disarmAuto();
                instance.updateClipboardListener();
                if (instance.autoInFlight) {
                    instance.generation++;
                    instance.cancelNetwork();
                }
                if (instance.listener == null) {
                    instance.close();
                    instance = null;
                }
            }
        }});
    }

    public static void refresh() {
        onMain(new Runnable() { public void run() {
            if (instance != null) { instance.refreshPolicy(); instance.armAuto(0); }
        }});
    }

    static Engine attach(Context context, Listener listener) {
        Engine engine = obtain(context);
        engine.listener = listener;
        engine.refreshPolicy();
        return engine;
    }

    public static void sendItem(final Context context, final String text) {
        onMain(new Runnable() { public void run() { sendReadyItem(obtain(context), text, 0); }});
    }

    private static void sendReadyItem(final Engine engine, final String text, final int attempt) {
        if (!engine.ready && !engine.closed && attempt < 8) {
            MAIN.postDelayed(new Runnable() { public void run() { sendReadyItem(engine, text, attempt + 1); }}, 150);
            return;
        }
        engine.pushText(text, new Done<String>() { public void accept(String result, Exception error) {
            Toast.makeText(engine.context, error == null ? "已发送到电脑" : engine.message(error), Toast.LENGTH_LONG).show();
            if (!imeRunning && engine.listener == null && !engine.busy && instance == engine) {
                engine.close(); instance = null;
            }
        }});
    }

    static void detach(Engine engine, Listener listener) {
        if (engine.listener != listener) return;
        engine.listener = null;
        engine.cancelCloudAuthorization(false);
        engine.cloudConfirmationCode = "";
        engine.cancelPair(false);
        if (!imeRunning && instance == engine) {
            engine.close();
            instance = null;
        }
    }

    private static Engine obtain(Context context) {
        if (instance == null) instance = new Engine(context.getApplicationContext());
        return instance;
    }

    private static void onMain(Runnable action) {
        if (Looper.myLooper() == Looper.getMainLooper()) action.run(); else MAIN.post(action);
    }

    static final class Engine {
        final Context context;
        final PairingVault vault;
        final ClipboardManager clipboard;
        final ExecutorService worker = Executors.newSingleThreadExecutor();
        Listener listener;
        JSONObject pairing;
        String status = "正在读取配对状态…";
        String pairCode = "";
        boolean ready;
        boolean busy;
        boolean pairingNow;
        boolean cloudAuthorizationNow;
        String cloudConfirmationCode = "";
        private QqCloudAuthorization.Request cloudRequest;
        boolean auto;
        String mode = SyncMode.MANUAL;
        String serverMode = SyncMode.BOTH;
        private boolean clipboardListening;
        private boolean policyRefreshPending;
        private volatile boolean closed;
        private volatile long generation;
        private volatile HttpURLConnection activeConnection;
        private final ThreadLocal<Long> requestGeneration = new ThreadLocal<Long>();
        private long pairDeadline;
        private String pairId = "";
        private String pairEndpoint = "";
        private String seenLocal = "";
        private String suppressed = "";
        private String pendingText;
        private long localVersion;
        private long sequence;
        private int failures;
        private boolean baseline;
        private boolean autoScheduled;
        private boolean autoInFlight;
        private boolean sensitiveClipboard;
        private final Object autoToken = new Object();
        private final Object pairToken = new Object();
        private final ClipboardManager.OnPrimaryClipChangedListener clipboardListener =
                new ClipboardManager.OnPrimaryClipChangedListener() {
                    public void onPrimaryClipChanged() {
                        if (!imeRunning || closed) return;
                        if (autoPush()) captureChangedClipboard();
                        else if (autoPull()) {
                            // Receive-only still observes replacement/sensitivity so an in-flight
                            // response never overwrites a newer local copy or password-manager clip.
                            localVersion++;
                            refreshClipboardSensitivity();
                        }
                    }
                };

        Engine(Context context) {
            this.context = context;
            vault = new PairingVault(context);
            clipboard = (ClipboardManager) context.getSystemService(Context.CLIPBOARD_SERVICE);
            auto = false;
            submit(new Work<JSONObject>() { public JSONObject run() throws Exception {
                JSONObject saved = vault.read();
                if (saved != null) validatePairing(saved);
                return saved;
            }}, new Done<JSONObject>() { public void accept(JSONObject value, Exception error) {
                ready = true;
                pairing = value;
                if (error != null) {
                    auto = false;
                    vault.prefs.edit().putBoolean("auto", false).apply();
                    status = "配对凭据不可用，请重新配对";
                } else {
                    String savedMode = vault.prefs.getString("sync_mode", "");
                    mode = value == null ? SyncMode.MANUAL : SyncMode.normalize(savedMode.isEmpty()
                            ? (vault.prefs.getBoolean("auto", false) ? SyncMode.BOTH : SyncMode.MANUAL) : savedMode);
                    serverMode = SyncMode.normalize(vault.prefs.getString("server_sync_mode", SyncMode.BOTH));
                    auto = !SyncMode.MANUAL.equals(mode);
                    vault.prefs.edit().putString("sync_mode", mode).putBoolean("auto", auto).apply();
                    status = value == null ? "尚未配对" : "已配对 · " + SyncMode.label(mode);
                }
                updateClipboardListener();
                changed();
                if (imeRunning || listener != null) refreshPolicy();
                armAuto(0);
            }});
        }

        String displayAddress() {
            return pairing == null ? vault.prefs.getString("candidate", "") : pairing.optString("endpoint");
        }

        String deviceLabel() {
            return pairing == null ? "未连接电脑" : pairing.optString("name", "Windows")
                    .replaceAll("[\\p{Cc}\\p{Cf}]", "") + " · " + pairing.optString("endpoint");
        }

        boolean hasPairing() { return pairing != null; }

        void requestCloudAuthorization() {
            if (!pairedAvailable() || listener == null) return;
            if (!QqLoginCapability.hasExistingLogin()) {
                status = QqLoginCapability.loginRequiredMessage(context);
                changed();
                listener.onCloudLoginRequired();
                armAuto(3000);
                return;
            }
            cloudAuthorizationNow = true;
            cloudConfirmationCode = "";
            final JSONObject target = pairing;
            status = "正在获取电脑的 QQ 云剪贴板授权请求…";
            submit(new Work<QqCloudAuthorization.Request>() { public QqCloudAuthorization.Request run() throws Exception {
                return QqCloudAuthorization.parseRequest(http(target.optString("endpoint"),
                        "/v1/qq-cloud/authorization", null, target, QqCloudAuthorization.MAX_BYTES));
            }}, new Done<QqCloudAuthorization.Request>() { public void accept(QqCloudAuthorization.Request request, Exception error) {
                if (error != null) { finishCloudAuthorization(message(error)); return; }
                if (listener == null) { cancelCloudAuthorization(false); return; }
                cloudRequest = request;
                status = "请核对手机与电脑显示的授权指纹";
                changed();
                listener.onCloudConfirmation(request);
            }});
        }

        void confirmCloudAuthorization(final QqCloudAuthorization.Request request) {
            if (closed || !cloudAuthorizationNow || cloudRequest != request || pairing == null || listener == null || busy) return;
            cloudRequest = null;
            final JSONObject target = pairing;
            status = "正在加密发送 QQ 云剪贴板授权内容…";
            submit(new Work<String>() { public String run() throws Exception {
                checkRequest();
                JSONObject sealed = QqCloudAuthorization.sealAccount(context, request, vault.deviceId);
                String localCode = QqCloudAuthorization.confirmationCode(sealed);
                checkRequest();
                request.checkFresh();
                JSONObject receipt = http(target.optString("endpoint"), "/v1/qq-cloud/authorize", sealed,
                        target, QqCloudAuthorization.MAX_BYTES);
                QqCloudAuthorization.verifyReceipt(receipt, localCode);
                return localCode;
            }}, new Done<String>() { public void accept(String localCode, Exception error) {
                if (error instanceof QqCloudAuthorization.LoginRequiredException) {
                    finishCloudAuthorization(QqLoginCapability.loginRequiredMessage(context));
                    if (listener != null) listener.onCloudLoginRequired();
                } else if (error != null) {
                    finishCloudAuthorization(message(error) + "；请在电脑取消本次待确认授权后重试");
                } else {
                    cloudConfirmationCode = localCode;
                    finishCloudAuthorization("授权内容已送达，待电脑确认；请在电脑核对确认码 "
                            + QqCloudAuthorization.displayConfirmationCode(localCode) + " 后保存");
                }
            }});
        }

        void cancelCloudAuthorization(boolean report) {
            if (!cloudAuthorizationNow) return;
            generation++;
            cancelNetwork();
            cloudAuthorizationNow = false;
            cloudRequest = null;
            cloudConfirmationCode = "";
            if (report) status = "已取消本次手机授权；若电脑仍显示待确认内容，请在电脑取消";
            changed();
            armAuto(3000);
        }

        private void finishCloudAuthorization(String result) {
            cloudAuthorizationNow = false;
            cloudRequest = null;
            status = result;
            changed();
            armAuto(3000);
        }

        void pair(final String rawAddress) {
            if (!available()) return;
            disarmAuto();
            pairingNow = true;
            generation++;
            pairCode = "";
            status = "正在请求配对…";
            changed();
            submit(new Work<JSONObject>() { public JSONObject run() throws Exception {
                String endpoint = BridgeProtocol.endpoint(rawAddress);
                JSONObject response = http(endpoint, "/v1/pair/request", BridgeProtocol.pairRequest(vault.deviceId), null);
                response.put("endpoint", endpoint);
                return response;
            }}, new Done<JSONObject>() { public void accept(JSONObject value, Exception error) {
                if (error != null) { finishPair(message(error)); return; }
                pairId = value.optString("pair_id");
                pairCode = value.optString("code");
                if (!pairId.matches("[A-Za-z0-9_-]{1,128}") || pairCode.length() > 32) {
                    finishPair("电脑返回的配对信息无效"); return;
                }
                pairEndpoint = value.optString("endpoint");
                vault.prefs.edit().putString("candidate", pairEndpoint).apply();
                pairDeadline = SystemClock.elapsedRealtime() + 90000;
                status = "请在电脑 ZSClip 的局域网设置中核对配对码并允许连接";
                changed();
                schedulePair(1000);
            }});
        }

        void cancelPair(boolean report) {
            if (!pairingNow) return;
            generation++;
            cancelNetwork();
            pairingNow = false;
            pairCode = "";
            MAIN.removeCallbacksAndMessages(pairToken);
            status = "已取消本次配对";
            changed();
            armAuto(3000);
        }

        private void schedulePair(long delay) {
            MAIN.postAtTime(new Runnable() { public void run() {
                if (closed || !pairingNow) return;
                if (SystemClock.elapsedRealtime() >= pairDeadline) { finishPair("配对等待已超时，请重新请求"); return; }
                if (busy) { schedulePair(1000); return; }
                final long expected = generation;
                submit(new Work<JSONObject>() { public JSONObject run() throws Exception {
                    JSONObject response = http(pairEndpoint, "/v1/pair/status?id=" + URLEncoder.encode(pairId, "UTF-8"), null, null);
                    if ("accepted".equals(response.optString("status"))) {
                        JSONObject saved = new JSONObject().put("endpoint", pairEndpoint)
                                .put("token", response.optString("token"))
                                .put("device_id", response.optString("device_id"))
                                .put("name", response.optString("name", "Windows"));
                        validatePairing(saved);
                        if (expected != generation || closed) return null;
                        response.put("sealed", vault.seal(saved));
                        response.put("saved", saved);
                    }
                    return response;
                }}, new Done<JSONObject>() { public void accept(JSONObject response, Exception error) {
                    if (!pairingNow || expected != generation) return;
                    if (error != null) {
                        status = message(error) + "；等待重试配对";
                        changed(); schedulePair(3000); return;
                    }
                    if (response == null) { finishPair("配对已取消"); return; }
                    String state = response.optString("status");
                    if ("accepted".equals(state)) {
                        pairing = response.optJSONObject("saved");
                        vault.prefs.edit().putString("pairing_v1", response.optString("sealed"))
                                .putBoolean("auto", false).putString("sync_mode", SyncMode.MANUAL)
                                .remove("last_remote_key").remove("blocked_remote_key").remove("received_remote_keys").apply();
                        mode = SyncMode.MANUAL; auto = false;
                        baseline = false; pendingText = null; suppressed = "";
                        finishPair("配对成功，可长按剪贴板条目发送到电脑，也可选择自动同步方向");
                        refreshPolicy();
                    } else if ("rejected".equals(state) || "missing".equals(state)) {
                        finishPair("rejected".equals(state) ? "电脑已拒绝配对" : "配对请求已失效");
                    } else if ("pending".equals(state)) schedulePair(1000);
                    else finishPair("电脑返回未知配对状态");
                }});
            }}, pairToken, SystemClock.uptimeMillis() + delay);
        }

        private void finishPair(String result) {
            pairingNow = false; pairCode = ""; status = result;
            MAIN.removeCallbacksAndMessages(pairToken);
            changed(); armAuto(3000);
        }

        void forget() {
            if (closed || !ready) return;
            cancelCloudAuthorization(false);
            generation++;
            cancelNetwork();
            disarmAuto();
            auto = false; mode = SyncMode.MANUAL; pairing = null; baseline = false; pendingText = null; suppressed = "";
            updateClipboardListener();
            vault.clear();
            status = "已断开电脑；可在电脑的设备列表中移除此配对";
            changed();
        }

        void setAuto(boolean enabled) { setMode(enabled ? SyncMode.BOTH : SyncMode.MANUAL); }

        String effectiveMode() { return SyncMode.intersection(mode, serverMode); }
        private boolean autoPush() { return SyncMode.pushes(effectiveMode()); }
        private boolean autoPull() { return SyncMode.pulls(effectiveMode()); }

        void setMode(String value) {
            if (closed || !ready || pairingNow || cloudAuthorizationNow) return;
            String next = SyncMode.normalize(value);
            if (!SyncMode.MANUAL.equals(next) && pairing == null) { status = "请先完成配对"; changed(); return; }
            if (mode.equals(next)) return;
            generation++; cancelNetwork(); disarmAuto();
            mode = next; auto = !SyncMode.MANUAL.equals(mode);
            vault.prefs.edit().putString("sync_mode", mode).putBoolean("auto", auto).apply();
            baseline = false; pendingText = null; failures = 0;
            updateClipboardListener();
            status = "同步方式：" + SyncMode.label(mode);
            changed(); refreshPolicy(); armAuto(0);
        }

        private void updateClipboardListener() {
            boolean enabled = !closed && ready && pairing != null && imeRunning && (autoPush() || autoPull());
            if (enabled != clipboardListening && clipboard != null) {
                if (enabled) clipboard.addPrimaryClipChangedListener(clipboardListener);
                else clipboard.removePrimaryClipChangedListener(clipboardListener);
                clipboardListening = enabled;
            }
            if (enabled && autoPush() && !baseline) captureChangedClipboard();
            if (enabled && !autoPush()) refreshClipboardSensitivity();
        }

        private void refreshClipboardSensitivity() {
            try {
                ClipData clip = clipboard == null ? null : clipboard.getPrimaryClip();
                sensitiveClipboard = clip != null && Build.VERSION.SDK_INT >= 24 && clip.getDescription().getExtras() != null
                        && clip.getDescription().getExtras().getBoolean("android.content.extra.IS_SENSITIVE", false);
            } catch (RuntimeException unavailable) { sensitiveClipboard = true; }
        }

        void refreshPolicy() {
            if (closed || !ready || pairing == null || pairingNow || cloudAuthorizationNow) return;
            if (busy) { policyRefreshPending = true; return; }
            policyRefreshPending = false;
            final JSONObject target = pairing;
            submit(new Work<JSONObject>() { public JSONObject run() throws Exception {
                try { return http(target.optString("endpoint"), "/v1/sync-policy", null, target); }
                catch (LegacyPolicyException oldDesktop) { return null; }
            }}, new Done<JSONObject>() { public void accept(JSONObject response, Exception error) {
                if (error == null) {
                    if (response == null) serverMode = SyncMode.BOTH;
                    else {
                        try { serverMode = SyncMode.checkedPolicy(response, pairing.optString("device_id")); }
                        catch (Exception invalid) { serverMode = SyncMode.MANUAL; status = "电脑同步策略无效，自动同步已暂停"; }
                    }
                    vault.prefs.edit().putString("server_sync_mode", serverMode).apply();
                    if (!autoPush()) pendingText = null;
                    updateClipboardListener();
                }
                changed(); armAuto(0);
            }});
        }

        private void applyPolicy(JSONObject response) {
            if (response == null || pairing == null) return;
            JSONObject policy = response.optJSONObject("sync_policy");
            if (policy == null && "ZSCLIP_SYNC_POLICY_V1".equals(response.optString("protocol"))) policy = response;
            if (policy == null) return; // Older paired desktops have no direction API.
            try { serverMode = SyncMode.checkedPolicy(policy, pairing.optString("device_id")); }
            catch (Exception invalid) { serverMode = SyncMode.MANUAL; status = "电脑同步策略无效，自动同步已暂停"; }
            vault.prefs.edit().putString("server_sync_mode", serverMode).apply();
            if (!autoPush()) pendingText = null;
            updateClipboardListener();
            if (!autoPull() && pendingText == null) disarmAuto();
        }

        void push() {
            final String text;
            try { text = readClipboard(); } catch (Exception error) {
                status = message(error); changed(); return;
            }
            pushText(text, null);
        }

        void pushText(final String candidate, final Done<String> completion) {
            if (!pairedAvailable()) { if (completion != null) completion.accept(null, new IOException(status)); return; }
            final String text;
            try { text = BridgeProtocol.checkedText(candidate); }
            catch (Exception error) { status = message(error); changed(); if (completion != null) completion.accept(null, error); return; }
            final JSONObject target = pairing;
            final long seq = nextSequence();
            status = "正在发送到电脑…";
            submit(new Work<JSONObject>() { public JSONObject run() throws Exception {
                return sendText(target, text, seq, false);
            }}, new Done<JSONObject>() { public void accept(JSONObject value, Exception error) {
                if (error == null) {
                    applyPolicy(value);
                    if (text.equals(pendingText)) pendingText = null;
                    status = "已发送到电脑";
                } else status = message(error);
                if (completion != null) completion.accept(error == null ? status : null, error);
                changed(); armAuto(3000);
            }});
        }

        void pull() {
            if (!pairedAvailable()) return;
            final JSONObject target = pairing;
            status = "正在拉取电脑最新文本…";
            submit(new Work<JSONObject>() { public JSONObject run() throws Exception {
                return http(target.optString("endpoint"), "/v1/latest?mode=manual", null, target);
            }}, new Done<JSONObject>() { public void accept(JSONObject value, Exception error) {
                if (error == null) {
                    try { applyPolicy(value); status = acceptRemote(value, false, localVersion); }
                    catch (Exception e) { status = message(e); }
                } else status = message(error);
                changed(); armAuto(3000);
            }});
        }

        private boolean available() {
            if (closed || !ready) { status = "正在初始化，请稍后"; changed(); return false; }
            if (busy || pairingNow || cloudAuthorizationNow) { status = "正在处理请求，请稍后"; changed(); return false; }
            return true;
        }

        private boolean pairedAvailable() {
            if (!available()) return false;
            if (pairing == null) { status = "请先完成配对"; changed(); return false; }
            disarmAuto();
            return true;
        }

        void armAuto(long delay) {
            updateClipboardListener();
            if (closed || !ready || !imeRunning || pairing == null || pairingNow || cloudAuthorizationNow || autoScheduled || busy
                    || (!autoPull() && (!autoPush() || pendingText == null))) return;
            autoScheduled = true;
            MAIN.postAtTime(new Runnable() { public void run() { autoScheduled = false; tick(); }},
                    autoToken, SystemClock.uptimeMillis() + delay);
        }

        void disarmAuto() { MAIN.removeCallbacksAndMessages(autoToken); autoScheduled = false; }

        private void tick() {
            if (closed || !imeRunning || pairing == null || pairingNow || cloudAuthorizationNow) return;
            if (!autoPull() && (!autoPush() || pendingText == null)) return;
            if (busy || !canAutoRead()) { armAuto(3000); return; }
            if (autoPush()) captureChangedClipboard();
            disarmAuto();
            if (sensitiveClipboard) { armAuto(3000); return; }
            final JSONObject target = pairing;
            final String outgoing = autoPush() ? pendingText : null;
            final long version = localVersion;
            final long seq = nextSequence();
            submit(new Work<JSONObject>() { public JSONObject run() throws Exception {
                if (outgoing != null) {
                    JSONObject response = sendText(target, outgoing, seq, true);
                    response.put("pushed", !"direction".equals(response.optString("ignored")));
                    return response;
                }
                return http(target.optString("endpoint"), "/v1/latest?mode=auto", null, target);
            }}, new Done<JSONObject>() { public void accept(JSONObject response, Exception error) {
                if (!imeRunning) return;
                if (error == null) {
                    failures = 0;
                    applyPolicy(response);
                    if (outgoing != null && version == localVersion) pendingText = null;
                    try {
                        String result = outgoing != null ? null : acceptRemote(response, true, version);
                        if (result != null) status = result;
                        else if (outgoing != null && response.optBoolean("pushed", false)) status = "文本已自动发送到电脑";
                        else if ("direction".equals(response.optString("ignored"))) status = "电脑已暂停此方向的自动同步，手动发送仍可使用";
                    } catch (Exception e) { status = message(e); }
                } else {
                    failures = Math.min(5, failures + 1);
                    status = message(error) + "；自动同步将在 " + retryDelay() / 1000 + " 秒后重试";
                }
                changed(); armAuto(retryDelay());
            }}, true);
        }

        private long retryDelay() { return Math.min(60000L, 3000L << failures); }

        private boolean canAutoRead() {
            PowerManager power = (PowerManager) context.getSystemService(Context.POWER_SERVICE);
            KeyguardManager lock = (KeyguardManager) context.getSystemService(Context.KEYGUARD_SERVICE);
            if ((power != null && !power.isInteractive()) || (lock != null && lock.isKeyguardLocked())) return false;
            InputMethodService service = activeIme.get();
            EditorInfo editor = service == null ? null : service.getCurrentInputEditorInfo();
            if (editor != null) {
                int type = editor.inputType & InputType.TYPE_MASK_CLASS;
                int variation = editor.inputType & InputType.TYPE_MASK_VARIATION;
                if ((type == InputType.TYPE_CLASS_TEXT && (variation == InputType.TYPE_TEXT_VARIATION_PASSWORD
                        || variation == InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD
                        || variation == InputType.TYPE_TEXT_VARIATION_WEB_PASSWORD))
                        || (type == InputType.TYPE_CLASS_NUMBER && variation == InputType.TYPE_NUMBER_VARIATION_PASSWORD)) return false;
            }
            String current = Settings.Secure.getString(context.getContentResolver(), Settings.Secure.DEFAULT_INPUT_METHOD);
            return current != null && current.startsWith(context.getPackageName() + "/");
        }

        private void captureChangedClipboard() {
            if (!autoPush() || !imeRunning || !canAutoRead()) return;
            try {
                String text = readClipboard();
                String hash = BridgeProtocol.signature(text);
                if (!baseline) { baseline = true; seenLocal = hash; return; }
                if (hash.equals(seenLocal)) return;
                seenLocal = hash; localVersion++;
                if (hash.equals(suppressed)) { pendingText = null; return; }
                suppressed = "";
                pendingText = text;
                armAuto(250);
            } catch (IllegalArgumentException replaced) {
                if (!baseline) baseline = true;
                if (!seenLocal.isEmpty() || pendingText != null) localVersion++;
                seenLocal = ""; pendingText = null;
            } catch (Exception unavailable) {
                // A temporary Android clipboard access restriction must not discard unsent text.
            }
        }

        private String readClipboard() {
            if (clipboard == null) throw new IllegalStateException("系统剪贴板不可用");
            ClipData clip = clipboard.getPrimaryClip();
            if (clip == null) throw new IllegalStateException("系统暂时无法读取剪贴板，请保持设置页位于前台后重试");
            if (clip.getItemCount() == 0) throw new IllegalArgumentException("剪贴板没有文本");
            if (Build.VERSION.SDK_INT >= 24 && clip.getDescription().getExtras() != null
                    && clip.getDescription().getExtras().getBoolean("android.content.extra.IS_SENSITIVE", false)) {
                sensitiveClipboard = true;
                throw new IllegalArgumentException("敏感剪贴板内容不参与同步");
            }
            sensitiveClipboard = false;
            return BridgeProtocol.checkedText(clip.getItemAt(0).getText());
        }

        private String acceptRemote(JSONObject response, boolean automatic, long expectedVersion) throws Exception {
            JSONObject clip = response.optJSONObject("clip");
            if (clip == null) return automatic ? null : "电脑暂无剪贴板记录";
            String key = remoteKey(response, pairing);
            String barrier = vault.prefs.getString("blocked_remote_key", "");
            if (automatic && !barrier.isEmpty() && barrier.equals(key)) return null;
            if (automatic && key.equals(vault.prefs.getString("last_remote_key", ""))) return null;
            if (automatic && BridgeProtocol.alreadyReceived(vault.prefs.getString("received_remote_keys", "[]"), key)) return null;
            if (!"text".equals(clip.optString("kind"))) {
                if (automatic) vault.prefs.edit().putString("last_remote_key", key).apply();
                return automatic ? null : "电脑最新记录不是文本；此入口支持文本同步";
            }
            if (vault.deviceId.equals(clip.optString("origin_device_id"))) {
                vault.prefs.edit().putString("last_remote_key", key).apply();
                return automatic ? null : "这是本机已发送的内容，无需重复接收";
            }
            if (automatic && !autoPull()) return null;
            String text = BridgeProtocol.checkedText(clip.isNull("text") ? null : clip.optString("text"));
            if (automatic && (expectedVersion != localVersion || pendingText != null || sensitiveClipboard || !canAutoRead())) return null;
            if (clipboard == null) throw new IllegalStateException("系统剪贴板不可用");
            String signature = BridgeProtocol.signature(text);
            boolean historySaved = QqClipboardHistory.ingest(context, text);
            if (automatic && signature.equals(seenLocal)) {
                if (historySaved) rememberRemote(key);
                return historySaved ? null : "文本已在系统剪贴板，QQ 剪贴板记录保存失败，将重试";
            }
            String previousSuppressed = suppressed;
            suppressed = signature;
            try { clipboard.setPrimaryClip(ClipData.newPlainText("ZSClip", text)); }
            catch (Exception failure) { suppressed = previousSuppressed; throw failure; }
            seenLocal = suppressed; baseline = true; pendingText = null;
            vault.prefs.edit().remove("blocked_remote_key").apply();
            if (historySaved) rememberRemote(key);
            if (!historySaved) return "电脑文本已写入系统剪贴板，QQ 剪贴板记录保存失败，请重试";
            return automatic ? "电脑文本已自动加入 QQ 剪贴板" : "电脑文本已加入 QQ 剪贴板";
        }

        private void rememberRemote(String key) {
            String received = BridgeProtocol.rememberReceived(vault.prefs.getString("received_remote_keys", "[]"), key);
            vault.prefs.edit().putString("last_remote_key", key).putString("received_remote_keys", received)
                    .remove("blocked_remote_key").apply();
        }

        private long nextSequence() { sequence = Math.max(System.currentTimeMillis(), sequence + 1); return sequence; }

        private JSONObject sendText(JSONObject target, String text, long seq, boolean automatic) throws Exception {
            JSONObject response = http(target.optString("endpoint"), "/v1/clip?mode=" + (automatic ? "auto" : "manual"),
                    BridgeProtocol.textEnvelope(vault.deviceId, text, seq), target);
            return BridgeProtocol.checkedSendResponse(response, automatic);
        }

        private String remoteKey(JSONObject response, JSONObject target) {
            JSONObject clip = response.optJSONObject("clip");
            if (clip == null) return "";
            String key = target.optString("endpoint") + "|" + clip.optString("message_id") + "|"
                    + clip.optString("origin_device_id") + "|" + clip.optLong("origin_seq") + "|" + clip.optString("hash");
            if (key.length() > 2048) throw new IllegalArgumentException("电脑记录标识过长");
            return key;
        }

        private void validatePairing(JSONObject value) throws Exception {
            value.put("endpoint", BridgeProtocol.endpoint(value.getString("endpoint")));
            String token = value.optString("token");
            String remote = value.optString("device_id");
            if (!token.matches("[A-Za-z0-9._~+/=-]{16,4096}") || remote.isEmpty() || remote.length() > 256) {
                throw new IllegalArgumentException("电脑返回的配对凭据无效");
            }
            String name = value.optString("name", "Windows").trim();
            value.put("name", name.substring(0, Math.min(128, name.length())));
        }

        private JSONObject http(String endpoint, String path, JSONObject body, JSONObject authentication) throws Exception {
            return http(endpoint, path, body, authentication, BridgeProtocol.MAX_RESPONSE_BYTES);
        }

        private JSONObject http(String endpoint, String path, JSONObject body, JSONObject authentication, int limit) throws Exception {
            checkRequest();
            HttpURLConnection connection = AndroidLanNetwork.openConnection(context, new URL(endpoint + path));
            activeConnection = connection;
            try {
                connection.setInstanceFollowRedirects(false);
                connection.setUseCaches(false);
                connection.setConnectTimeout(4000);
                connection.setReadTimeout(5000);
                connection.setRequestMethod(body == null ? "GET" : "POST");
                connection.setRequestProperty("Accept", "application/json");
                connection.setRequestProperty("Content-Type", "application/json; charset=utf-8");
                if (authentication != null) {
                    connection.setRequestProperty("X-ZSClip-Device", vault.deviceId);
                    connection.setRequestProperty("X-ZSClip-Token", authentication.getString("token"));
                }
                if (body != null) {
                    checkRequest();
                    byte[] bytes = body.toString().getBytes(StandardCharsets.UTF_8);
                    if (bytes.length > limit) throw new IOException("发送数据过长");
                    connection.setDoOutput(true);
                    connection.setFixedLengthStreamingMode(bytes.length);
                    try (OutputStream stream = connection.getOutputStream()) { checkRequest(); stream.write(bytes); }
                }
                checkRequest();
                int code = connection.getResponseCode();
                if ("/v1/sync-policy".equals(path) && code == 404) throw new LegacyPolicyException();
                if (path.startsWith("/v1/qq-cloud/")) {
                    if (code == 404) throw new IOException("电脑 ZSClip 不支持此入口，请更新电脑版本");
                    if (code == 409) throw new IOException("授权窗口未开启或已过期，请先在电脑点击“连接 QQ 云剪贴板”");
                }
                if (code == 401 || code == 403) throw new IOException("配对已失效，请在电脑允许重新配对");
                if (code < 200 || code >= 300) throw new IOException("电脑响应 HTTP " + code);
                if (connection.getContentLength() > limit) throw oversizedResponse(limit);
                ByteArrayOutputStream bytes = new ByteArrayOutputStream();
                try (InputStream input = connection.getInputStream()) {
                    byte[] buffer = new byte[8192];
                    int count;
                    while ((count = input.read(buffer)) != -1) {
                        checkRequest();
                        if (bytes.size() + count > limit) throw oversizedResponse(limit);
                        bytes.write(buffer, 0, count);
                    }
                }
                String text = new String(bytes.toByteArray(), StandardCharsets.UTF_8);
                return new JSONObject(text.trim().isEmpty() ? "{}" : text);
            } finally { activeConnection = null; connection.disconnect(); }
        }

        private IOException oversizedResponse(int limit) {
            return limit == QqCloudAuthorization.MAX_BYTES ? new IOException("电脑授权响应过长，请重试") : new ResponseTooLargeException();
        }

        private <T> void submit(final Work<T> work, final Done<T> done) {
            submit(work, done, false);
        }

        private <T> void submit(final Work<T> work, final Done<T> done, boolean automatic) {
            if (closed) return;
            busy = true;
            autoInFlight = automatic;
            changed();
            final long expected = generation;
            try {
                worker.execute(new Runnable() { public void run() {
                    requestGeneration.set(expected);
                    T value = null; Exception error = null;
                    try { checkRequest(); value = work.run(); } catch (Exception e) { error = e; }
                    finally { requestGeneration.remove(); }
                    final T result = value; final Exception failure = error;
                    MAIN.postAtTime(new Runnable() { public void run() {
                        if (closed) return;
                        busy = false;
                        autoInFlight = false;
                        if (expected == generation) done.accept(result, failure);
                        changed();
                        if (policyRefreshPending && !pairingNow && !cloudAuthorizationNow) refreshPolicy();
                        if (!pairingNow) armAuto(3000);
                    }}, Engine.this, SystemClock.uptimeMillis());
                }});
            } catch (RejectedExecutionException error) {
                busy = false;
                if (!closed) { status = "同步线程已停止，请重新打开设置"; changed(); }
            }
        }

        private String message(Exception error) {
            if (error instanceof java.net.SocketTimeoutException || error instanceof java.net.ConnectException
                    || error instanceof java.net.UnknownHostException || error instanceof java.net.NoRouteToHostException) {
                return "电脑暂不可达，请检查同一 Wi-Fi、ZSClip 局域网服务及防火墙";
            }
            if (error instanceof org.json.JSONException) return "电脑返回的数据格式无效";
            String detail = error.getMessage();
            return detail == null || detail.length() > 180 ? "同步失败，请检查网络后重试" : detail;
        }

        private void changed() { if (!closed && listener != null) listener.onChanged(); }

        private void checkRequest() throws IOException {
            Long expected = requestGeneration.get();
            if (closed || Thread.currentThread().isInterrupted() || expected == null || expected.longValue() != generation) {
                throw new IOException("同步已停止");
            }
        }

        private void cancelNetwork() {
            final HttpURLConnection connection = activeConnection;
            if (connection != null) {
                Thread cancellation = new Thread(new Runnable() { public void run() { connection.disconnect(); }}, "ZSClip-disconnect");
                cancellation.setDaemon(true);
                cancellation.start();
            }
        }

        private static final class LegacyPolicyException extends IOException { }

        private static final class ResponseTooLargeException extends IOException {
            ResponseTooLargeException() { super("电脑最新记录超过 2 MB，请在电脑复制需要同步的文本"); }
        }

        void close() {
            closed = true; generation++; listener = null;
            cloudAuthorizationNow = false; cloudRequest = null; cloudConfirmationCode = "";
            disarmAuto();
            MAIN.removeCallbacksAndMessages(pairToken);
            MAIN.removeCallbacksAndMessages(this);
            if (clipboard != null) clipboard.removePrimaryClipChangedListener(clipboardListener);
            cancelNetwork();
            worker.shutdownNow();
            pendingText = null; pairing = null;
        }
    }

    private LanBridge() { }
}
