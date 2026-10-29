package appmodel

import (
	"testing"

	"onley/internal/db"
)

func group(md5 string, size int64, paths ...string) db.DuplicateGroup {
	g := db.DuplicateGroup{MD5: md5, Size: size}
	for _, path := range paths {
		g.Files = append(g.Files, db.FileRecord{Path: path, Size: size, MD5: md5})
	}
	return g
}

func TestSummarize(t *testing.T) {
	groups := []db.DuplicateGroup{
		group("a", 100, "/a/1", "/a/2", "/a/3"),
		group("b", 50, "/b/1", "/b/2"),
	}
	got := Summarize(groups)
	if got.Groups != 2 || got.DuplicateFiles != 5 || got.Wasted != 250 {
		t.Fatalf("Summarize = %+v", got)
	}
}

func TestBuildPlanKeepsOnePerGroup(t *testing.T) {
	groups := []db.DuplicateGroup{
		group("a", 100, "/a/1", "/a/2", "/a/3"),
		group("b", 50, "/b/1", "/b/2"),
	}
	plan := BuildPlan(groups, map[string]string{"a": "/a/2", "b": "/b/1"}, "")
	if plan.Groups != 2 {
		t.Errorf("groups = %d, want 2", plan.Groups)
	}
	if len(plan.Keeps) != 2 {
		t.Errorf("keeps = %+v", plan.Keeps)
	}
	if len(plan.Targets) != 3 {
		t.Fatalf("targets = %+v", plan.Targets)
	}
	if plan.Bytes != 250 {
		t.Errorf("bytes = %d, want 250", plan.Bytes)
	}
	for _, target := range plan.Targets {
		if target.Path == "/a/2" || target.Path == "/b/1" {
			t.Errorf("kept file targeted: %s", target.Path)
		}
	}
}

func TestBuildPlanSkipsGroupsWithoutChoice(t *testing.T) {
	groups := []db.DuplicateGroup{
		group("a", 100, "/a/1", "/a/2"),
		group("b", 50, "/b/1", "/b/2"),
	}
	plan := BuildPlan(groups, map[string]string{"a": "/a/1"}, "")
	if plan.Groups != 1 || len(plan.Keeps) != 1 || len(plan.Targets) != 1 || plan.Targets[0].Path != "/a/2" {
		t.Fatalf("plan = %+v", plan)
	}
}

func TestBuildPlanIgnoresUnknownKeepPath(t *testing.T) {
	groups := []db.DuplicateGroup{group("a", 100, "/a/1", "/a/2")}
	plan := BuildPlan(groups, map[string]string{"a": "/elsewhere"}, "")
	if plan.Groups != 0 || len(plan.Targets) != 0 {
		t.Fatalf("plan = %+v", plan)
	}
}

func TestBuildPlanTargetsEachPathOnce(t *testing.T) {
	groups := []db.DuplicateGroup{
		group("a", 100, "/same", "/a/2"),
		group("b", 100, "/same", "/b/2"),
	}
	plan := BuildPlan(groups, map[string]string{"a": "/a/2", "b": "/b/2"}, "")
	if len(plan.Targets) != 1 || plan.Targets[0].Path != "/same" {
		t.Fatalf("targets = %+v", plan.Targets)
	}
}

func TestBuildPlanProtectsRunningExecutable(t *testing.T) {
	groups := []db.DuplicateGroup{group("a", 100, "/onley.exe", "/copy.exe")}
	plan := BuildPlan(groups, map[string]string{"a": "/onley.exe"}, "/onley.exe")
	if plan.Groups != 1 || len(plan.Keeps) != 1 || plan.Keeps[0].Path != "/onley.exe" {
		t.Fatalf("plan = %+v", plan)
	}
	if len(plan.Targets) != 1 || plan.Targets[0].Path != "/copy.exe" {
		t.Fatalf("targets = %+v", plan.Targets)
	}
}

func TestBuildPlanSkipsGroupWhenRunningExecutableIsNotKept(t *testing.T) {
	groups := []db.DuplicateGroup{group("a", 100, "/onley.exe", "/copy.exe")}
	plan := BuildPlan(groups, map[string]string{"a": "/copy.exe"}, "/onley.exe")
	if plan.Groups != 0 || len(plan.Targets) != 0 {
		t.Fatalf("plan = %+v", plan)
	}
}
