package appmodel

import (
	"path/filepath"
	"reflect"
	"testing"
)

func TestMergeRoots(t *testing.T) {
	base := t.TempDir()
	parent := filepath.Join(base, "data")
	child := filepath.Join(parent, "photos")
	sibling := filepath.Join(base, "other")

	tests := []struct {
		name  string
		roots []string
		want  []string
	}{
		{"empty", nil, nil},
		{"duplicates", []string{parent, parent}, []string{parent}},
		{"child added after parent", []string{parent, child}, []string{parent}},
		{"parent added after child", []string{child, parent}, []string{parent}},
		{"siblings stay", []string{sibling, parent}, []string{parent, sibling}},
		{"nested and sibling", []string{child, sibling, parent}, []string{parent, sibling}},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := MergeRoots(tt.roots); !reflect.DeepEqual(got, tt.want) {
				t.Errorf("MergeRoots(%v) = %v, want %v", tt.roots, got, tt.want)
			}
		})
	}
}
