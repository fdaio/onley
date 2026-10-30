package main

import (
	"fmt"
	"path/filepath"
	"strings"

	"onley/internal/db"
	"onley/internal/i18n"
)

type category uint8

const (
	categoryAll category = iota
	categoryImages
	categoryVideos
	categoryDocuments
	categoryArchives
	categorySystem
	categorySearch
)

var imageExtensions = map[string]bool{
	".avif": true, ".bmp": true, ".cr2": true, ".dng": true, ".gif": true,
	".heic": true, ".jpeg": true, ".jpg": true, ".nef": true, ".png": true,
	".raw": true, ".svg": true, ".tif": true, ".tiff": true, ".webp": true,
}

var videoExtensions = map[string]bool{
	".avi": true, ".flv": true, ".m4v": true, ".mkv": true, ".mov": true,
	".mp4": true, ".mpeg": true, ".mpg": true, ".webm": true, ".wmv": true,
}

var documentExtensions = map[string]bool{
	".csv": true, ".doc": true, ".docx": true, ".key": true, ".md": true,
	".numbers": true, ".pages": true, ".pdf": true, ".ppt": true, ".pptx": true,
	".rtf": true, ".txt": true, ".xls": true, ".xlsx": true,
}

var archiveExtensions = map[string]bool{
	".7z": true, ".bz2": true, ".dmg": true, ".gz": true, ".iso": true,
	".rar": true, ".tar": true, ".tgz": true, ".xz": true, ".zip": true,
}

func fileCategory(name string) category {
	ext := strings.ToLower(filepath.Ext(name))
	switch {
	case imageExtensions[ext]:
		return categoryImages
	case videoExtensions[ext]:
		return categoryVideos
	case documentExtensions[ext]:
		return categoryDocuments
	case archiveExtensions[ext]:
		return categoryArchives
	default:
		return categorySystem
	}
}

func (c category) keyName() string {
	switch c {
	case categoryImages:
		return "images"
	case categoryVideos:
		return "videos"
	case categoryDocuments:
		return "documents"
	case categoryArchives:
		return "archives"
	case categorySystem:
		return "system"
	case categorySearch:
		return "search"
	default:
		return "all"
	}
}

func (c category) labelKey() i18n.Key {
	switch c {
	case categoryImages:
		return i18n.KeyImages
	case categoryVideos:
		return i18n.KeyVideos
	case categoryDocuments:
		return i18n.KeyDocuments
	case categoryArchives:
		return i18n.KeyArchives
	case categorySystem:
		return i18n.KeySystem
	case categorySearch:
		return i18n.KeySearch
	default:
		return i18n.KeyAllCopies
	}
}

// categoryCounts counts duplicate groups per category.
func (s *state) categoryCounts() map[category]int {
	counts := map[category]int{}
	roots := s.scanDirs.Get()
	for _, group := range s.groups.Get() {
		if len(group.Files) == 0 || !groupInRoots(group, roots) {
			continue
		}
		counts[categoryAll]++
		counts[fileCategory(group.Files[0].Name)]++
	}
	return counts
}

func groupInRoots(group db.DuplicateGroup, roots []string) bool {
	for _, file := range group.Files {
		for _, root := range roots {
			if file.Path == root || strings.HasPrefix(file.Path, root+string(filepath.Separator)) {
				return true
			}
		}
	}
	return false
}

func groupMatches(group db.DuplicateGroup, query string) bool {
	for _, file := range group.Files {
		if strings.Contains(strings.ToLower(file.Name), query) ||
			strings.Contains(strings.ToLower(file.Path), query) {
			return true
		}
	}
	return false
}

// formatCount groups digits with commas for the large header numbers.
func formatCount(n int) string {
	digits := fmt.Sprintf("%d", n)
	if len(digits) <= 3 {
		return digits
	}
	var b strings.Builder
	first := len(digits) % 3
	if first > 0 {
		b.WriteString(digits[:first])
	}
	for i := first; i < len(digits); i += 3 {
		if b.Len() > 0 {
			b.WriteByte(',')
		}
		b.WriteString(digits[i : i+3])
	}
	return b.String()
}
