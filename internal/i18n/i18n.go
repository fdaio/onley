// Package i18n holds the desktop client's translated text.
package i18n

import (
	"fmt"
	"strings"
)

// Lang is a supported UI language.
type Lang string

const (
	English Lang = "en"
	Chinese Lang = "zh"
)

var order = []Lang{English, Chinese}

// Languages returns the supported languages in selector order.
func Languages() []Lang {
	return append([]Lang(nil), order...)
}

// Detect picks a language from a system locale such as zh-Hans or en-US.
func Detect(locale string) Lang {
	if strings.HasPrefix(strings.ToLower(locale), "zh") {
		return Chinese
	}
	return English
}

// Parse returns the language named by s.
func Parse(s string) (Lang, bool) {
	for _, lang := range order {
		if string(lang) == s {
			return lang, true
		}
	}
	return English, false
}

// Next returns the language after lang in selector order.
func Next(lang Lang) Lang {
	for i, candidate := range order {
		if candidate == lang {
			return order[(i+1)%len(order)]
		}
	}
	return order[0]
}

// Key names one UI message.
type Key string

const (
	KeySubtitle             Key = "subtitle"
	KeyPreview              Key = "preview"
	KeyLocalOnly            Key = "localOnly"
	KeyDismiss              Key = "dismiss"
	KeySwitchLanguage       Key = "switchLanguage"
	KeyIndex                Key = "index"
	KeyIndexedFiles         Key = "indexedFiles"
	KeyDuplicateFiles       Key = "duplicateFiles"
	KeyGroups               Key = "groups"
	KeyReclaimable          Key = "reclaimable"
	KeyScanFolders          Key = "scanFolders"
	KeySharedIndex          Key = "sharedIndex"
	KeyAddFolder            Key = "addFolder"
	KeyAdd                  Key = "add"
	KeyRemove               Key = "remove"
	KeyMergeHint            Key = "mergeHint"
	KeyNoFolders            Key = "noFolders"
	KeyScanAll              Key = "scanAll"
	KeyDropFolders          Key = "dropFolders"
	KeyDropHint             Key = "dropHint"
	KeyCleanup              Key = "cleanup"
	KeyRefreshIndex         Key = "refreshIndex"
	KeyClearChoices         Key = "clearChoices"
	KeyIndexRefreshed       Key = "indexRefreshed"
	KeyIndexFile            Key = "indexFile"
	KeyNotAvailable         Key = "notAvailable"
	KeyScanning             Key = "scanning"
	KeyScanSpeedGauge       Key = "scanSpeedGauge"
	KeyScanStats            Key = "scanStats"
	KeyElapsed              Key = "elapsed"
	KeyCancelScan           Key = "cancelScan"
	KeyCancelScanHint       Key = "cancelScanHint"
	KeyScanCanceled         Key = "scanCanceled"
	KeyScanFinished         Key = "scanFinished"
	KeyReadErrors           Key = "readErrors"
	KeyCanceling            Key = "canceling"
	KeyAddFolderFirst       Key = "addFolderFirst"
	KeyCannotResolvePath    Key = "cannotResolvePath"
	KeyCannotScanFolder     Key = "cannotScanFolder"
	KeyNotAFolder           Key = "notAFolder"
	KeyCannotOpenIndex      Key = "cannotOpenIndex"
	KeyIndexUnavailable     Key = "indexUnavailable"
	KeyCannotReadIndex      Key = "cannotReadIndex"
	KeyCannotKeep           Key = "cannotKeep"
	KeyDeleteCanceledKeep   Key = "deleteCanceledKeep"
	KeyDeleted              Key = "deleted"
	KeyAlreadyGone          Key = "alreadyGone"
	KeyFailed               Key = "failed"
	KeyIndexCleanupFailed   Key = "indexCleanupFailed"
	KeyDropFolderFirst      Key = "dropFolderFirst"
	KeyDuplicateGroups      Key = "duplicateGroups"
	KeyNoDuplicates         Key = "noDuplicates"
	KeyEmptyBody            Key = "emptyBody"
	KeyCloseIndexHolder     Key = "closeIndexHolder"
	KeyScanningDots         Key = "scanningDots"
	KeyScanningBody         Key = "scanningBody"
	KeyScanDefaultFolder    Key = "scanDefaultFolder"
	KeyChooseCopyOrKeep     Key = "chooseCopyOrKeep"
	KeySelection            Key = "selection"
	KeyPermanentHint        Key = "permanentHint"
	KeyKeepPolicy           Key = "keepPolicy"
	KeyNewest               Key = "newest"
	KeyOldest               Key = "oldest"
	KeyPolicyHint           Key = "policyHint"
	KeyKeepNewestPerGroup   Key = "keepNewestPerGroup"
	KeyKeepOldestPerGroup   Key = "keepOldestPerGroup"
	KeyKeepOne              Key = "keepOne"
	KeyReviewDelete         Key = "reviewDelete"
	KeyConfirmDelete        Key = "confirmDelete"
	KeyDeleteDuplicates     Key = "deleteDuplicates"
	KeyDeleteConfirm        Key = "deleteConfirm"
	KeyCannotUndo           Key = "cannotUndo"
	KeyAndMore              Key = "andMore"
	KeyCancel               Key = "cancel"
	KeyDeletePermanently    Key = "deletePermanently"
	KeyGroupWord            Key = "groupWord"
	KeyGroup                Key = "group"
	KeyDuplicateGroupTitle  Key = "duplicateGroupTitle"
	KeyDuplicateGroupsTitle Key = "duplicateGroupsTitle"
	KeyDuplicateSummary     Key = "duplicateSummary"
	KeyAutoKeep             Key = "autoKeep"
	KeyRescan               Key = "rescan"
	KeyDelete               Key = "delete"
	KeyDeleteCount          Key = "deleteCount"
	KeyKeepHint             Key = "keepHint"
	KeySelfKeepHint         Key = "selfKeepHint"
	KeyKeepBadge            Key = "keepBadge"
	KeyRunning              Key = "running"
	KeyDeleteBadge          Key = "deleteBadge"
	KeyCannotDeleteSelf     Key = "cannotDeleteSelf"
	KeyFileMenu             Key = "fileMenu"
	KeyEditMenu             Key = "editMenu"

	KeyReclaimableSpace  Key = "reclaimableSpace"
	KeyAllCopies         Key = "allCopies"
	KeyImages            Key = "images"
	KeyVideos            Key = "videos"
	KeyDocuments         Key = "documents"
	KeyArchives          Key = "archives"
	KeySystem            Key = "system"
	KeySearch            Key = "search"
	KeySourceFolders     Key = "sourceFolders"
	KeyAllFolders        Key = "allFolders"
	KeyDropDirectory     Key = "dropDirectory"
	KeyInitializeScan    Key = "initializeScan"
	KeyRunGlobalScan     Key = "runGlobalScan"
	KeyAutoSelectPolicy  Key = "autoSelectPolicy"
	KeyKeepNewest        Key = "keepNewest"
	KeyKeepOldest        Key = "keepOldest"
	KeyKeepByPath        Key = "keepByPath"
	KeyClear             Key = "clear"
	KeyOriginalCopy      Key = "originalCopy"
	KeyDuplicateN        Key = "duplicateN"
	KeyReviewDeleteN     Key = "reviewDeleteN"
	KeySearchPlaceholder Key = "searchPlaceholder"
	KeyLocalIndex        Key = "localIndex"
)

var messages = map[Lang]map[Key]string{
	English: {
		KeySubtitle:             "Find and remove duplicate files",
		KeyPreview:              "Preview",
		KeyLocalOnly:            "Local only",
		KeyDismiss:              "Dismiss",
		KeySwitchLanguage:       "Switch language",
		KeyIndex:                "Index",
		KeyIndexedFiles:         "Indexed files",
		KeyDuplicateFiles:       "Duplicate files",
		KeyGroups:               "Groups",
		KeyReclaimable:          "Reclaimable",
		KeyScanFolders:          "Scan folders",
		KeySharedIndex:          "The index is shared with the onley CLI.",
		KeyAddFolder:            "Add a folder",
		KeyAdd:                  "Add",
		KeyRemove:               "Remove",
		KeyMergeHint:            "Child folders merge into their parent.",
		KeyNoFolders:            "no folders",
		KeyScanAll:              "Scan all",
		KeyDropFolders:          "Drop folders here",
		KeyDropHint:             "they are added and merged before scanning",
		KeyCleanup:              "Cleanup",
		KeyRefreshIndex:         "Refresh index",
		KeyClearChoices:         "Clear choices",
		KeyIndexRefreshed:       "Index refreshed.",
		KeyIndexFile:            "Index file",
		KeyNotAvailable:         "not available",
		KeyScanning:             "Scanning",
		KeyScanSpeedGauge:       "Scan speed gauge",
		KeyScanStats:            "%s hashed · %s · %d skipped · %d errors",
		KeyElapsed:              "elapsed %s",
		KeyCancelScan:           "Cancel scan",
		KeyCancelScanHint:       "Finished files are saved. Scan again to resume.",
		KeyScanCanceled:         "Scan canceled. The partial index is kept.",
		KeyScanFinished:         "Scan finished: %d files processed, %d unchanged skipped.",
		KeyReadErrors:           "%d file(s) could not be read",
		KeyCanceling:            "Canceling the scan…",
		KeyAddFolderFirst:       "Add a folder to scan.",
		KeyCannotResolvePath:    "Cannot resolve the scan path: %v",
		KeyCannotScanFolder:     "Cannot scan that folder: %v",
		KeyNotAFolder:           "The scan path is not a folder: %s",
		KeyCannotOpenIndex:      "Cannot open the index: %v",
		KeyIndexUnavailable:     "The index is not available.",
		KeyCannotReadIndex:      "Cannot read the index: %v",
		KeyCannotKeep:           "Cannot keep %s: %v. Review the files again.",
		KeyDeleteCanceledKeep:   "Delete canceled: the copy to keep is no longer available.",
		KeyDeleted:              "Deleted %s, freed %s.",
		KeyAlreadyGone:          "%d was already gone.",
		KeyFailed:               "%d failed.",
		KeyIndexCleanupFailed:   "index cleanup failed",
		KeyDropFolderFirst:      "Drop a folder to scan it.",
		KeyDuplicateGroups:      "Duplicate groups",
		KeyNoDuplicates:         "No duplicates yet",
		KeyEmptyBody:            "Scan a folder to build the index. Only files in the index can be deleted.",
		KeyCloseIndexHolder:     "Close any process that holds the index, then reopen Onley.",
		KeyScanningDots:         "Scanning…",
		KeyScanningBody:         "Duplicate groups appear when the scan finishes.",
		KeyScanDefaultFolder:    "Scan default folder",
		KeyChooseCopyOrKeep:     "Choose a copy, or use the keep policy.",
		KeySelection:            "%s selected · %s reclaimable",
		KeyPermanentHint:        "Permanent deletion. You will confirm the exact files next.",
		KeyKeepPolicy:           "Keep policy",
		KeyNewest:               "Newest",
		KeyOldest:               "Oldest",
		KeyPolicyHint:           "Uses modification time; ties use path.",
		KeyKeepNewestPerGroup:   "Keep newest per group",
		KeyKeepOldestPerGroup:   "Keep oldest per group",
		KeyKeepOne:              "Keep one",
		KeyReviewDelete:         "Review & delete",
		KeyConfirmDelete:        "Confirm delete",
		KeyDeleteDuplicates:     "Delete duplicates?",
		KeyDeleteConfirm:        "Delete %s and free %s?",
		KeyCannotUndo:           "This cannot be undone. Files changed since the scan are skipped.",
		KeyAndMore:              "and %d more",
		KeyCancel:               "Cancel",
		KeyDeletePermanently:    "Delete permanently",
		KeyGroupWord:            "Group",
		KeyGroup:                "Group %d",
		KeyDuplicateGroupTitle:  "%d duplicate group",
		KeyDuplicateGroupsTitle: "%d duplicate groups",
		KeyDuplicateSummary:     "%s · %s reclaimable",
		KeyAutoKeep:             "Auto keep",
		KeyRescan:               "Rescan",
		KeyDelete:               "Delete",
		KeyDeleteCount:          "Delete %s",
		KeyKeepHint:             "Choose the copy to keep. A group without a choice is skipped.",
		KeySelfKeepHint:         "Onley keeps its running executable. Select it to delete the other copies.",
		KeyKeepBadge:            "keep",
		KeyRunning:              "running",
		KeyDeleteBadge:          "delete",
		KeyCannotDeleteSelf:     "Onley cannot delete its own running executable.",
		KeyFileMenu:             "File",
		KeyEditMenu:             "Edit",
		KeyReclaimableSpace:     "Reclaimable space",
		KeyAllCopies:            "All copies",
		KeyImages:               "Images",
		KeyVideos:               "Videos",
		KeyDocuments:            "Documents",
		KeyArchives:             "Archives",
		KeySystem:               "System",
		KeySearch:               "Search",
		KeySourceFolders:        "Source folders",
		KeyAllFolders:           "All folders",
		KeyDropDirectory:        "Drop directory here",
		KeyInitializeScan:       "Initialize scan sequence",
		KeyRunGlobalScan:        "Run global scan",
		KeyAutoSelectPolicy:     "Auto-select policy:",
		KeyKeepNewest:           "Keep newest",
		KeyKeepOldest:           "Keep oldest",
		KeyKeepByPath:           "Keep by path",
		KeyClear:                "Clear",
		KeyOriginalCopy:         "Original copy",
		KeyDuplicateN:           "Duplicate — %02d",
		KeyReviewDeleteN:        "Review & delete (%s)",
		KeySearchPlaceholder:    "Search by file name or path",
		KeyLocalIndex:           "Local index",
	},
	Chinese: {
		KeySubtitle:             "查找并删除重复文件",
		KeyPreview:              "预览",
		KeyLocalOnly:            "仅本地运行",
		KeyDismiss:              "关闭",
		KeySwitchLanguage:       "切换语言",
		KeyIndex:                "索引",
		KeyIndexedFiles:         "已索引文件",
		KeyDuplicateFiles:       "重复文件",
		KeyGroups:               "重复组",
		KeyReclaimable:          "可释放空间",
		KeyScanFolders:          "扫描文件夹",
		KeySharedIndex:          "索引与 onley CLI 共用。",
		KeyAddFolder:            "添加文件夹",
		KeyAdd:                  "添加",
		KeyRemove:               "移除",
		KeyMergeHint:            "子文件夹会归并到父文件夹。",
		KeyNoFolders:            "没有文件夹",
		KeyScanAll:              "扫描全部",
		KeyDropFolders:          "拖入文件夹",
		KeyDropHint:             "添加并归并后再开始扫描",
		KeyCleanup:              "清理",
		KeyRefreshIndex:         "刷新索引",
		KeyClearChoices:         "清除选择",
		KeyIndexRefreshed:       "索引已刷新。",
		KeyIndexFile:            "索引文件",
		KeyNotAvailable:         "不可用",
		KeyScanning:             "扫描中",
		KeyScanSpeedGauge:       "扫描速度仪表",
		KeyScanStats:            "已读取 %s · %s · 跳过 %d · 错误 %d",
		KeyElapsed:              "耗时 %s",
		KeyCancelScan:           "取消扫描",
		KeyCancelScanHint:       "已完成的文件会保存，重新扫描即可继续。",
		KeyScanCanceled:         "扫描已取消，保留部分索引。",
		KeyScanFinished:         "扫描完成：已处理 %d 个文件，跳过 %d 个未变化文件。",
		KeyReadErrors:           "%d 个文件无法读取",
		KeyCanceling:            "正在取消扫描…",
		KeyAddFolderFirst:       "请先添加要扫描的文件夹。",
		KeyCannotResolvePath:    "无法解析扫描路径：%v",
		KeyCannotScanFolder:     "无法扫描该文件夹：%v",
		KeyNotAFolder:           "扫描路径不是文件夹：%s",
		KeyCannotOpenIndex:      "无法打开索引：%v",
		KeyIndexUnavailable:     "索引不可用。",
		KeyCannotReadIndex:      "无法读取索引：%v",
		KeyCannotKeep:           "无法保留 %s：%v。请重新检查文件。",
		KeyDeleteCanceledKeep:   "删除已取消：要保留的副本已不可用。",
		KeyDeleted:              "已删除 %s，释放 %s。",
		KeyAlreadyGone:          "%d 个文件已不存在。",
		KeyFailed:               "%d 个失败。",
		KeyIndexCleanupFailed:   "索引清理失败",
		KeyDropFolderFirst:      "请拖入一个文件夹。",
		KeyDuplicateGroups:      "重复组",
		KeyNoDuplicates:         "暂无重复文件",
		KeyEmptyBody:            "扫描文件夹以建立索引。只有索引中的文件才能删除。",
		KeyCloseIndexHolder:     "请关闭占用索引的进程，然后重新打开 Onley。",
		KeyScanningDots:         "扫描中…",
		KeyScanningBody:         "扫描完成后会显示重复组。",
		KeyScanDefaultFolder:    "扫描默认文件夹",
		KeyChooseCopyOrKeep:     "请选择要保留的副本，或使用保留策略。",
		KeySelection:            "已选择 %s · 可释放 %s",
		KeyPermanentHint:        "永久删除。下一步会再确认具体文件。",
		KeyKeepPolicy:           "保留策略",
		KeyNewest:               "最新",
		KeyOldest:               "最旧",
		KeyPolicyHint:           "按修改时间选择；时间相同按路径排序。",
		KeyKeepNewestPerGroup:   "每组保留最新一个",
		KeyKeepOldestPerGroup:   "每组保留最旧一个",
		KeyKeepOne:              "保留一个",
		KeyReviewDelete:         "检查并删除",
		KeyConfirmDelete:        "确认删除",
		KeyDeleteDuplicates:     "删除重复文件？",
		KeyDeleteConfirm:        "删除 %s 并释放 %s？",
		KeyCannotUndo:           "此操作不可撤销。扫描后发生变化的文件会被跳过。",
		KeyAndMore:              "还有 %d 个",
		KeyCancel:               "取消",
		KeyDeletePermanently:    "永久删除",
		KeyGroupWord:            "组",
		KeyGroup:                "第 %d 组",
		KeyDuplicateGroupTitle:  "%d 组重复文件",
		KeyDuplicateGroupsTitle: "%d 组重复文件",
		KeyDuplicateSummary:     "共 %s，清理多余副本可释放 %s",
		KeyAutoKeep:             "自动保留",
		KeyRescan:               "重新扫描",
		KeyDelete:               "删除",
		KeyDeleteCount:          "删除 %s",
		KeyKeepHint:             "请选择要保留的副本。未选择的组会被跳过。",
		KeySelfKeepHint:         "Onley 会保留正在运行的自身程序。选中它即可删除其他副本。",
		KeyKeepBadge:            "保留",
		KeyRunning:              "运行中",
		KeyDeleteBadge:          "删除",
		KeyCannotDeleteSelf:     "Onley 无法删除正在运行的自身程序。",
		KeyFileMenu:             "文件",
		KeyEditMenu:             "编辑",
		KeyReclaimableSpace:     "可释放空间",
		KeyAllCopies:            "全部副本",
		KeyImages:               "图片",
		KeyVideos:               "视频",
		KeyDocuments:            "文档",
		KeyArchives:             "压缩包",
		KeySystem:               "系统",
		KeySearch:               "搜索",
		KeySourceFolders:        "源文件夹",
		KeyAllFolders:           "全部文件夹",
		KeyDropDirectory:        "拖入文件夹",
		KeyInitializeScan:       "开始扫描序列",
		KeyRunGlobalScan:        "全局扫描",
		KeyAutoSelectPolicy:     "自动选择策略：",
		KeyKeepNewest:           "保留最新",
		KeyKeepOldest:           "保留最旧",
		KeyKeepByPath:           "按路径保留",
		KeyClear:                "清除",
		KeyOriginalCopy:         "原始副本",
		KeyDuplicateN:           "重复副本 — %02d",
		KeyReviewDeleteN:        "检查并删除（%s）",
		KeySearchPlaceholder:    "按文件名或路径搜索",
		KeyLocalIndex:           "本地索引",
	},
}

// T returns key's text in lang, falling back to English.
func T(lang Lang, key Key) string {
	if catalog := messages[lang]; catalog != nil {
		if text, ok := catalog[key]; ok {
			return text
		}
	}
	if text, ok := messages[English][key]; ok {
		return text
	}
	return string(key)
}

// F formats key's text in lang with args.
func F(lang Lang, key Key, args ...any) string {
	return fmt.Sprintf(T(lang, key), args...)
}

// FileCount returns a localized file count.
func FileCount(lang Lang, n int) string {
	if lang == Chinese {
		return fmt.Sprintf("%d 个文件", n)
	}
	if n == 1 {
		return "1 file"
	}
	return fmt.Sprintf("%d files", n)
}

// DuplicateFileCount returns a localized duplicate-file count.
func DuplicateFileCount(lang Lang, n int) string {
	if lang == Chinese {
		return fmt.Sprintf("%d 个重复文件", n)
	}
	if n == 1 {
		return "1 duplicate file"
	}
	return fmt.Sprintf("%d duplicate files", n)
}

// GroupCount returns a localized group count.
func GroupCount(lang Lang, n int) string {
	if lang == Chinese {
		return fmt.Sprintf("%d 组", n)
	}
	if n == 1 {
		return "1 group"
	}
	return fmt.Sprintf("%d groups", n)
}

// CopyCount returns a localized copy count.
func CopyCount(lang Lang, n int) string {
	if lang == Chinese {
		return fmt.Sprintf("%d 份", n)
	}
	if n == 1 {
		return "1 copy"
	}
	return fmt.Sprintf("%d copies", n)
}

// FolderCount returns a localized folder count.
func FolderCount(lang Lang, n int) string {
	if lang == Chinese {
		return fmt.Sprintf("%d 个文件夹", n)
	}
	if n == 1 {
		return "1 folder"
	}
	return fmt.Sprintf("%d folders", n)
}
