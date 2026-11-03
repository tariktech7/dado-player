package android.app;

import androidx.games.activity.GameActivity;

/**
 * Dado-Player entry activity.
 *
 * Bridges the AndroidX Games Activity (provided by the
 * {@code androidx.games:games-activity} AAR) to the Rust {@code android_main}
 * symbol exposed by the {@code dadoplayer} cdylib. The native library name is
 * configured via the {@code android.app.lib_name} meta-data in the manifest.
 *
 * The AndroidX library supplies the Java {@code GameActivity} base class and the
 * native {@code libandroid_game_activity.so}; this subclass is the only glue
 * required on the Java side.
 */
public class GameActivity extends GameActivity {
}
