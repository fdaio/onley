package appmodel

import (
	"testing"

	"onley/internal/db"
)

func policyGroup() db.DuplicateGroup {
	return db.DuplicateGroup{MD5: "a", Size: 10, Files: []db.FileRecord{
		{Path: "/z-old", Mtime: 100},
		{Path: "/a-new", Mtime: 300},
		{Path: "/m-mid", Mtime: 200},
	}}
}

func TestChooseKeepNewest(t *testing.T) {
	got := ChooseKeep(policyGroup(), KeepNewest)
	if got.Path != "/a-new" {
		t.Errorf("newest = %q, want /a-new", got.Path)
	}
}

func TestChooseKeepOldest(t *testing.T) {
	got := ChooseKeep(policyGroup(), KeepOldest)
	if got.Path != "/z-old" {
		t.Errorf("oldest = %q, want /z-old", got.Path)
	}
}

func TestChooseKeepByPath(t *testing.T) {
	got := ChooseKeep(policyGroup(), KeepByPath)
	if got.Path != "/a-new" {
		t.Errorf("by path = %q, want /a-new", got.Path)
	}
}

func TestChooseKeepTieUsesPath(t *testing.T) {
	group := db.DuplicateGroup{MD5: "a", Files: []db.FileRecord{
		{Path: "/b", Mtime: 100},
		{Path: "/a", Mtime: 100},
	}}
	if got := ChooseKeep(group, KeepNewest); got.Path != "/a" {
		t.Errorf("newest tie = %q, want /a", got.Path)
	}
	if got := ChooseKeep(group, KeepOldest); got.Path != "/a" {
		t.Errorf("oldest tie = %q, want /a", got.Path)
	}
}

func TestParseKeepPolicy(t *testing.T) {
	for _, policy := range []KeepPolicy{KeepNewest, KeepOldest, KeepByPath} {
		got, ok := ParseKeepPolicy(policy.String())
		if !ok || got != policy {
			t.Errorf("ParseKeepPolicy(%q) = %v, %v", policy.String(), got, ok)
		}
	}
	if got, ok := ParseKeepPolicy("other"); ok || got != KeepNewest {
		t.Errorf("ParseKeepPolicy(other) = %v, %v", got, ok)
	}
}
