package com.tarik.dadoplayer;

import android.app.UiModeManager;
import android.content.ActivityNotFoundException;
import android.content.Context;
import android.content.Intent;
import android.content.pm.ResolveInfo;
import android.content.res.Configuration;
import android.net.Uri;
import android.os.Bundle;
import android.text.InputType;
import android.util.Log;
import android.view.KeyEvent;
import android.view.View;
import android.view.WindowInsets;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputMethodManager;

import androidx.activity.OnBackPressedCallback;

import androidx.core.graphics.Insets;
import androidx.core.view.WindowInsetsCompat;

import com.google.androidgamesdk.gametextinput.State;

/**
 * Dado-Player entry activity.
 *
 * <p>Subclasses the Games Activity shipped by {@code androidx.games:games-activity}
 * (package {@code com.google.androidgamesdk}). Its native glue loads the Rust
 * library named by the {@code android.app.lib_name} manifest meta-data
 * ({@code dadoplayer} -&gt; {@code libdadoplayer.so}) and calls {@code android_main}.
 *
 * <p>The superclass is referenced by its fully-qualified name on purpose: a
 * single-type {@code import} of {@code GameActivity} would collide with this
 * class's own simple name (JLS 7.5.1), and would otherwise read as a cyclic
 * inheritance.
 *
 * <p>Three things winit cannot give us are implemented here and pushed into
 * Rust through JNI (see {@code src/platform/android/jni_bridge.rs}):
 * <ul>
 *   <li>soft-keyboard text — {@code stateChanged()} of the GameTextInput
 *       listener that GameActivity already implements;</li>
 *   <li>window insets — GameActivity 2.x forces edge-to-edge, so the window
 *       never shrinks for the keyboard or the gesture bar;</li>
 *   <li>back navigation — AppCompatActivity owns the back dispatcher, so it has
 *       to ask the Rust UI whether it can pop a screen first.</li>
 * </ul>
 */
public class GameActivity extends com.google.androidgamesdk.GameActivity {

    /**
     * Keyboard type for the field Rust currently has focused.
     *
     * <p>{@code GameActivity} builds its {@code gametextinput.Settings} once, in
     * the {@code InputEnabledSurfaceView} constructor, by calling
     * {@link #getImeEditorInfo()}. It keeps a <em>reference</em> to that
     * {@code EditorInfo}, so mutating it later through
     * {@code setImeEditorInfoFields()} is picked up by the next
     * {@code onCreateInputConnection()}. That is the only hook available: the
     * surface view's own {@code onCreateInputConnection()} is package-private
     * in {@code com.google.androidgamesdk}.
     *
     * <p>The default must not be 0. The surface view derives
     * {@code Settings.forwardKeyEvents} from {@code inputType != 0}: with 0 it
     * forwards raw key events to winit <em>in addition to</em> the
     * {@code stateChanged()} text stream, which double-types every character.
     */
    // Plain text, no auto-capitalisation: before Rust pushes the focused
    // field's type this is the EditorInfo the IME sees, and a stray
    // CAP_SENTENCES here would uppercase the first letter of a username.
    private static int sImeType = InputType.TYPE_CLASS_TEXT;

    /** IME_ACTION_DONE | IME_FLAG_NO_EXTRACT_UI | IME_FLAG_NO_FULLSCREEN. */
    private static final int IME_OPTIONS =
            EditorInfo.IME_ACTION_DONE
                    | EditorInfo.IME_FLAG_NO_EXTRACT_UI
                    | EditorInfo.IME_FLAG_NO_FULLSCREEN;

    private static GameActivity sSelf;
    private volatile float playerBrightness = -1;

    public static float playerBrightness(float requested) {
        GameActivity activity = sSelf;
        if (activity == null) return 0.5f;
        if (requested < 0) {
            if (activity.playerBrightness >= 0) return activity.playerBrightness;
            return android.provider.Settings.System.getInt(activity.getContentResolver(),
                    android.provider.Settings.System.SCREEN_BRIGHTNESS, 128) / 255f;
        }
        float level = Math.max(0, Math.min(1, requested));
        activity.playerBrightness = level;
        activity.runOnUiThread(() -> {
            android.view.Window window = activity.getWindow();
            android.view.WindowManager.LayoutParams attributes = window.getAttributes();
            attributes.screenBrightness = level;
            window.setAttributes(attributes);
        });
        return level;
    }

    public static boolean copyDeviceAddress(String address) {
        GameActivity activity = sSelf;
        if (activity == null) return false;
        android.content.ClipboardManager clipboard =
                activity.getSystemService(android.content.ClipboardManager.class);
        if (clipboard == null) return false;
        activity.runOnUiThread(() -> clipboard.setPrimaryClip(
                android.content.ClipData.newPlainText("Dado Player", address)));
        return true;
    }

    public static int mobileUpdate(int install) {
        GameActivity activity = sSelf;
        if (activity == null) return 0;
        activity.runOnUiThread(() -> {
            if (install == 1) {
                PlayerUpdate.install(activity);
            } else {
                try {
                    activity.startActivity(new Intent(Intent.ACTION_VIEW,
                            Uri.parse("https://github.com/tariktech7/dado-player/releases")));
                } catch (ActivityNotFoundException ignored) {
                    PlayerUpdate.status = "Auf diesem Gerät ist kein Browser verfügbar.";
                }
            }
        });
        return 1;
    }

    public static String mobileUpdateStatus() {
        return PlayerUpdate.status;
    }

    // ---- external players -------------------------------------------------
    // Wire codes must match PlayerBackend::android_code() in src/types.rs.
    private static final int BACKEND_MX = 0;
    private static final int BACKEND_VLC = 1;
    private static final int BACKEND_SYSTEM = 2;
    private static final int BACKEND_MPV = 3;
    private static final int BACKEND_MX_INTEGRATED = 4;
    private static final int BACKEND_MX_FALLBACK = 5;

    /** MX Player free / Pro and VLC package ids. Declared in <queries>. */
    private static final String MX_FREE = "com.mxtech.videoplayer.ad";
    private static final String MX_PRO = "com.mxtech.videoplayer.pro";
    private static final String VLC = "org.videolan.vlc";

    /** Status codes returned to Rust (never a URL — those carry credentials). */
    private static final int EXTERNAL_OK = 0;
    private static final int EXTERNAL_NO_APP = 1;
    private static final int EXTERNAL_ERROR = 2;

    /**
     * MIME types tried when handing a URL to another player. Panels disagree
     * about live and series formats: some expose {@code .ts}, some only
     * {@code .m3u8}, and a few declare no MIME type at all. A single
     * {@code video/*} intent is not enough — an {@code application/vnd.apple.mpegurl}
     * filter does not match a {@code video/*} intent. {@code null} means a bare
     * data URI with no type, which catches OEM players that filter on scheme.
     */
    private static final String[] MIME_CANDIDATES = {
        "video/*",
        "application/vnd.apple.mpegurl",
        "application/x-mpegURL",
        "application/octet-stream",
        null
    };

    /**
     * Diagnostic tag for the back key. Back is dead on this app on the emulator
     * and this log is how the cause gets decided: whether the key reaches
     * {@link #dispatchKeyEvent} at all, or only the {@code OnBackInvoked}
     * dispatcher. Remove the logs once the path is settled.
     */
    private static final String TAG = "DadoBack";

    // ---- natives implemented in libdadoplayer.so ----
    private static native void nativeOnImeText(
            String text, int selStart, int selEnd, int compStart, int compEnd, boolean dismissed);

    private static native void nativeOnInsets(
            int sysTop, int sysBottom, int sysLeft, int sysRight,
            int cutTop, int cutBottom, int cutLeft, int cutRight,
            int gestTop, int gestBottom, int gestLeft, int gestRight,
            int imeBottom);

    private static native boolean nativeCanGoBack();

    private static native void nativeRequestBack();

    /** Forward a TV remote / D-pad key (and its action) to the Rust key queue. */
    private static native void nativeOnKey(int keyCode, int action);

    /**
     * The one thing Rust cannot discover for itself: where it is allowed to
     * write. {@code directories} cannot resolve a config dir on Android
     * ({@code dirs-sys} hard-codes a {@code None} home-directory fallback
     * there), so the app used to fall back to {@code "."} - and an app
     * process's working directory is {@code "/"}, which is not writable. Every
     * save then failed and every failure was swallowed.
     */
    private static native void nativeOnStorage(String filesDir);

    /**
     * Whether the device reports {@code UI_MODE_TYPE_TELEVISION}. Rust has no
     * Context and therefore cannot ask UiModeManager itself. The result only
     * feeds {@code Settings.tv_mode = Auto}; the user can still force either
     * layout from the settings screen (also useful on TV boxes that forget to
     * declare {@code android.software.leanback}).
     */
    private static native void nativeOnTvDevice(boolean isTv);

    /**
     * Called from Rust ({@code jni_bridge::set_input_type}) when a text field
     * gains focus. Static so Rust never has to hold a reference to the activity.
     */
    public static void setImeFields(int inputType) {
        sImeType = inputType;
        GameActivity self = sSelf;
        if (self != null) {
            self.setImeEditorInfoFields(inputType, 0, IME_OPTIONS);
        }
    }

    /**
     * Open a stream in a separate video player app.
     *
     * <p>Called from Rust ({@code jni_bridge::open_external}) on the
     * {@code android_main} thread. {@code Activity.startActivity} ends in a
     * binder transaction and is safe from that thread; the new activity's
     * lifecycle still runs on the UI thread. This must stay synchronous so the
     * status code reaches Rust — a posted runnable cannot report a missing app.
     *
     * <p>Only a status code crosses back over JNI. Provider URLs embed
     * username/password in the path, so letting a Java exception message (which
     * often contains the intent's data URI) reach Rust would risk printing
     * credentials in the app's error box.
     *
     * @return 0 started, 1 no matching app installed, 2 other failure
     */
    public static int openExternalPlayer(String url, String title, int backend, long position) {
        final GameActivity self = sSelf;
        if (self == null || self.isFinishing() || self.isDestroyed()
                || url == null || url.length() == 0) {
            return EXTERNAL_ERROR;
        }
        final String label = (title == null || title.length() == 0) ? "Video" : title;
        try {
            final Uri uri = Uri.parse(url);
            // Explicit packages are visible through <queries><package> even on
            // Android 11+, so try them FIRST. A generic resolveActivity() on a
            // "video/*" + http URI is not a reliable canary: some player
            // filters only declare the HLS MIME type, others only a scheme.
            switch (backend) {
                case BACKEND_MX:
                    // Free build first, Pro second. Some devices ship only one,
                    // and the two are not interchangeable package names.
                    if (startWithPackage(self, uri, label, MX_FREE, position)
                            || startWithPackage(self, uri, label, MX_PRO, position)) {
                        return EXTERNAL_OK;
                    }
                    // Explicitly chosen and not installed. Report NO_APP so Rust
                    // falls back to the built-in player. Do NOT fall through to
                    // the generic chooser: the user asked for MX Player, and on
                    // the QA emulator "video/*" resolved to Google Photos, which
                    // opened a black screen instead of the stream.
                    return EXTERNAL_NO_APP;
                case BACKEND_VLC:
                    if (startWithPackage(self, uri, label, VLC, position)) {
                        return EXTERNAL_OK;
                    }
                    return EXTERNAL_NO_APP;
                case BACKEND_SYSTEM:
                default:
                    // Only "System-Auswahl" is supposed to show a chooser.
                    return self.startChooser(uri, label);
            }
        } catch (ActivityNotFoundException e) {
            return EXTERNAL_NO_APP;
        } catch (Exception e) {
            // Never log the exception itself: its message can contain the
            // intent data URI, and provider URLs carry username/password in
            // the path. Only the numeric backend is safe to print.
            Log.w(TAG, "openExternalPlayer failed (backend=" + backend + ")");
            return EXTERNAL_ERROR;
        }
    }

    /** Build one ACTION_VIEW intent for a URL / MIME combination. */
    private static Intent buildViewIntent(Uri uri, String mime, String label) {
        Intent i = new Intent(Intent.ACTION_VIEW);
        if (mime == null) {
            i.setData(uri);
        } else {
            i.setDataAndType(uri, mime);
        }
        i.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
        // MX Player and VLC both read the standard title extra; harmless for
        // players that ignore it.
        i.putExtra(Intent.EXTRA_TITLE, label);
        i.putExtra("title", label);
        return i;
    }

    /**
     * Try one explicit player package with every plausible MIME type.
     * A package that is not installed (or hidden by package visibility) makes
     * startActivity throw; that is not an error, just a reason to try the next.
     */
    private static boolean startWithPackage(
            GameActivity self, Uri uri, String label, String pkg, long position) {
        for (String mime : MIME_CANDIDATES) {
            Intent i = buildViewIntent(uri, mime, label);
            i.setPackage(pkg);
            i.putExtra("return_result", true);
            if (pkg.equals(VLC)) { i.putExtra("position", position); }
            else {
                i.putExtra("position", (int)Math.min(Integer.MAX_VALUE, position));
                i.putExtra("secure_uri", true);
                i.setClassName(pkg, pkg.equals(MX_FREE) ? "com.mxtech.videoplayer.ad.ActivityScreen" : "com.mxtech.videoplayer.ActivityScreen");
            }
            try {
                self.startActivityForResult(i, REQUEST_PLAYBACK);
                return true;
            } catch (ActivityNotFoundException e) {
                // Try the next MIME type, then the next package.
            }
        }
        return false;
    }

    private static final int REQUEST_PLAYBACK = 7207;
    private static native void nativePlaybackResult(long position, long duration, boolean completed);
    @Override protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        if (requestCode != REQUEST_PLAYBACK || data == null || resultCode == RESULT_CANCELED) return;
        Bundle extras = data.getExtras();
        if (extras == null) return;
        Object position = extras.containsKey("extra_position") ? extras.get("extra_position") : extras.get("position");
        Object duration = extras.containsKey("extra_duration") ? extras.get("extra_duration") : extras.get("duration");
        boolean completed = "playback_completion".equals(data.getStringExtra("end_by"));
        if (!(position instanceof Number) && !completed) return;
        nativePlaybackResult(position instanceof Number ? ((Number)position).longValue() : 0,
                duration instanceof Number ? ((Number)duration).longValue() : 0, completed);
    }

    /** Let Android pick among every visible handler; NO_APP when there is none. */
    private int startChooser(Uri uri, String label) {
        for (String mime : MIME_CANDIDATES) {
            Intent base = buildViewIntent(uri, mime, label);
            ResolveInfo resolved = getPackageManager().resolveActivity(base, 0);
            if (resolved == null) {
                continue;
            }
            Intent chooser = Intent.createChooser(base, label);
            // A chooser started from an Activity context needs no NEW_TASK, but
            // adding it covers OEMs that route through a resolver activity
            // with a cleared task affinity.
            chooser.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            try {
                startActivity(chooser);
                return EXTERNAL_OK;
            } catch (ActivityNotFoundException e) {
                return EXTERNAL_NO_APP;
            }
        }
        return EXTERNAL_NO_APP;
    }

    /**
     * Give the focused field a fresh IME session.
     *
     * <p>Called from Rust ({@code jni_bridge::seed}) whenever a TextEdit gains
     * focus. This is NOT cosmetic. GameTextInput keeps one
     * {@code InputConnection} with one {@code Editable} for the whole activity,
     * while egui has a separate String per field. Switching fields without
     * telling the IME leaves Gboard holding the PREVIOUS field's text: the next
     * keystroke appends to that stale buffer and {@code stateChanged} reports
     * e.g. {@code "probeprobe"} for a password the user typed as
     * {@code "probe"}. Worse, the suggestion strip keeps the old word, so a
     * "fix" that only re-bases Rust's diff would still show the wrong text.
     *
     * <p>{@code setTextInputState} clears and refills the connection's
     * {@code Editable}; {@code InputMethodManager.restartInput} then forces the
     * keyboard to drop its own composing buffer and re-query the connection.
     * It also makes the CURRENT {@code EditorInfo.inputType} take effect, which
     * otherwise is only read when a connection is first created — so the
     * password/URL keyboard now actually matches the focused field.
     *
     * <p>Everything runs on the UI thread: the JNI call arrives on
     * {@code android_main}, and GameTextInput's C++ glue uses a JNIEnv bound to
     * the UI thread. The whole sequence runs in ONE runnable so the state is
     * guaranteed to be in place before the restart.
     */
    public static void seedIme(String text) {
        final GameActivity self = sSelf;
        if (self == null) {
            return;
        }
        final String value = text == null ? "" : text;
        self.runOnUiThread(new Runnable() {
            @Override
            public void run() {
                self.restartTextInput(value);
            }
        });
    }

    private void restartTextInput(String text) {
        if (isFinishing() || isDestroyed()) {
            return;
        }
        // Stamp the keyboard type first: restartInput is what makes the new
        // EditorInfo visible to the IME.
        setImeEditorInfoFields(sImeType, 0, IME_OPTIONS);
        // codePointCount matches Rust's `chars().count()`, not UTF-16 units.
        int len = text.codePointCount(0, text.length());
        setTextInputState(new State(text, len, len, -1, -1));
        View target = mSurfaceView;
        if (target != null) {
            InputMethodManager imm =
                    (InputMethodManager) getSystemService(Context.INPUT_METHOD_SERVICE);
            if (imm != null) {
                imm.restartInput(target);
            }
        }
    }

    private android.hardware.display.DisplayManager refreshManager;
    private final android.hardware.display.DisplayManager.DisplayListener refreshListener =
            new android.hardware.display.DisplayManager.DisplayListener() {
        public void onDisplayAdded(int id) { requestNativeRefreshRate(); }
        public void onDisplayRemoved(int id) { requestNativeRefreshRate(); }
        public void onDisplayChanged(int id) { requestNativeRefreshRate(); }
    };

    private void requestNativeRefreshRate() {
        android.view.Display display = getWindowManager().getDefaultDisplay();
        if (display == null) return;
        android.view.Display.Mode current = display.getMode();
        float rate = current.getRefreshRate();
        for (android.view.Display.Mode mode : display.getSupportedModes()) {
            if (mode.getPhysicalWidth() == current.getPhysicalWidth()
                    && mode.getPhysicalHeight() == current.getPhysicalHeight()) {
                rate = Math.max(rate, mode.getRefreshRate());
            }
        }
        android.view.WindowManager.LayoutParams params = getWindow().getAttributes();
        // Hint only: preserve resolution, HDR, and the OS's thermal/power policy.
        if (Math.abs(params.preferredRefreshRate - rate) > 0.5f) {
            params.preferredRefreshRate = rate;
            getWindow().setAttributes(params);
        }
    }
    @Override protected void onResume() {
        super.onResume();
        refreshManager = (android.hardware.display.DisplayManager)getSystemService(Context.DISPLAY_SERVICE);
        if (refreshManager != null) refreshManager.registerDisplayListener(refreshListener, new android.os.Handler(android.os.Looper.getMainLooper()));
        requestNativeRefreshRate();
    }
    @Override protected void onPause() {
        if (refreshManager != null) refreshManager.unregisterDisplayListener(refreshListener);
        super.onPause();
    }

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        sSelf = this;
        super.onCreate(savedInstanceState);
        // Pushed before any Rust code can run: `android_main` starts from this
        // activity, and nothing in Rust touches `store` until it does.
        nativeOnStorage(getFilesDir().getAbsolutePath());
        UiModeManager ui = (UiModeManager) getSystemService(Context.UI_MODE_SERVICE);
        nativeOnTvDevice(ui != null
                && ui.getCurrentModeType() == Configuration.UI_MODE_TYPE_TELEVISION);
        installBackCallback();
    }

    /**
     * Registers the back handler with the androidx dispatcher, which is the path
     * that does not depend on key dispatch at all.
     *
     * <p>With {@code android:enableOnBackInvokedCallback="true"} in the manifest,
     * the system delivers back — button or gesture — to the window's
     * {@code OnBackInvokedDispatcher}, and androidx.activity bridges that to this
     * dispatcher on API 33+. That matters here because a
     * {@code GameActivity.InputEnabledSurfaceView} owns its own input channel:
     * if it consumes the key, no amount of {@code dispatchKeyEvent} work in the
     * activity can see it, but the system back callback still arrives.
     *
     * <p>The callback stays enabled. When the UI has nothing to pop, back means
     * "leave the app". Keep the native root activity in its background task:
     * winit's process-wide event loop cannot be recreated after finish().
     */
    private void installBackCallback() {
        getOnBackPressedDispatcher().addCallback(this, new OnBackPressedCallback(true) {
            @Override
            public void handleOnBackPressed() {
                Log.i(TAG, "handleOnBackPressed canGoBack=" + nativeCanGoBack());
                if (nativeCanGoBack()) {
                    nativeRequestBack();
                } else {
                    moveTaskToBack(true);
                }
            }
        });
    }

    @Override
    public boolean onKeyDown(int keyCode, KeyEvent event) {
        if (keyCode == KeyEvent.KEYCODE_BACK) {
            Log.i(TAG, "onKeyDown BACK");
        }
        return super.onKeyDown(keyCode, event);
    }

    @Override
    protected void onDestroy() {
        if (sSelf == this) {
            sSelf = null;
        }
        super.onDestroy();
    }

    /**
     * Stamps the current keyboard type before {@code super} hands the object to
     * the input connection. Without this the very first {@code EditorInfo} would
     * carry {@code inputType == 0} and enable raw key forwarding.
     */
    @Override
    public EditorInfo getImeEditorInfo() {
        EditorInfo info = super.getImeEditorInfo();
        if (info != null) {
            info.inputType = sImeType;
            info.imeOptions = IME_OPTIONS;
        }
        return info;
    }

    /** Called by the GameTextInput glue on every IME change (including composing text). */
    @Override
    public void stateChanged(State state, boolean dismissed) {
        super.stateChanged(state, dismissed);
        if (state == null) {
            nativeOnImeText("", 0, 0, -1, -1, true);
            return;
        }
        nativeOnImeText(
                state.text == null ? "" : state.text,
                state.selectionStart,
                state.selectionEnd,
                state.composingRegionStart,
                state.composingRegionEnd,
                dismissed);
    }

    /** Push status-bar / cutout / gesture / keyboard insets to Rust. */
    @Override
    public WindowInsetsCompat onApplyWindowInsets(View view, WindowInsetsCompat insets) {
        // Read the *incoming* insets, not the return value of super: the
        // GameActivity implementation applies/consumes them, so the returned
        // object reports zeros for the bars it already handled and the UI would
        // pad itself by nothing.
        WindowInsetsCompat out = super.onApplyWindowInsets(view, insets);
        WindowInsetsCompat src = insets != null ? insets : out;
        if (src != null) {
            Insets sys = src.getInsets(WindowInsetsCompat.Type.systemBars());
            Insets nav = src.getInsets(WindowInsetsCompat.Type.navigationBars());
            Insets cut = src.getInsets(WindowInsetsCompat.Type.displayCutout());
            Insets gest = src.getInsets(WindowInsetsCompat.Type.systemGestures());
            Insets ime = src.getInsets(WindowInsetsCompat.Type.ime());
            android.util.Log.d("DadoInsets", "sys=" + sys.top + "," + sys.bottom
                    + " nav=" + nav.top + "," + nav.bottom
                    + " gest=" + gest.top + "," + gest.bottom
                    + " cut=" + cut.top + "," + cut.bottom
                    + " ime=" + ime.bottom);
            // Left/right on a separate line: the side inset is what decides the
            // page margin, and a single line got unreadable past ~4 types.
            android.util.Log.d("DadoInsets", "L/R sys=" + sys.left + "," + sys.right
                    + " cut=" + cut.left + "," + cut.right
                    + " gest=" + gest.left + "," + gest.right
                    + " nav=" + nav.left + "," + nav.right);
            nativeOnInsets(
                    Math.max(sys.top, cut.top),
                    Math.max(Math.max(sys.bottom, nav.bottom), Math.max(gest.bottom, cut.bottom)),
                    Math.max(sys.left, cut.left),
                    Math.max(sys.right, cut.right),
                    cut.top, cut.bottom, cut.left, cut.right,
                    gest.top, gest.bottom, gest.left, gest.right,
                    ime.bottom);
        }
        return out;
    }

    /**
     * BACK is dispatched here, because it never reaches {@link #onBackPressed()}.
     *
     * `GameActivity` overrides `onKeyDown`/`onKeyUp` to ask its native key
     * handler first and to skip `super` when that reports the key as consumed.
     * The key reaches `onBackPressed()` through `Activity.onKeyUp`, so when the
     * native handler takes it, `onBackPressed()` is never called at all and back
     * silently does nothing — on every screen, including the profile-delete
     * confirm, where back has to cancel the dialog instead of closing the
     * activity underneath it.
     *
     * Evidence: `javap -c` on `com.google.androidgamesdk.GameActivity` from
     * games-activity 2.0.2 shows `onKeyDownNative(handle, event)` followed by
     * `ifne 14` — return true — before `super.onKeyDown`, and the identical
     * shape in `onKeyUp`.
     *
     * STATUS: this override is in place but **did not fix it**. On the emulator,
     * back still changed nothing (three byte-identical captures) and the
     * override is verified present in the shipped dex, so the key does not reach
     * the Activity at all — most likely because
     * {@code GameActivity.InputEnabledSurfaceView} owns its own input channel.
     * The fix under test is {@link #installBackCallback}, which does not go
     * through key dispatch. The {@code DadoBack} log line here is what decides
     * whether this method ever runs.
     *
     * ACTION_DOWN is consumed without doing anything so it cannot leak into the
     * native handler; the action runs on ACTION_UP, which is what a press means.
     * The IME gets BACK first while it is up (it is a separate window), so this
     * does not steal the "close the keyboard" behaviour.
     */
    /**
     * Is the soft keyboard currently owning the screen?
     *
     * <p>Queried at dispatch time rather than cached from
     * {@link #onApplyWindowInsets}: a cached flag goes stale when the IME is
     * hidden without an inset pass, and then every remote key would be
     * swallowed forever.
     */
    private boolean imeVisible() {
        View decor = getWindow().getDecorView();
        WindowInsets wi = decor.getRootWindowInsets();
        return wi != null
                && WindowInsetsCompat.toWindowInsetsCompat(wi, decor)
                        .isVisible(WindowInsetsCompat.Type.ime());
    }

    @Override
    public boolean dispatchKeyEvent(KeyEvent event) {
        /**
         * While the IME is up, EVERY key belongs to the keyboard: D-pad moves
         * between its keys, ENTER/DPAD_CENTER commits or fires IME_ACTION_DONE,
         * BACK dismisses it. Consuming them here (and forwarding them to the
         * Rust focus engine) used to mean two things on the login screen:
         * <ul>
         *   <li>a TV remote could not type a URL or password at all;</li>
         *   <li>ENTER activated the first registered control, which with the
         *       keyboard up is "Anmelden", so a half-filled form was submitted
         *       on every Done press.</li>
         * </ul>
         * Rust has a second, independent guard (platform::text_focus()) for
         * hardware keyboards that produce keys without showing an IME window.
         */
        if (imeVisible()) {
            return super.dispatchKeyEvent(event);
        }
        /**
         * TV remote / D-pad / media keys are consumed here and forwarded to the
         * Rust focus engine through JNI. BOTH ACTION_DOWN and ACTION_UP are
         * consumed (return true) so GameActivity's own onKeyDownNative handler
         * and the InputEnabledSurfaceView cannot double-process them and move
         * the focus twice per press. BACK is deliberately excluded (handled
         * below via the back dispatcher) and every other key — especially
         * printable text — falls through to super so the IME keeps working.
         *
         * Every keycode listed here MUST have a matching arm in
         * Java_com_tarik_dadoplayer_GameActivity_nativeOnKey in jni_bridge.rs.
         * KEYCODE_MENU is therefore NOT consumed: this app has no options menu,
         * so swallowing it would leave a dead button on a TV remote. Let it
         * fall through to the framework instead.
         */
        switch (event.getKeyCode()) {
            case KeyEvent.KEYCODE_PROG_RED:
            case KeyEvent.KEYCODE_PROG_GREEN:
            case KeyEvent.KEYCODE_PROG_YELLOW:
            case KeyEvent.KEYCODE_PROG_BLUE:
                if (event.getRepeatCount() == 0) {
                    nativeOnKey(event.getKeyCode(), event.getAction());
                }
                return true;
            case KeyEvent.KEYCODE_0:
            case KeyEvent.KEYCODE_NUMPAD_0:
                if (event.getRepeatCount() == 0) {
                    nativeOnKey(event.getKeyCode(), event.getAction());
                }
                break; // Keep the printable key available to text fields.
            case KeyEvent.KEYCODE_DPAD_CENTER:
            case KeyEvent.KEYCODE_ENTER:
            case KeyEvent.KEYCODE_NUMPAD_ENTER:
                if (event.getRepeatCount() == 0) {
                    nativeOnKey(event.getKeyCode(), event.getAction());
                }
                return true;
            case KeyEvent.KEYCODE_DPAD_UP:
            case KeyEvent.KEYCODE_DPAD_DOWN:
            case KeyEvent.KEYCODE_DPAD_LEFT:
            case KeyEvent.KEYCODE_DPAD_RIGHT:
            case KeyEvent.KEYCODE_MEDIA_PLAY_PAUSE:
            case KeyEvent.KEYCODE_MEDIA_PLAY:
            case KeyEvent.KEYCODE_MEDIA_PAUSE:
                nativeOnKey(event.getKeyCode(), event.getAction());
                return true;
            default:
                break;
        }
        if (event.getKeyCode() == KeyEvent.KEYCODE_BACK) {
            Log.i(TAG, "dispatchKeyEvent BACK action=" + event.getAction());
            if (event.getAction() == KeyEvent.ACTION_UP && !event.isCanceled()) {
                onBackPressed();
            }
            return true;
        }
        return super.dispatchKeyEvent(event);
    }

    @Override
    public void onBackPressed() {
        Log.i(TAG, "onBackPressed canGoBack=" + nativeCanGoBack());
        if (nativeCanGoBack()) {
            nativeRequestBack();
        } else {
            moveTaskToBack(true);
        }
    }
}
