//! Translated text for the desktop client.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    English,
    Chinese,
}

pub const O_R_D_E_R: [Lang; 2] = [Lang::English, Lang::Chinese];

impl Lang {
    pub fn as_str(self) -> &'static str {
        match self {
            Lang::English => "en",
            Lang::Chinese => "zh",
        }
    }
}

impl std::fmt::Display for Lang {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Picks a language from a system locale such as `zh-Hans` or `en-US`.
pub fn detect(locale: &str) -> Lang {
    if locale.to_lowercase().starts_with("zh") {
        Lang::Chinese
    } else {
        Lang::English
    }
}

pub fn parse(value: &str) -> Option<Lang> {
    match value {
        "en" => Some(Lang::English),
        "zh" => Some(Lang::Chinese),
        _ => None,
    }
}
pub fn next(lang: Lang) -> Lang {
    match lang {
        Lang::English => Lang::Chinese,
        Lang::Chinese => Lang::English,
    }
}

// Message keys.
pub const SUBTITLE: &str = "subtitle";
pub const PREVIEW: &str = "preview";
pub const LOCAL_ONLY: &str = "localOnly";
pub const DISMISS: &str = "dismiss";
pub const SWITCH_LANGUAGE: &str = "switchLanguage";
pub const INDEX: &str = "index";
pub const INDEXED_FILES: &str = "indexedFiles";
pub const DUPLICATE_FILES: &str = "duplicateFiles";
pub const GROUPS: &str = "groups";
pub const RECLAIMABLE: &str = "reclaimable";
pub const SCAN_FOLDERS: &str = "scanFolders";
pub const SHARED_INDEX: &str = "sharedIndex";
pub const ADD_FOLDER: &str = "addFolder";
pub const ADD: &str = "add";
pub const REMOVE: &str = "remove";
pub const MERGE_HINT: &str = "mergeHint";
pub const NO_FOLDERS: &str = "noFolders";
pub const SCAN_ALL: &str = "scanAll";
pub const DROP_FOLDERS: &str = "dropFolders";
pub const DROP_HINT: &str = "dropHint";
pub const CLEANUP: &str = "cleanup";
pub const REFRESH_INDEX: &str = "refreshIndex";
pub const CLEAR_CHOICES: &str = "clearChoices";
pub const INDEX_REFRESHED: &str = "indexRefreshed";
pub const INDEX_FILE: &str = "indexFile";
pub const NOT_AVAILABLE: &str = "notAvailable";
pub const SCANNING: &str = "scanning";
pub const SCAN_SPEED_GAUGE: &str = "scanSpeedGauge";
pub const SCAN_STATS: &str = "scanStats";
pub const ELAPSED: &str = "elapsed";
pub const CANCEL_SCAN: &str = "cancelScan";
pub const CANCEL_SCAN_HINT: &str = "cancelScanHint";
pub const SCAN_CANCELED: &str = "scanCanceled";
pub const SCAN_FINISHED: &str = "scanFinished";
pub const READ_ERRORS: &str = "readErrors";
pub const CANCELING: &str = "canceling";
pub const ADD_FOLDER_FIRST: &str = "addFolderFirst";
pub const CANNOT_RESOLVE_PATH: &str = "cannotResolvePath";
pub const CANNOT_SCAN_FOLDER: &str = "cannotScanFolder";
pub const NOT_A_FOLDER: &str = "notAFolder";
pub const CANNOT_OPEN_INDEX: &str = "cannotOpenIndex";
pub const INDEX_UNAVAILABLE: &str = "indexUnavailable";
pub const CANNOT_READ_INDEX: &str = "cannotReadIndex";
pub const CANNOT_KEEP: &str = "cannotKeep";
pub const DELETE_CANCELED_KEEP: &str = "deleteCanceledKeep";
pub const DELETED: &str = "deleted";
pub const ALREADY_GONE: &str = "alreadyGone";
pub const FAILED: &str = "failed";
pub const INDEX_CLEANUP_FAILED: &str = "indexCleanupFailed";
pub const DROP_FOLDER_FIRST: &str = "dropFolderFirst";
pub const DUPLICATE_GROUPS: &str = "duplicateGroups";
pub const NO_DUPLICATES: &str = "noDuplicates";
pub const EMPTY_BODY: &str = "emptyBody";
pub const CLOSE_INDEX_HOLDER: &str = "closeIndexHolder";
pub const SCANNING_DOTS: &str = "scanningDots";
pub const SCANNING_BODY: &str = "scanningBody";
pub const SCAN_DEFAULT_FOLDER: &str = "scanDefaultFolder";
pub const CHOOSE_COPY_OR_KEEP: &str = "chooseCopyOrKeep";
pub const SELECTION: &str = "selection";
pub const PERMANENT_HINT: &str = "permanentHint";
pub const KEEP_POLICY: &str = "keepPolicy";
pub const NEWEST: &str = "newest";
pub const OLDEST: &str = "oldest";
pub const POLICY_HINT: &str = "policyHint";
pub const KEEP_NEWEST_PER_GROUP: &str = "keepNewestPerGroup";
pub const KEEP_OLDEST_PER_GROUP: &str = "keepOldestPerGroup";
pub const KEEP_ONE: &str = "keepOne";
pub const REVIEW_DELETE: &str = "reviewDelete";
pub const CONFIRM_DELETE: &str = "confirmDelete";
pub const DELETE_DUPLICATES: &str = "deleteDuplicates";
pub const DELETE_CONFIRM: &str = "deleteConfirm";
pub const CANNOT_UNDO: &str = "cannotUndo";
pub const AND_MORE: &str = "andMore";
pub const CANCEL: &str = "cancel";
pub const DELETE_PERMANENTLY: &str = "deletePermanently";
pub const GROUP_WORD: &str = "groupWord";
pub const GROUP: &str = "group";
pub const DUPLICATE_GROUP_TITLE: &str = "duplicateGroupTitle";
pub const DUPLICATE_GROUPS_TITLE: &str = "duplicateGroupsTitle";
pub const DUPLICATE_SUMMARY: &str = "duplicateSummary";
pub const AUTO_KEEP: &str = "autoKeep";
pub const RESCAN: &str = "rescan";
pub const DELETE: &str = "delete";
pub const DELETE_COUNT: &str = "deleteCount";
pub const KEEP_HINT: &str = "keepHint";
pub const SELF_KEEP_HINT: &str = "selfKeepHint";
pub const KEEP_BADGE: &str = "keepBadge";
pub const RUNNING: &str = "running";
pub const DELETE_BADGE: &str = "deleteBadge";
pub const CANNOT_DELETE_SELF: &str = "cannotDeleteSelf";
pub const FILE_MENU: &str = "fileMenu";
pub const EDIT_MENU: &str = "editMenu";
pub const RECLAIMABLE_SPACE: &str = "reclaimableSpace";
pub const ALL_COPIES: &str = "allCopies";
pub const IMAGES: &str = "images";
pub const VIDEOS: &str = "videos";
pub const DOCUMENTS: &str = "documents";
pub const ARCHIVES: &str = "archives";
pub const SYSTEM: &str = "system";
pub const SEARCH: &str = "search";
pub const SOURCE_FOLDERS: &str = "sourceFolders";
pub const FILE_TYPES: &str = "fileTypes";
pub const ALL_FOLDERS: &str = "allFolders";
pub const DROP_DIRECTORY: &str = "dropDirectory";
pub const RUN_GLOBAL_SCAN: &str = "runGlobalScan";
pub const AUTO_SELECT_POLICY: &str = "autoSelectPolicy";
pub const KEEP_NEWEST: &str = "keepNewest";
pub const KEEP_OLDEST: &str = "keepOldest";
pub const KEEP_BY_PATH: &str = "keepByPath";
pub const CLEAR: &str = "clear";
pub const ORIGINAL_COPY: &str = "originalCopy";
pub const DUPLICATE_N: &str = "duplicateN";
pub const REVIEW_DELETE_N: &str = "reviewDeleteN";
pub const SEARCH_PLACEHOLDER: &str = "searchPlaceholder";
pub const LOCAL_INDEX: &str = "localIndex";
pub const PERMISSION_DELETE: &str = "permissionDelete";
pub const PERMISSION_KEEP: &str = "permissionKeep";
pub const PERMISSION_READ_ERRORS: &str = "permissionReadErrors";
pub const PERMISSION_FOLDER: &str = "permissionFolder";
pub const PERMISSION_INDEX: &str = "permissionIndex";
pub const PASTE_PATH: &str = "pastePath";
pub const BROWSE: &str = "browse";
pub const CLIPBOARD_EMPTY: &str = "clipboardEmpty";
pub const RECENT_FOLDERS: &str = "recentFolders";
pub const PATH_USABLE: &str = "pathUsable";
pub const PATH_MISSING: &str = "pathMissing";
pub const PATH_FILE: &str = "pathFile";
pub const PATH_DENIED: &str = "pathDenied";
pub const BROWSE_HINT: &str = "browseHint";
pub const TYPE_PATH: &str = "typePath";
pub const REMOVED_FOLDERS: &str = "removedFolders";
pub const SETTINGS: &str = "settings";
pub const LANGUAGE: &str = "language";
pub const CLOSE: &str = "close";
pub const ABOUT: &str = "about";
pub const VERSION: &str = "version";
pub const DATA_DIR: &str = "dataDir";

fn english(key: &str) -> Option<&'static str> {
    Some(match key {
        "subtitle" => "Find and remove duplicate files",
        "preview" => "Preview",
        "localOnly" => "Local only",
        "dismiss" => "Dismiss",
        "switchLanguage" => "Switch language",
        "index" => "Index",
        "indexedFiles" => "Indexed files",
        "duplicateFiles" => "Duplicate files",
        "groups" => "Groups",
        "reclaimable" => "Reclaimable",
        "scanFolders" => "Scan folders",
        "sharedIndex" => "The index is shared with the onley CLI.",
        "addFolder" => "Add a folder",
        "add" => "Add",
        "remove" => "Remove",
        "mergeHint" => "Child folders merge into their parent.",
        "noFolders" => "no folders",
        "scanAll" => "Scan all",
        "dropFolders" => "Drop folders here",
        "dropHint" => "they are added and merged before scanning",
        "cleanup" => "Cleanup",
        "refreshIndex" => "Refresh index",
        "clearChoices" => "Clear choices",
        "indexRefreshed" => "Index refreshed.",
        "indexFile" => "Index file",
        "notAvailable" => "not available",
        "scanning" => "Scanning",
        "scanSpeedGauge" => "Scan speed gauge",
        "scanStats" => "{} hashed · {} · {} skipped · {} errors",
        "elapsed" => "elapsed {}",
        "cancelScan" => "Cancel scan",
        "cancelScanHint" => "Finished files are saved. Scan again to resume.",
        "scanCanceled" => "Scan canceled. The partial index is kept.",
        "scanFinished" => "Scan finished: {} files processed, {} unchanged skipped.",
        "readErrors" => "{} file(s) could not be read",
        "canceling" => "Canceling the scan…",
        "addFolderFirst" => "Add a folder to scan.",
        "cannotResolvePath" => "Cannot resolve the scan path: {}",
        "cannotScanFolder" => "Cannot scan that folder: {}",
        "notAFolder" => "The scan path is not a folder: {}",
        "cannotOpenIndex" => "Cannot open the index: {}",
        "indexUnavailable" => "The index is not available.",
        "cannotReadIndex" => "Cannot read the index: {}",
        "cannotKeep" => "Cannot keep {}: {}. Review the files again.",
        "deleteCanceledKeep" => "Delete canceled: the copy to keep is no longer available.",
        "deleted" => "Deleted {}, freed {}.",
        "alreadyGone" => "{} was already gone.",
        "failed" => "{} failed.",
        "indexCleanupFailed" => "index cleanup failed",
        "dropFolderFirst" => "Drop a folder to scan it.",
        "duplicateGroups" => "Duplicate groups",
        "noDuplicates" => "No duplicates yet",
        "emptyBody" => "Scan a folder to build the index. Only files in the index can be deleted.",
        "closeIndexHolder" => "Close any process that holds the index, then reopen Onley.",
        "scanningDots" => "Scanning…",
        "scanningBody" => "Duplicate groups appear when the scan finishes.",
        "scanDefaultFolder" => "Scan default folder",
        "chooseCopyOrKeep" => "Choose a copy, or use the keep policy.",
        "selection" => "{} selected · {} reclaimable",
        "permanentHint" => "Permanent deletion. You will confirm the exact files next.",
        "keepPolicy" => "Keep policy",
        "newest" => "Newest",
        "oldest" => "Oldest",
        "policyHint" => "Uses modification time; ties use path.",
        "keepNewestPerGroup" => "Keep newest per group",
        "keepOldestPerGroup" => "Keep oldest per group",
        "keepOne" => "Keep one",
        "reviewDelete" => "Review & delete",
        "confirmDelete" => "Confirm delete",
        "deleteDuplicates" => "Delete duplicates?",
        "deleteConfirm" => "Delete {} and free {}?",
        "cannotUndo" => "This cannot be undone. Files changed since the scan are skipped.",
        "andMore" => "and {} more",
        "cancel" => "Cancel",
        "deletePermanently" => "Delete permanently",
        "groupWord" => "Group",
        "group" => "Group {}",
        "duplicateGroupTitle" => "{} duplicate group",
        "duplicateGroupsTitle" => "{} duplicate groups",
        "duplicateSummary" => "{} · {} reclaimable",
        "autoKeep" => "Auto keep",
        "rescan" => "Rescan",
        "delete" => "Delete",
        "deleteCount" => "Delete {}",
        "keepHint" => "Choose the copy to keep. A group without a choice is skipped.",
        "selfKeepHint" => {
            "Onley keeps its running executable. Select it to delete the other copies."
        }
        "keepBadge" => "keep",
        "running" => "running",
        "deleteBadge" => "delete",
        "cannotDeleteSelf" => "Onley cannot delete its own running executable.",
        "fileMenu" => "File",
        "editMenu" => "Edit",
        "reclaimableSpace" => "Reclaimable space",
        "allCopies" => "All copies",
        "images" => "Images",
        "videos" => "Videos",
        "documents" => "Documents",
        "archives" => "Archives",
        "system" => "System",
        "search" => "Search",
        "sourceFolders" => "Source folders",
        "fileTypes" => "File types",
        "allFolders" => "All folders",
        "dropDirectory" => "Drop directory here",
        "runGlobalScan" => "Run global scan",
        "autoSelectPolicy" => "Auto-select policy:",
        "keepNewest" => "Keep newest",
        "keepOldest" => "Keep oldest",
        "keepByPath" => "Keep by path",
        "clear" => "Clear",
        "originalCopy" => "Original copy",
        "duplicateN" => "Duplicate — {}",
        "reviewDeleteN" => "Review & delete ({})",
        "searchPlaceholder" => "Search by file name or path",
        "localIndex" => "Local index",
        "permissionDelete" => {
            "Cannot delete {}: permission denied. Check the file or folder permissions, then try again."
        }
        "permissionKeep" => {
            "Cannot keep {}: permission denied. Check the file or folder permissions."
        }
        "permissionReadErrors" => {
            "{} file(s) could not be read because of permissions. Check folder access or grant Onley Full Disk Access."
        }
        "permissionFolder" => {
            "Cannot read {}: permission denied. Check the folder permissions or grant Onley Full Disk Access."
        }
        "permissionIndex" => {
            "Cannot open the index: permission denied. Check the permissions on ~/.onley."
        }
        "pastePath" => "Paste",
        "browse" => "Browse…",
        "clipboardEmpty" => "The clipboard holds no folder or path.",
        "recentFolders" => "Recent folders",
        "pathUsable" => "Folder can be added.",
        "pathMissing" => "No folder at this path.",
        "pathFile" => "That path is a file, not a folder.",
        "pathDenied" => "Cannot read that folder: permission denied.",
        "browseHint" => "click to browse or drop a folder",
        "typePath" => "Type a path",
        "removedFolders" => "{} empty folder(s) removed.",
        "settings" => "Settings",
        "language" => "Language",
        "close" => "Close",
        "about" => "About",
        "version" => "Version",
        "dataDir" => "Data folder",
        _ => return None,
    })
}

fn chinese(key: &str) -> Option<&'static str> {
    Some(match key {
        "subtitle" => "查找并删除重复文件",
        "preview" => "预览",
        "localOnly" => "仅本地运行",
        "dismiss" => "关闭",
        "switchLanguage" => "切换语言",
        "index" => "索引",
        "indexedFiles" => "已索引文件",
        "duplicateFiles" => "重复文件",
        "groups" => "重复组",
        "reclaimable" => "可释放空间",
        "scanFolders" => "扫描文件夹",
        "sharedIndex" => "索引与 onley CLI 共用。",
        "addFolder" => "添加文件夹",
        "add" => "添加",
        "remove" => "移除",
        "mergeHint" => "子文件夹会归并到父文件夹。",
        "noFolders" => "没有文件夹",
        "scanAll" => "扫描全部",
        "dropFolders" => "拖入文件夹",
        "dropHint" => "添加并归并后再开始扫描",
        "cleanup" => "清理",
        "refreshIndex" => "刷新索引",
        "clearChoices" => "清除选择",
        "indexRefreshed" => "索引已刷新。",
        "indexFile" => "索引文件",
        "notAvailable" => "不可用",
        "scanning" => "扫描中",
        "scanSpeedGauge" => "扫描速度仪表",
        "scanStats" => "已读取 {} · {} · 跳过 {} · 错误 {}",
        "elapsed" => "耗时 {}",
        "cancelScan" => "取消扫描",
        "cancelScanHint" => "已完成的文件会保存，重新扫描即可继续。",
        "scanCanceled" => "扫描已取消，保留部分索引。",
        "scanFinished" => "扫描完成：已处理 {} 个文件，跳过 {} 个未变化文件。",
        "readErrors" => "{} 个文件无法读取",
        "canceling" => "正在取消扫描…",
        "addFolderFirst" => "请先添加要扫描的文件夹。",
        "cannotResolvePath" => "无法解析扫描路径：{}",
        "cannotScanFolder" => "无法扫描该文件夹：{}",
        "notAFolder" => "扫描路径不是文件夹：{}",
        "cannotOpenIndex" => "无法打开索引：{}",
        "indexUnavailable" => "索引不可用。",
        "cannotReadIndex" => "无法读取索引：{}",
        "cannotKeep" => "无法保留 {}：{}。请重新检查文件。",
        "deleteCanceledKeep" => "删除已取消：要保留的副本已不可用。",
        "deleted" => "已删除 {}，释放 {}。",
        "alreadyGone" => "{} 个文件已不存在。",
        "failed" => "{} 个失败。",
        "indexCleanupFailed" => "索引清理失败",
        "dropFolderFirst" => "请拖入一个文件夹。",
        "duplicateGroups" => "重复组",
        "noDuplicates" => "暂无重复文件",
        "emptyBody" => "扫描文件夹以建立索引。只有索引中的文件才能删除。",
        "closeIndexHolder" => "请关闭占用索引的进程，然后重新打开 Onley。",
        "scanningDots" => "扫描中…",
        "scanningBody" => "扫描完成后会显示重复组。",
        "scanDefaultFolder" => "扫描默认文件夹",
        "chooseCopyOrKeep" => "请选择要保留的副本，或使用保留策略。",
        "selection" => "已选择 {} · 可释放 {}",
        "permanentHint" => "永久删除。下一步会再确认具体文件。",
        "keepPolicy" => "保留策略",
        "newest" => "最新",
        "oldest" => "最旧",
        "policyHint" => "按修改时间选择；时间相同按路径排序。",
        "keepNewestPerGroup" => "每组保留最新一个",
        "keepOldestPerGroup" => "每组保留最旧一个",
        "keepOne" => "保留一个",
        "reviewDelete" => "检查并删除",
        "confirmDelete" => "确认删除",
        "deleteDuplicates" => "删除重复文件？",
        "deleteConfirm" => "删除 {} 并释放 {}？",
        "cannotUndo" => "此操作不可撤销。扫描后发生变化的文件会被跳过。",
        "andMore" => "还有 {} 个",
        "cancel" => "取消",
        "deletePermanently" => "永久删除",
        "groupWord" => "组",
        "group" => "第 {} 组",
        "duplicateGroupTitle" => "{} 组重复文件",
        "duplicateGroupsTitle" => "{} 组重复文件",
        "duplicateSummary" => "共 {}，清理多余副本可释放 {}",
        "autoKeep" => "自动保留",
        "rescan" => "重新扫描",
        "delete" => "删除",
        "deleteCount" => "删除 {}",
        "keepHint" => "请选择要保留的副本。未选择的组会被跳过。",
        "selfKeepHint" => "Onley 会保留正在运行的自身程序。选中它即可删除其他副本。",
        "keepBadge" => "保留",
        "running" => "运行中",
        "deleteBadge" => "删除",
        "cannotDeleteSelf" => "Onley 无法删除正在运行的自身程序。",
        "fileMenu" => "文件",
        "editMenu" => "编辑",
        "reclaimableSpace" => "可释放空间",
        "allCopies" => "全部副本",
        "images" => "图片",
        "videos" => "视频",
        "documents" => "文档",
        "archives" => "压缩包",
        "system" => "系统",
        "search" => "搜索",
        "sourceFolders" => "源文件夹",
        "fileTypes" => "文件类型",
        "allFolders" => "全部文件夹",
        "dropDirectory" => "拖入文件夹",
        "runGlobalScan" => "全局扫描",
        "autoSelectPolicy" => "自动选择策略：",
        "keepNewest" => "保留最新",
        "keepOldest" => "保留最旧",
        "keepByPath" => "按路径保留",
        "clear" => "清除",
        "originalCopy" => "原始副本",
        "duplicateN" => "重复副本 — {}",
        "reviewDeleteN" => "检查并删除（{}）",
        "searchPlaceholder" => "按文件名或路径搜索",
        "localIndex" => "本地索引",
        "permissionDelete" => "无法删除 {}：权限不足。请检查文件或文件夹权限后重试。",
        "permissionKeep" => "无法保留 {}：权限不足。请检查文件或文件夹权限。",
        "permissionReadErrors" => {
            "{} 个文件因权限不足无法读取。请检查文件夹权限，或授予 Onley 完全磁盘访问权限。"
        }
        "permissionFolder" => {
            "无法读取 {}：权限不足。请检查文件夹权限，或授予 Onley 完全磁盘访问权限。"
        }
        "permissionIndex" => "无法打开索引：权限不足。请检查 ~/.onley 的权限。",
        "pastePath" => "粘贴",
        "browse" => "浏览…",
        "clipboardEmpty" => "剪贴板里没有文件夹或路径。",
        "recentFolders" => "最近使用",
        "pathUsable" => "可以添加该文件夹。",
        "pathMissing" => "该路径下没有文件夹。",
        "pathFile" => "该路径是文件，不是文件夹。",
        "pathDenied" => "无法读取该文件夹：权限不足。",
        "browseHint" => "点击浏览或拖入文件夹",
        "typePath" => "输入路径",
        "removedFolders" => "已移除 {} 个空文件夹。",
        "settings" => "设置",
        "language" => "语言",
        "close" => "关闭",
        "about" => "关于",
        "version" => "版本",
        "dataDir" => "数据目录",
        _ => return None,
    })
}

/// Returns the message for a key in a language, falling back to English.
pub fn t(lang: Lang, key: &'static str) -> &'static str {
    match lang {
        Lang::Chinese => chinese(key).or_else(|| english(key)).unwrap_or(key),
        Lang::English => english(key).unwrap_or(key),
    }
}

/// Formats a message with arguments, one per `{}`.
pub fn f(lang: Lang, key: &'static str, args: &[&dyn std::fmt::Display]) -> String {
    let mut text = t(lang, key).to_string();
    for arg in args {
        text = text.replacen("{}", &arg.to_string(), 1);
    }
    text
}

pub fn file_count(lang: Lang, n: usize) -> String {
    match lang {
        Lang::Chinese => format!("{} 个文件", n),
        _ if n == 1 => "1 file".into(),
        _ => format!("{} files", n),
    }
}

pub fn duplicate_file_count(lang: Lang, n: usize) -> String {
    match lang {
        Lang::Chinese => format!("{} 个重复文件", n),
        _ if n == 1 => "1 duplicate file".into(),
        _ => format!("{} duplicate files", n),
    }
}

pub fn group_count(lang: Lang, n: usize) -> String {
    match lang {
        Lang::Chinese => format!("{} 组", n),
        _ if n == 1 => "1 group".into(),
        _ => format!("{} groups", n),
    }
}

pub fn copy_count(lang: Lang, n: usize) -> String {
    match lang {
        Lang::Chinese => format!("{} 份", n),
        _ if n == 1 => "1 copy".into(),
        _ => format!("{} copies", n),
    }
}

pub fn folder_count(lang: Lang, n: usize) -> String {
    match lang {
        Lang::Chinese => format!("{} 个文件夹", n),
        _ if n == 1 => "1 folder".into(),
        _ => format!("{} folders", n),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_chinese_and_english() {
        assert_eq!(detect("zh-Hans"), Lang::Chinese);
        assert_eq!(detect("en-US"), Lang::English);
        assert_eq!(detect(""), Lang::English);
    }

    #[test]
    fn parses_and_cycles() {
        assert_eq!(parse("zh"), Some(Lang::Chinese));
        assert_eq!(parse("en"), Some(Lang::English));
        assert_eq!(parse("fr"), None);
        assert_eq!(next(Lang::English), Lang::Chinese);
        assert_eq!(next(Lang::Chinese), Lang::English);
    }

    #[test]
    fn every_key_has_both_languages() {
        let keys = ["subtitle", "settings", "deleteConfirmed"];
        for key in keys {
            assert!(!t(Lang::English, key).is_empty());
        }
        assert_eq!(t(Lang::English, "settings"), "Settings");
        assert_eq!(t(Lang::Chinese, "settings"), "设置");
    }

    #[test]
    fn formats_arguments_in_order() {
        assert_eq!(
            f(Lang::English, "deleted", &[&"1 file", &"2.0 MB"]),
            "Deleted 1 file, freed 2.0 MB."
        );
    }

    #[test]
    fn counts_are_localized() {
        assert_eq!(file_count(Lang::English, 1), "1 file");
        assert_eq!(file_count(Lang::English, 2), "2 files");
        assert_eq!(file_count(Lang::Chinese, 2), "2 个文件");
    }
}
