//! The app's two languages.

use nsh_core::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lang {
    #[default]
    En,
    Zh,
}

impl Lang {
    /// Follows the system language.
    pub fn detect() -> Self {
        if locale().to_string().to_lowercase().contains("zh") {
            Self::Zh
        } else {
            Self::En
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            Self::En => Self::Zh,
            Self::Zh => Self::En,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::En => "EN",
            Self::Zh => "中文",
        }
    }
}

pub struct Strings {
    pub app_name: &'static str,
    pub tagline: &'static str,
    pub scan: &'static str,
    pub scanning: &'static str,
    pub cancel: &'static str,
    pub groups: &'static str,
    pub reclaimable: &'static str,
    pub keep_newest: &'static str,
    pub keep_oldest: &'static str,
    pub delete_copies: &'static str,
    pub confirm_title: &'static str,
    pub confirm_body: &'static str,
    pub delete: &'static str,
    pub back: &'static str,
    pub no_duplicates: &'static str,
    pub empty_hint: &'static str,
    pub access_denied: &'static str,
    pub access_limited: &'static str,
    pub access_unknown: &'static str,
    pub access_unsupported: &'static str,
    pub grant_access: &'static str,
    pub scan_done: &'static str,
    pub scan_failed: &'static str,
    pub deleted: &'static str,
    pub delete_failed: &'static str,
    pub copies: &'static str,
    pub kept: &'static str,
    pub keep: &'static str,
    pub failed: &'static str,
}

static EN: Strings = Strings {
    app_name: "Onley Photos",
    tagline: "Find duplicate photos on this phone.",
    scan: "SCAN PHOTOS",
    scanning: "SCANNING",
    cancel: "CANCEL",
    groups: "groups",
    reclaimable: "reclaimable",
    keep_newest: "KEEP NEWEST",
    keep_oldest: "KEEP OLDEST",
    delete_copies: "DELETE",
    confirm_title: "Delete duplicate photos?",
    confirm_body: "The kept copy stays. The others go to Recently Deleted and can be recovered there.",
    delete: "DELETE",
    back: "BACK",
    no_duplicates: "No duplicate photos.",
    empty_hint: "Scan the photo library to find copies that take the same space.",
    access_denied: "Photo access is off. Allow it in Settings > Privacy > Photos.",
    access_limited: "Only the photos you selected are visible. Allow all photos in Settings to find every copy.",
    access_unknown: "Allow photo access to scan the library.",
    access_unsupported: "Photo access needs the iOS app.",
    grant_access: "ALLOW ACCESS",
    scan_done: "Scan finished.",
    scan_failed: "Some photos could not be read.",
    deleted: "Deleted copies.",
    delete_failed: "Could not delete the copies.",
    copies: "copies",
    kept: "KEPT",
    keep: "KEEP",
    failed: "failed",
};

static ZH: Strings = Strings {
    app_name: "Onley 照片",
    tagline: "找出手机里的重复照片。",
    scan: "扫描照片",
    scanning: "扫描中",
    cancel: "取消",
    groups: "组重复",
    reclaimable: "可释放",
    keep_newest: "保留最新",
    keep_oldest: "保留最旧",
    delete_copies: "删除",
    confirm_title: "删除重复照片？",
    confirm_body: "保留的那张不会动，其余会进入「最近删除」，仍可恢复。",
    delete: "删除",
    back: "返回",
    no_duplicates: "没有重复照片。",
    empty_hint: "扫描相册，找出占用了同一份空间的副本。",
    access_denied: "照片权限已关闭。请在「设置 > 隐私 > 照片」中允许访问。",
    access_limited: "当前只能看到你选中的照片。在设置中允许访问全部照片才能找出所有副本。",
    access_unknown: "允许访问相册后即可扫描。",
    access_unsupported: "照片访问需要 iOS 应用。",
    grant_access: "允许访问",
    scan_done: "扫描完成。",
    scan_failed: "部分照片无法读取。",
    deleted: "已删除副本。",
    delete_failed: "删除副本失败。",
    copies: "张副本",
    kept: "保留",
    keep: "保留",
    failed: "失败",
};

pub fn strings(lang: Lang) -> &'static Strings {
    match lang {
        Lang::En => &EN,
        Lang::Zh => &ZH,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_languages_have_every_label() {
        for lang in [Lang::En, Lang::Zh] {
            let s = strings(lang);
            for value in [
                s.app_name,
                s.tagline,
                s.scan,
                s.scanning,
                s.cancel,
                s.groups,
                s.reclaimable,
                s.keep_newest,
                s.keep_oldest,
                s.delete_copies,
                s.confirm_title,
                s.confirm_body,
                s.delete,
                s.back,
                s.no_duplicates,
                s.empty_hint,
                s.access_denied,
                s.access_limited,
                s.access_unknown,
                s.access_unsupported,
                s.grant_access,
                s.scan_done,
                s.scan_failed,
                s.deleted,
                s.delete_failed,
                s.copies,
                s.kept,
                s.keep,
                s.failed,
            ] {
                assert!(!value.is_empty());
            }
        }
    }

    #[test]
    fn toggles_language() {
        assert_eq!(Lang::En.toggle(), Lang::Zh);
        assert_eq!(Lang::Zh.toggle(), Lang::En);
    }
}
