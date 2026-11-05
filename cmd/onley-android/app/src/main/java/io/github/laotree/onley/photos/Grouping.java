package io.github.laotree.onley.photos;

import java.util.ArrayList;
import java.util.Collections;
import java.util.Comparator;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/** Duplicate groups and keep policies, without Android. */
public final class Grouping {
    private Grouping() {}

    public enum Policy {
        NEWEST,
        OLDEST
    }

    /** One photo, after hashing. */
    public static final class Photo {
        public final String id;
        public final String name;
        public final long size;
        public final long modified;
        public final String hash;

        public Photo(String id, String name, long size, long modified, String hash) {
            this.id = id;
            this.name = name;
            this.size = size;
            this.modified = modified;
            this.hash = hash;
        }
    }

    /** Copies of one photo. */
    public static final class Group {
        public final String hash;
        public final List<Photo> photos;

        Group(String hash, List<Photo> photos) {
            this.hash = hash;
            this.photos = photos;
        }

        /** The copy the policy keeps. */
        public Photo kept(Policy policy) {
            Comparator<Photo> order = Comparator
                    .comparingLong((Photo photo) -> photo.modified)
                    .thenComparing((Photo photo) -> photo.name)
                    .thenComparing((Photo photo) -> photo.id);
            return Collections.max(photos, policy == Policy.NEWEST ? order : order.reversed());
        }

        /** Bytes freed when every copy but the kept one goes. */
        public long reclaimable() {
            long total = 0;
            long smallest = Long.MAX_VALUE;
            for (Photo photo : photos) {
                total += photo.size;
                smallest = Math.min(smallest, photo.size);
            }
            return total - smallest;
        }
    }

    /** Groups the photos by hash, keeps groups with more than one copy, and orders them by reclaimable space. */
    public static List<Group> duplicates(List<Photo> photos) {
        Map<String, List<Photo>> byHash = new HashMap<>();
        for (Photo photo : photos) {
            byHash.computeIfAbsent(photo.hash, key -> new ArrayList<>()).add(photo);
        }
        List<Group> groups = new ArrayList<>();
        for (Map.Entry<String, List<Photo>> entry : byHash.entrySet()) {
            if (entry.getValue().size() < 2) {
                continue;
            }
            List<Photo> copies = entry.getValue();
            copies.sort(Comparator
                    .comparingLong((Photo photo) -> photo.modified)
                    .thenComparing((Photo photo) -> photo.name)
                    .thenComparing((Photo photo) -> photo.id)
                    .reversed());
            groups.add(new Group(entry.getKey(), copies));
        }
        groups.sort(Comparator
                .comparingLong(Group::reclaimable)
                .reversed()
                .thenComparing(group -> group.hash));
        return groups;
    }

    /** Formats a byte count for a small screen. */
    public static String bytes(long value) {
        String[] units = {"B", "KB", "MB", "GB", "TB"};
        double size = Math.max(0, value);
        int unit = 0;
        while (size >= 1024 && unit + 1 < units.length) {
            size /= 1024;
            unit++;
        }
        if (unit == 0) {
            return value + " B";
        }
        return String.format(java.util.Locale.US, "%.1f %s", size, units[unit]);
    }
}
