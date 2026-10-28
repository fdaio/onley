package appmodel

import (
	"path/filepath"
	"sort"
	"strings"
)

// MergeRoots cleans roots and removes any root covered by another. A parent
// keeps its children out of the scan; equal paths collapse. The result is
// sorted so the scan order is stable.
func MergeRoots(roots []string) []string {
	var out []string
	for _, root := range roots {
		if root == "" {
			continue
		}
		root = filepath.Clean(root)
		covered := false
		for _, existing := range out {
			if within(existing, root) {
				covered = true
				break
			}
		}
		if covered {
			continue
		}
		kept := out[:0]
		for _, existing := range out {
			if !within(root, existing) {
				kept = append(kept, existing)
			}
		}
		out = append(kept, root)
	}
	sort.Strings(out)
	return out
}

// within reports whether path is parent or a descendant of parent.
func within(parent, path string) bool {
	rel, err := filepath.Rel(parent, path)
	if err != nil {
		return false
	}
	return rel == "." || (rel != ".." && !strings.HasPrefix(rel, ".."+string(filepath.Separator)))
}
