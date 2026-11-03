package cleaner

import (
	"os"
	"path/filepath"
	"strings"
)

// RemoveEmptyParents removes the directories that deleting the given files
// left empty. It walks up from each file's directory and stops at the scan
// root that contains it, so a root is never removed.
//
// Only an empty directory goes: os.Remove fails on any other, so a folder that
// still holds a file, or that was empty before the cleanup and is not on the
// walk, stays. A symlink is never followed, because a linked folder could sit
// outside the root the user chose.
func RemoveEmptyParents(deleted []string, roots []string) []string {
	cleanRoots := make([]string, 0, len(roots))
	for _, root := range roots {
		if root != "" {
			cleanRoots = append(cleanRoots, filepath.Clean(root))
		}
	}
	if len(cleanRoots) == 0 {
		return nil
	}

	seen := make(map[string]bool, len(deleted))
	dirs := make([]string, 0, len(deleted))
	for _, path := range deleted {
		if path == "" {
			continue
		}
		dir := filepath.Dir(filepath.Clean(path))
		if !seen[dir] {
			seen[dir] = true
			dirs = append(dirs, dir)
		}
	}

	var removed []string
	for _, dir := range dirs {
		for {
			root, ok := containingRoot(dir, cleanRoots)
			if !ok {
				break
			}
			if !plainDir(dir, root) {
				break
			}
			if err := os.Remove(dir); err != nil {
				break
			}
			removed = append(removed, dir)
			dir = filepath.Dir(dir)
		}
	}
	return removed
}

// containingRoot returns the root that holds dir, apart from dir itself.
func containingRoot(dir string, roots []string) (string, bool) {
	for _, root := range roots {
		if within(root, dir) {
			return root, true
		}
	}
	return "", false
}

// within reports whether path is strictly inside root, by the paths as given.
func within(root, path string) bool {
	rel, err := filepath.Rel(root, path)
	if err != nil {
		return false
	}
	return rel != "." && rel != ".." && !strings.HasPrefix(rel, ".."+string(filepath.Separator))
}

// plainDir reports whether dir is a real directory reached from root without a
// symlink on the way. A link would name a folder the user did not choose.
func plainDir(dir, root string) bool {
	info, err := os.Lstat(dir)
	if err != nil || !info.IsDir() {
		return false
	}
	rel, err := filepath.Rel(root, dir)
	if err != nil {
		return false
	}
	resolvedRoot, err := filepath.EvalSymlinks(root)
	if err != nil {
		return false
	}
	resolvedDir, err := filepath.EvalSymlinks(dir)
	if err != nil {
		return false
	}
	return resolvedDir == filepath.Join(resolvedRoot, rel)
}
