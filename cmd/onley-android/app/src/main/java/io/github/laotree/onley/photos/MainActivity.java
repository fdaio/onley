package io.github.laotree.onley.photos;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.Intent;
import android.content.SharedPreferences;
import android.database.Cursor;
import android.graphics.Typeface;
import android.net.Uri;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.provider.DocumentsContract;
import android.view.View;
import android.view.ViewGroup;
import android.widget.ArrayAdapter;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.ListView;
import android.widget.TextView;
import android.widget.Toast;

import java.io.IOException;
import java.io.InputStream;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.text.SimpleDateFormat;
import java.util.ArrayList;
import java.util.Date;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

/**
 * Finds duplicate photos under a folder the user grants through the Storage
 * Access Framework. SAF needs no storage permission, and deleting through the
 * granted tree needs no separate system consent.
 */
public final class MainActivity extends Activity {
    private static final int REQUEST_TREE = 1;
    private static final String PREFS = "onley";
    private static final String KEY_TREE = "tree";
    private static final String[] PROJECTION = {
            DocumentsContract.Document.COLUMN_DOCUMENT_ID,
            DocumentsContract.Document.COLUMN_DISPLAY_NAME,
            DocumentsContract.Document.COLUMN_MIME_TYPE,
            DocumentsContract.Document.COLUMN_SIZE,
            DocumentsContract.Document.COLUMN_LAST_MODIFIED,
    };

    private final ExecutorService executor = Executors.newSingleThreadExecutor();
    private final Handler main = new Handler(Looper.getMainLooper());
    private final Map<String, Uri> uris = new HashMap<>();
    private final Map<String, String> keep = new HashMap<>();
    private final List<Grouping.Group> groups = new ArrayList<>();

    private Uri treeUri;
    private Grouping.Policy policy = Grouping.Policy.NEWEST;
    private boolean busy;
    private TextView summary;
    private TextView status;
    private TextView folder;
    private Button select;
    private Button scan;
    private Button delete;
    private ListView list;
    private ArrayAdapter<String> adapter;

    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        setContentView(buildUi());
        treeUri = loadTree();
        if (treeUri != null) {
            takePermission(treeUri);
        }
        refresh();
    }

    private ViewGroup buildUi() {
        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setPadding(dp(18), dp(18), dp(18), dp(18));

        TextView title = new TextView(this);
        title.setText(R.string.app_name);
        title.setTextSize(28);
        title.setTypeface(null, Typeface.BOLD);
        root.addView(title, matchWrap());

        TextView tagline = new TextView(this);
        tagline.setText(R.string.tagline);
        tagline.setTextSize(13);
        root.addView(tagline, matchWrap());

        folder = new TextView(this);
        folder.setTextSize(12);
        folder.setPadding(0, dp(10), 0, dp(10));
        root.addView(folder, matchWrap());

        summary = new TextView(this);
        summary.setTextSize(14);
        summary.setTypeface(null, Typeface.BOLD);
        root.addView(summary, matchWrap());

        status = new TextView(this);
        status.setTextSize(12);
        status.setPadding(0, dp(4), 0, dp(8));
        root.addView(status, matchWrap());

        LinearLayout actions = new LinearLayout(this);
        actions.setOrientation(LinearLayout.HORIZONTAL);
        select = new Button(this);
        select.setText(R.string.select_folder);
        select.setOnClickListener(view -> chooseFolder());
        scan = new Button(this);
        scan.setText(R.string.scan);
        scan.setOnClickListener(view -> scan());
        actions.addView(select, weighted());
        actions.addView(scan, weighted());
        root.addView(actions, matchWrap());

        delete = new Button(this);
        delete.setOnClickListener(view -> confirmDelete());
        root.addView(delete, matchWrap());

        list = new ListView(this);
        adapter = new ArrayAdapter<>(this, android.R.layout.simple_list_item_1, new ArrayList<>());
        list.setAdapter(adapter);
        list.setOnItemClickListener((parent, view, position, id) -> showKeepDialog(groups.get(position)));
        root.addView(list, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, 0, 1f));

        return root;
    }

    private LinearLayout.LayoutParams matchWrap() {
        return new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
    }

    private LinearLayout.LayoutParams weighted() {
        return new LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f);
    }

    private int dp(int value) {
        return Math.round(value * getResources().getDisplayMetrics().density);
    }

    private Uri loadTree() {
        SharedPreferences prefs = getSharedPreferences(PREFS, MODE_PRIVATE);
        String saved = prefs.getString(KEY_TREE, null);
        return saved == null ? null : Uri.parse(saved);
    }

    private void saveTree(Uri uri) {
        getSharedPreferences(PREFS, MODE_PRIVATE)
                .edit()
                .putString(KEY_TREE, uri.toString())
                .apply();
    }

    private void chooseFolder() {
        Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT_TREE);
        intent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION
                | Intent.FLAG_GRANT_WRITE_URI_PERMISSION
                | Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION);
        startActivityForResult(intent, REQUEST_TREE);
    }

    @Override
    protected void onActivityResult(int request, int result, Intent data) {
        super.onActivityResult(request, result, data);
        if (request != REQUEST_TREE || result != RESULT_OK || data == null || data.getData() == null) {
            return;
        }
        treeUri = data.getData();
        saveTree(treeUri);
        takePermission(treeUri);
        groups.clear();
        uris.clear();
        keep.clear();
        status.setText("");
        refresh();
    }

    private void takePermission(Uri uri) {
        try {
            getContentResolver().takePersistableUriPermission(
                    uri,
                    Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_GRANT_WRITE_URI_PERMISSION);
        } catch (SecurityException error) {
            status.setText(R.string.permission_revoked);
        }
    }

    private void scan() {
        if (treeUri == null) {
            Toast.makeText(this, R.string.no_folder, Toast.LENGTH_SHORT).show();
            return;
        }
        setBusy(true);
        status.setText(R.string.scanning);
        groups.clear();
        uris.clear();
        keep.clear();
        refresh();

        executor.execute(() -> {
            List<Grouping.Photo> photos = new ArrayList<>();
            int failed = 0;
            String firstError = null;
            try {
                List<Document> documents = listDocuments(treeUri);
                for (Document document : documents) {
                    try {
                        String hash = md5(document.uri);
                        photos.add(new Grouping.Photo(
                                document.uri.toString(),
                                document.name,
                                document.size,
                                document.modified,
                                hash));
                        uris.put(document.uri.toString(), document.uri);
                    } catch (Exception error) {
                        failed++;
                        if (firstError == null) {
                            firstError = error.getMessage();
                        }
                    }
                }
            } catch (Exception error) {
                failed++;
                firstError = error.getMessage();
            }

            final int failures = failed;
            final String error = firstError;
            final List<Grouping.Group> found = Grouping.duplicates(photos);
            main.post(() -> {
                groups.clear();
                groups.addAll(found);
                applyDefaultKeep();
                setBusy(false);
                refresh();
                if (found.isEmpty() && failures == 0) {
                    status.setText(R.string.no_duplicates);
                } else if (failures > 0) {
                    status.setText(getString(R.string.scan_failed)
                            + (error == null ? "" : ": " + error));
                } else {
                    status.setText(R.string.scan_done);
                }
            });
        });
    }

    private List<Document> listDocuments(Uri tree) throws Exception {
        List<Document> documents = new ArrayList<>();
        String root = DocumentsContract.getTreeDocumentId(tree);
        walk(tree, root, documents, new HashSet<>(), 0);
        return documents;
    }

    private void walk(Uri tree, String documentId, List<Document> out, Set<String> seen, int depth) {
        if (depth > 32 || !seen.add(documentId)) {
            return;
        }
        Uri children = DocumentsContract.buildChildDocumentsUriUsingTree(tree, documentId);
        try (Cursor cursor = getContentResolver().query(children, PROJECTION, null, null, null)) {
            if (cursor == null) {
                return;
            }
            while (cursor.moveToNext()) {
                String id = cursor.getString(0);
                String name = cursor.getString(1);
                String mime = cursor.getString(2);
                long size = cursor.isNull(3) ? 0 : cursor.getLong(3);
                long modified = cursor.isNull(4) ? 0 : cursor.getLong(4);
                if (DocumentsContract.Document.MIME_TYPE_DIR.equals(mime)) {
                    walk(tree, id, out, seen, depth + 1);
                } else if (isImage(name, mime)) {
                    out.add(new Document(
                            DocumentsContract.buildDocumentUriUsingTree(tree, id),
                            name == null ? "" : name,
                            size,
                            modified));
                }
            }
        } catch (SecurityException error) {
            throw new SecurityException(getString(R.string.permission_revoked), error);
        }
    }

    private static boolean isImage(String name, String mime) {
        if (mime != null && mime.startsWith("image/")) {
            return true;
        }
        if (name == null) {
            return false;
        }
        String lower = name.toLowerCase(Locale.US);
        return lower.endsWith(".jpg")
                || lower.endsWith(".jpeg")
                || lower.endsWith(".png")
                || lower.endsWith(".gif")
                || lower.endsWith(".webp")
                || lower.endsWith(".heic")
                || lower.endsWith(".heif")
                || lower.endsWith(".bmp")
                || lower.endsWith(".tif")
                || lower.endsWith(".tiff");
    }

    private String md5(Uri uri) throws IOException {
        try (InputStream input = getContentResolver().openInputStream(uri)) {
            if (input == null) {
                throw new IOException("no stream for " + uri);
            }
            MessageDigest digest = MessageDigest.getInstance("MD5");
            byte[] buffer = new byte[1 << 20];
            int read;
            while ((read = input.read(buffer)) > 0) {
                digest.update(buffer, 0, read);
            }
            StringBuilder hex = new StringBuilder();
            for (byte value : digest.digest()) {
                hex.append(String.format(Locale.US, "%02x", value));
            }
            return hex.toString();
        } catch (NoSuchAlgorithmException error) {
            throw new IOException(error);
        }
    }

    private void applyDefaultKeep() {
        for (Grouping.Group group : groups) {
            keep.putIfAbsent(group.hash, group.kept(policy).id);
        }
    }

    private Grouping.Photo kept(Grouping.Group group) {
        String id = keep.get(group.hash);
        for (Grouping.Photo photo : group.photos) {
            if (photo.id.equals(id)) {
                return photo;
            }
        }
        return group.kept(policy);
    }

    private int targetCount() {
        int count = 0;
        for (Grouping.Group group : groups) {
            count += group.photos.size() - 1;
        }
        return count;
    }

    private long reclaimable() {
        long bytes = 0;
        for (Grouping.Group group : groups) {
            long total = 0;
            for (Grouping.Photo photo : group.photos) {
                total += photo.size;
            }
            bytes += total - kept(group).size;
        }
        return bytes;
    }

    private void refresh() {
        summary.setText(getString(
                R.string.summary,
                groups.size(),
                targetCount(),
                Grouping.bytes(reclaimable())));
        folder.setText(treeUri == null
                ? getString(R.string.no_folder)
                : getString(R.string.folder, treeUri.getLastPathSegment()));
        adapter.clear();
        for (Grouping.Group group : groups) {
            adapter.add(groupLabel(group));
        }
        adapter.notifyDataSetChanged();
        select.setEnabled(!busy);
        scan.setEnabled(!busy && treeUri != null);
        delete.setVisibility(targetCount() > 0 ? View.VISIBLE : View.GONE);
        delete.setText(getString(R.string.delete_n, targetCount()));
        delete.setEnabled(!busy && targetCount() > 0);
    }

    private String groupLabel(Grouping.Group group) {
        Grouping.Photo kept = kept(group);
        return getString(R.string.copies, group.photos.size())
                + "\n"
                + getString(R.string.details, Grouping.bytes(group.reclaimable()), date(kept.modified))
                + "\n"
                + getString(R.string.keep, kept.name);
    }

    private void showKeepDialog(Grouping.Group group) {
        Grouping.Photo current = kept(group);
        String[] labels = new String[group.photos.size()];
        for (int index = 0; index < group.photos.size(); index++) {
            Grouping.Photo photo = group.photos.get(index);
            labels[index] = photo.name
                    + "\n"
                    + Grouping.bytes(photo.size)
                    + " · "
                    + date(photo.modified)
                    + (photo.id.equals(current.id) ? "  ✓" : "");
        }
        new AlertDialog.Builder(this)
                .setTitle(R.string.keep_one)
                .setItems(labels, (dialog, which) -> {
                    keep.put(group.hash, group.photos.get(which).id);
                    refresh();
                })
                .setNegativeButton(R.string.cancel, null)
                .show();
    }

    private void confirmDelete() {
        List<Grouping.Photo> targets = new ArrayList<>();
        for (Grouping.Group group : groups) {
            Grouping.Photo kept = kept(group);
            for (Grouping.Photo photo : group.photos) {
                if (!photo.id.equals(kept.id)) {
                    targets.add(photo);
                }
            }
        }
        if (targets.isEmpty()) {
            return;
        }
        new AlertDialog.Builder(this)
                .setTitle(R.string.confirm_title)
                .setMessage(R.string.confirm_message)
                .setNegativeButton(R.string.cancel, null)
                .setPositiveButton(R.string.delete, (dialog, which) -> delete(targets))
                .show();
    }

    private void delete(List<Grouping.Photo> targets) {
        setBusy(true);
        executor.execute(() -> {
            int deleted = 0;
            int failed = 0;
            for (Grouping.Photo photo : targets) {
                Uri uri = uris.get(photo.id);
                try {
                    if (uri != null && DocumentsContract.deleteDocument(getContentResolver(), uri)) {
                        deleted++;
                    } else {
                        failed++;
                    }
                } catch (Exception error) {
                    failed++;
                }
            }
            final int removed = deleted;
            final int failures = failed;
            main.post(() -> {
                Set<String> gone = new HashSet<>();
                for (Grouping.Photo photo : targets) {
                    gone.add(photo.id);
                }
                for (Grouping.Group group : groups) {
                    group.photos.removeIf(photo -> gone.contains(photo.id));
                }
                groups.removeIf(group -> group.photos.size() < 2);
                applyDefaultKeep();
                setBusy(false);
                refresh();
                status.setText(getString(R.string.deleted, removed)
                        + (failures > 0 ? " " + getString(R.string.failed, failures) : ""));
            });
        });
    }

    private void setBusy(boolean value) {
        busy = value;
        refresh();
    }

    private String date(long millis) {
        if (millis <= 0) {
            return "—";
        }
        return new SimpleDateFormat("yyyy-MM-dd", Locale.US).format(new Date(millis));
    }

    private static final class Document {
        final Uri uri;
        final String name;
        final long size;
        final long modified;

        Document(Uri uri, String name, long size, long modified) {
            this.uri = uri;
            this.name = name;
            this.size = size;
            this.modified = modified;
        }
    }
}
