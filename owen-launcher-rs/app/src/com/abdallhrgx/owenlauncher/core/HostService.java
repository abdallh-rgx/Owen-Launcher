package com.abdallhrgx.owenlauncher.core;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.Service;
import android.content.Context;
import android.content.Intent;
import android.content.pm.ServiceInfo;
import android.os.Build;
import android.os.IBinder;

/**
 * HostService — the foreground service that keeps this process (and with it
 * the in-process Owen Server: HTTP 127.0.0.1:3551 + XMPP 127.0.0.1:8080)
 * alive while the launcher is backgrounded and the game runs. This is the
 * Rust twin of the original Kotlin app's two foreground services.
 *
 * Started and stopped from Rust through the JNI bridge
 * (crates/owen-app/src/jni_android.rs, fired by owen-launcher's
 * hosting_start / hosting_stop):
 *
 * <pre>
 *   HostService.start(context);  // hosting_start
 *   HostService.stop(context);   // hosting_stop
 * </pre>
 *
 * Deliberately dependency-free: no AndroidX, no app resources — the
 * notification is pure code over a framework icon, every 26+/29+ API is
 * SDK_INT-guarded (minSdk 24), and every entry point swallows its errors:
 * a failed foreground promotion must never kill the host process.
 */
public final class HostService extends Service {

    private static final String CHANNEL_ID = "owen-host";
    private static final int NOTIFICATION_ID = 1;

    @Override
    public void onCreate() {
        super.onCreate();
        // NotificationChannel is API 26+; below it the plain builder is used.
        // A channel failure must not prevent onStartCommand → startForeground.
        try {
            if (Build.VERSION.SDK_INT >= 26) {
                NotificationManager nm =
                        (NotificationManager) getSystemService(NOTIFICATION_SERVICE);
                if (nm != null) {
                    NotificationChannel channel = new NotificationChannel(
                            CHANNEL_ID,
                            "Owen Host",
                            NotificationManager.IMPORTANCE_LOW);
                    channel.setDescription("Keeps the Owen Server running while the game plays");
                    nm.createNotificationChannel(channel);
                }
            }
        } catch (Throwable t) {
            android.util.Log.w("HostService", "notification channel failed", t);
        }
    }

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        try {
            Notification.Builder b = Build.VERSION.SDK_INT >= 26
                    ? new Notification.Builder(this, CHANNEL_ID)
                    : new Notification.Builder(this);
            Notification n = b
                    .setSmallIcon(android.R.drawable.stat_notify_chat)
                    .setContentTitle("Owen Server")
                    .setContentText("Owen Server running")
                    .setOngoing(true)
                    .setOnlyAlertOnce(true)
                    .build();
            if (Build.VERSION.SDK_INT >= 29) {
                // The 3-arg startForeground declares the service type; the
                // constant is a compile-time int, inlined for API 24-28.
                startForeground(NOTIFICATION_ID, n,
                        ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC);
            } else {
                startForeground(NOTIFICATION_ID, n);
            }
        } catch (Throwable t) {
            // e.g. ForegroundServiceStartNotAllowedException (API 31+) —
            // never crash the host; the server keeps running without the
            // foreground promotion.
            android.util.Log.e("HostService", "startForeground failed", t);
        }
        return START_STICKY;
    }

    @Override
    public IBinder onBind(Intent intent) {
        return null;
    }

    /**
     * Start the foreground host service (called from Rust via JNI).
     * startForegroundService is API 26+ — on 24/25 the classic startService
     * still reaches onStartCommand → startForeground immediately because the
     * app is foreground when hosting is toggled.
     */
    public static void start(Context c) {
        try {
            Intent i = new Intent(c, HostService.class);
            if (Build.VERSION.SDK_INT >= 26) {
                c.startForegroundService(i);
            } else {
                c.startService(i);
            }
        } catch (Throwable t) {
            android.util.Log.w("HostService", "start failed", t);
        }
    }

    /** Stop the foreground host service (called from Rust via JNI). */
    public static void stop(Context c) {
        try {
            c.stopService(new Intent(c, HostService.class));
        } catch (Throwable t) {
            android.util.Log.w("HostService", "stop failed", t);
        }
    }
}
