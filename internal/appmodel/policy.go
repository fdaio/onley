package appmodel

import "onley/internal/db"

// KeepPolicy chooses which copy of a duplicate group to keep.
type KeepPolicy uint8

const (
	KeepNewest KeepPolicy = iota
	KeepOldest
)

// String returns the storage value for p.
func (p KeepPolicy) String() string {
	if p == KeepOldest {
		return "oldest"
	}
	return "newest"
}

// ParseKeepPolicy parses a stored policy, falling back to newest.
func ParseKeepPolicy(s string) (KeepPolicy, bool) {
	switch s {
	case "newest":
		return KeepNewest, true
	case "oldest":
		return KeepOldest, true
	default:
		return KeepNewest, false
	}
}

// ChooseKeep returns the copy p keeps from group. It compares modification
// time and uses the path as a stable tie-breaker.
func ChooseKeep(group db.DuplicateGroup, p KeepPolicy) db.FileRecord {
	if len(group.Files) == 0 {
		return db.FileRecord{}
	}
	chosen := group.Files[0]
	for _, file := range group.Files[1:] {
		var better bool
		if p == KeepOldest {
			better = file.Mtime < chosen.Mtime || (file.Mtime == chosen.Mtime && file.Path < chosen.Path)
		} else {
			better = file.Mtime > chosen.Mtime || (file.Mtime == chosen.Mtime && file.Path < chosen.Path)
		}
		if better {
			chosen = file
		}
	}
	return chosen
}
