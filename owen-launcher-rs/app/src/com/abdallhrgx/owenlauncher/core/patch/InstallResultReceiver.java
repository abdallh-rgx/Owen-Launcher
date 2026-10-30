package com.abdallhrgx.owenlauncher.core.patch;

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;

/**
 * InstallResultReceiver — the PackageInstaller status-broadcast sink.
 *
 * 1:1 with the handcrafted dex it replaces (tools/dex/make_dex.py) and with
 * the Kotlin original (InstallResultReceiver.kt):
 *
 * <pre>
 *   &lt;clinit&gt;  → System.loadLibrary("owen_app")   // the Rust cdylib; its
 *                                                // JNI_OnLoad RegisterNatives-es
 *                                                // nativeOnReceive
 *   onReceive → InstallResultReceiver.nativeOnReceive(intent)
 * </pre>
 *
 * The native side (crates/owen-app/src/jni_android.rs::native_on_receive)
 * routes the broadcast into the install result bus.
 *
 * NOTHING else may live in this class: it is created by the framework from a
 * PendingIntent while this process may be backgrounded, so its &lt;clinit&gt;
 * must stay minimal and exception-free.
 */
public class InstallResultReceiver extends BroadcastReceiver {

    static {
        System.loadLibrary("owen_app");
    }

    public static native void nativeOnReceive(Intent intent);

    @Override
    public void onReceive(Context context, Intent intent) {
        InstallResultReceiver.nativeOnReceive(intent);
    }
}
