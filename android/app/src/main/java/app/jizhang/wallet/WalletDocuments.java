package app.jizhang.wallet;

import android.app.Activity;
import android.content.Intent;
import android.net.Uri;
import java.io.ByteArrayOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.nio.ByteBuffer;
import java.nio.charset.CodingErrorAction;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

/** System document picker and bounded background I/O; Rust validates the ledger. */
final class WalletDocuments {
    private static final int PICK_DOCUMENT = 2401;
    private static final int MAX_BYTES = 16 * 1024 * 1024;
    private final Activity activity;
    private final ExecutorService worker = Executors.newSingleThreadExecutor();
    private long request;
    private String export;

    WalletDocuments(Activity activity) { this.activity = activity; }

    void choose(long id, String json, String filename) {
        activity.runOnUiThread(() -> {
            if (request != 0) { nativeResult(id, 3, "另一个文件操作尚未结束"); return; }
            request = id;
            export = json;
            Intent intent = new Intent(json == null ? Intent.ACTION_OPEN_DOCUMENT : Intent.ACTION_CREATE_DOCUMENT);
            intent.addCategory(Intent.CATEGORY_OPENABLE);
            intent.setType(json == null ? "*/*" : "application/json");
            if (json != null) intent.putExtra(Intent.EXTRA_TITLE, filename);
            try { activity.startActivityForResult(intent, PICK_DOCUMENT); }
            catch (RuntimeException error) { finish(3, "无法打开系统文件选择器"); }
        });
    }

    boolean onResult(int code, int result, Intent data) {
        if (code != PICK_DOCUMENT) return false;
        if (request == 0) return true;
        if (result != Activity.RESULT_OK || data == null || data.getData() == null) {
            finish(0, "");
            return true;
        }
        final Uri uri = data.getData();
        final String snapshot = export;
        worker.execute(() -> {
            int kind;
            String content;
            try {
                if (snapshot == null) {
                    try (InputStream stream = activity.getContentResolver().openInputStream(uri);
                         ByteArrayOutputStream bytes = new ByteArrayOutputStream()) {
                        if (stream == null) throw new java.io.IOException();
                        byte[] buffer = new byte[8192];
                        int count;
                        while ((count = stream.read(buffer)) != -1) {
                            if (bytes.size() + count > MAX_BYTES) throw new IllegalArgumentException("备份文件过大（最多 16 MB）");
                            bytes.write(buffer, 0, count);
                        }
                        content = StandardCharsets.UTF_8.newDecoder()
                                .onMalformedInput(CodingErrorAction.REPORT)
                                .onUnmappableCharacter(CodingErrorAction.REPORT)
                                .decode(ByteBuffer.wrap(bytes.toByteArray())).toString();
                        kind = 2;
                    }
                } else {
                    try (OutputStream stream = activity.getContentResolver().openOutputStream(uri, "wt")) {
                        if (stream == null) throw new java.io.IOException();
                        stream.write(snapshot.getBytes(StandardCharsets.UTF_8));
                        stream.flush();
                    }
                    kind = 1;
                    content = "";
                }
            } catch (IllegalArgumentException error) {
                kind = 3;
                content = error.getMessage();
            } catch (Exception error) {
                kind = 3;
                content = snapshot == null ? "无法读取备份文件" : "无法保存备份文件";
            }
            final int completedKind = kind;
            final String completedContent = content;
            activity.runOnUiThread(() -> finish(completedKind, completedContent));
        });
        return true;
    }

    private void finish(int kind, String content) {
        long completed = request;
        request = 0;
        export = null;
        if (completed != 0) nativeResult(completed, kind, content);
    }
    void close() { finish(0, ""); worker.shutdownNow(); }
    private static native void nativeResult(long request, int kind, String content);
}
