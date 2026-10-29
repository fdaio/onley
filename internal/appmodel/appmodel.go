// Package appmodel holds the desktop client's decisions without a UI.
package appmodel

import (
	"onley/internal/db"
	"onley/internal/selfpath"
)

// Summary describes the duplicate index.
type Summary struct {
	Groups         int
	DuplicateFiles int
	Wasted         int64
}

// Summarize counts duplicate groups, files and the space that deleting all but
// one copy per group would reclaim.
func Summarize(groups []db.DuplicateGroup) Summary {
	var s Summary
	s.Groups = len(groups)
	for _, group := range groups {
		s.DuplicateFiles += len(group.Files)
		if len(group.Files) > 1 {
			s.Wasted += group.Size * int64(len(group.Files)-1)
		}
	}
	return s
}

// Plan is the files a delete would remove and the copies it would keep.
type Plan struct {
	Targets []db.FileRecord
	Keeps   []db.FileRecord
	Bytes   int64
	Groups  int
}

// ProtectedFile returns the file in group that matches protect.
func ProtectedFile(group db.DuplicateGroup, protect string) (db.FileRecord, bool) {
	if protect == "" {
		return db.FileRecord{}, false
	}
	for _, file := range group.Files {
		if selfpath.Same(file.Path, protect) {
			return file, true
		}
	}
	return db.FileRecord{}, false
}

// BuildPlan keeps keep[group.MD5] in each group and targets every other file.
// A group with no valid keep choice is skipped. A group that holds protect
// must keep protect; otherwise it is skipped, because deleting the running
// executable cannot succeed. Each path is targeted once.
func BuildPlan(groups []db.DuplicateGroup, keep map[string]string, protect string) Plan {
	var plan Plan
	seen := make(map[string]bool)
	for _, group := range groups {
		keepPath, ok := keep[group.MD5]
		if !ok {
			continue
		}
		if self, found := ProtectedFile(group, protect); found && !selfpath.Same(self.Path, keepPath) {
			continue
		}
		found := false
		for _, file := range group.Files {
			if file.Path == keepPath {
				found = true
				break
			}
		}
		if !found {
			continue
		}
		plan.Groups++
		for _, file := range group.Files {
			if file.Path == keepPath {
				plan.Keeps = append(plan.Keeps, file)
				continue
			}
			if seen[file.Path] {
				continue
			}
			seen[file.Path] = true
			plan.Targets = append(plan.Targets, file)
			plan.Bytes += file.Size
		}
	}
	return plan
}
