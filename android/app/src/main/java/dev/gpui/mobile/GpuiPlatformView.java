package dev.gpui.mobile;

/** GPUI's lifecycle bridge. This app has no embedded media or platform views. */
public final class GpuiPlatformView {
    private GpuiPlatformView() {}
    public static void pauseAll() {}
    public static void resumeAll() {}
    public static void disposeAll() {}
}
