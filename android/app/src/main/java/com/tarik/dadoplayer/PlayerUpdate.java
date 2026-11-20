package com.tarik.dadoplayer;

import android.app.Activity;
import android.app.PendingIntent;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageInfo;
import android.content.pm.PackageInstaller;
import android.net.Uri;
import android.os.Build;
import android.provider.Settings;
import java.io.File;
import java.io.FileInputStream;
import java.io.IOException;
import java.io.OutputStream;
import java.util.concurrent.atomic.AtomicBoolean;

/** Hands the verified APK to Android's installer to replace this same package. */
public final class PlayerUpdate extends BroadcastReceiver {
    private static final AtomicBoolean INSTALLING = new AtomicBoolean();
    static volatile String status = "";

    private static File apk(Context context) {
        return new File(context.getFilesDir(), "updates/Dado-Player.apk");
    }

    static void install(Activity activity) {
        if (!INSTALLING.compareAndSet(false, true)) return;
        if (Build.VERSION.SDK_INT >= 26 && !activity.getPackageManager().canRequestPackageInstalls()) {
            status = "Erlaube Dado Player die Installation in Android. Tippe danach erneut auf Installieren.";
            INSTALLING.set(false);
            try {
                activity.startActivity(new Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,
                        Uri.parse("package:" + activity.getPackageName())));
            } catch (RuntimeException ignored) {
                status = "Aktiviere in Android den Zugriff „Unbekannte Apps installieren“ für Dado Player.";
            }
            return;
        }
        status = "Android installiert das Update …";
        Context app = activity.getApplicationContext();
        new Thread(() -> submit(app), "Dado package updater").start();
    }

    private static void submit(Context app) {
        File file = apk(app);
        PackageInstaller installer = app.getPackageManager().getPackageInstaller();
        int id = -1;
        try {
            PackageInfo archive = app.getPackageManager().getPackageArchiveInfo(file.getPath(), 0);
            PackageInfo installed = app.getPackageManager().getPackageInfo(app.getPackageName(), 0);
            if (archive == null || !installed.packageName.equals(archive.packageName)
                    || version(archive) <= version(installed)) {
                status = "Das APK gehört nicht zu Dado Player oder ist nicht neuer.";
                INSTALLING.set(false);
                return;
            }
            PackageInstaller.SessionParams params =
                    new PackageInstaller.SessionParams(PackageInstaller.SessionParams.MODE_FULL_INSTALL);
            params.setAppPackageName(app.getPackageName());
            params.setSize(file.length());
            if (Build.VERSION.SDK_INT >= 31) {
                params.setRequireUserAction(PackageInstaller.SessionParams.USER_ACTION_NOT_REQUIRED);
            }
            id = installer.createSession(params);
            try (PackageInstaller.Session session = installer.openSession(id)) {
                try (FileInputStream input = new FileInputStream(file);
                     OutputStream output = session.openWrite("base.apk", 0, file.length())) {
                    byte[] buffer = new byte[128 * 1024];
                    int count;
                    while ((count = input.read(buffer)) != -1) output.write(buffer, 0, count);
                    session.fsync(output);
                }
                Intent callback = new Intent(app, PlayerUpdate.class);
                int flags = PendingIntent.FLAG_UPDATE_CURRENT;
                if (Build.VERSION.SDK_INT >= 31) flags |= PendingIntent.FLAG_MUTABLE;
                session.commit(PendingIntent.getBroadcast(app, id, callback, flags).getIntentSender());
            }
        } catch (Exception error) {
            if (id >= 0) {
                try { installer.abandonSession(id); } catch (RuntimeException ignored) { }
            }
            status = "Android konnte die Installation nicht starten. Prüfe den freien Speicher und versuche es erneut.";
            INSTALLING.set(false);
        }
    }

    private static long version(PackageInfo info) {
        return Build.VERSION.SDK_INT >= 28 ? info.getLongVersionCode() : info.versionCode;
    }

    @Override public void onReceive(Context context, Intent intent) {
        int result = intent.getIntExtra(PackageInstaller.EXTRA_STATUS, PackageInstaller.STATUS_FAILURE);
        if (result == PackageInstaller.STATUS_PENDING_USER_ACTION) {
            Intent confirmation = intent.getParcelableExtra(Intent.EXTRA_INTENT);
            if (confirmation != null) {
                try {
                    confirmation.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                    context.startActivity(confirmation);
                    status = "Bestätige die Installation im Android-Dialog.";
                    return;
                } catch (RuntimeException ignored) { }
            }
            status = "Der Android-Dialog ließ sich nicht öffnen. Versuche es erneut.";
        } else if (result == PackageInstaller.STATUS_SUCCESS) {
            apk(context).delete();
            status = "Update installiert.";
        } else {
            status = result == PackageInstaller.STATUS_FAILURE_ABORTED
                    ? "Installation abgebrochen."
                    : "Android hat das Update abgelehnt. Prüfe App-Signatur, Speicher und Installationsberechtigung.";
        }
        INSTALLING.set(false);
    }
}
