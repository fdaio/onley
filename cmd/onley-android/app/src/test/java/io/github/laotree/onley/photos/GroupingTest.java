package io.github.laotree.onley.photos;

import static org.junit.Assert.assertEquals;

import java.util.Arrays;
import java.util.List;
import org.junit.Test;

public final class GroupingTest {
    private static Grouping.Photo photo(String id, String hash, long size, long modified) {
        return new Grouping.Photo(id, id + ".jpg", size, modified, hash);
    }

    @Test
    public void keepsOnlyGroupsWithMoreThanOneCopy() {
        List<Grouping.Group> groups = Grouping.duplicates(Arrays.asList(
                photo("a", "same", 10, 1),
                photo("b", "same", 10, 2),
                photo("c", "unique", 20, 3)));
        assertEquals(1, groups.size());
        assertEquals(2, groups.get(0).photos.size());
    }

    @Test
    public void newestAndOldestPickTheRightCopy() {
        Grouping.Group group = Grouping.duplicates(Arrays.asList(
                photo("old", "same", 10, 1),
                photo("new", "same", 10, 2))).get(0);
        assertEquals("new", group.kept(Grouping.Policy.NEWEST).id);
        assertEquals("old", group.kept(Grouping.Policy.OLDEST).id);
    }

    @Test
    public void reclaimableExcludesTheKeptCopy() {
        Grouping.Group group = Grouping.duplicates(Arrays.asList(
                photo("a", "same", 10, 1),
                photo("b", "same", 30, 2))).get(0);
        assertEquals(30, group.reclaimable());
    }

    @Test
    public void groupsAreOrderedByReclaimableSpace() {
        List<Grouping.Group> groups = Grouping.duplicates(Arrays.asList(
                photo("a", "small", 10, 1),
                photo("b", "small", 10, 2),
                photo("c", "large", 100, 1),
                photo("d", "large", 100, 2)));
        assertEquals("large", groups.get(0).hash);
        assertEquals("small", groups.get(1).hash);
    }

    @Test
    public void formatsBytes() {
        assertEquals("512 B", Grouping.bytes(512));
        assertEquals("1.0 KB", Grouping.bytes(1024));
        assertEquals("1.5 MB", Grouping.bytes(1024 * 1024 * 3 / 2));
    }
}
