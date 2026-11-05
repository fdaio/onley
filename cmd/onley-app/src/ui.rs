//! The desktop client's view.

use crate::categories::{self, Category, TABS};
use crate::gauge;
use crate::state::{PathStatus, State, display_path};
use crate::theme;
use nsh_base::input;
use nsh_core::{
    element::{AccessibilityRole, AdaptiveColor, FontWeight, rgb},
    geometry::{deg, px},
    prelude::*,
};
use onley_core::{appmodel, db::DuplicateGroup, format, i18n};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    Primary,
    Secondary,
    Danger,
}

/// The whole window. Everything the structure depends on is read here, so a
/// change builds the tree again.
pub fn root(s: &State) -> impl IntoElement + use<> {
    let state = s.clone();
    dynamic(move || {
        let s = state.clone();
        let _ = s.lang.get();
        let _ = s.keep_policy.get();
        let _ = s.keep.get();
        let _ = s.groups.get();
        let _ = s.scoped.get();
        let _ = s.visible.get();
        let _ = s.summary.get();
        let _ = s.indexed.get();
        let _ = s.category.get();
        let _ = s.active_root.get();
        let _ = s.scanning.get();
        let _ = s.busy.get();
        let _ = s.message.get();
        let _ = s.note.get();
        let _ = s.note_success.get();
        let _ = s.confirm.get();
        let _ = s.adding_folder.get();
        let _ = s.recent.get();
        let _ = s.settings.get();
        let _ = s.progress.get();
        let _ = s.speed_max.get();
        let _ = s.scan_dirs.get();
        main_view(&s)
    })
}

fn main_view(s: &State) -> impl IntoElement + use<> {
    let state = s.clone();
    let size = window();
    let compact = size.width.0 < 900.0 || size.height.0 < 700.0;
    view()
        .flex_1()
        .w_full()
        .h_full()
        .bg(theme::BACKGROUND)
        .test_id("root")
        .on_file_drop(move |event: &FileDropEvent| {
            for uri in &event.uris {
                if let Some(path) = onley_core::replica::file_url_path(uri) {
                    state.add_folder(&path.to_string_lossy());
                }
            }
        })
        .child(
            view()
                .flex_1()
                .w_full()
                .h_full()
                .child(header(s, compact))
                .child(tab_bar(s))
                .child(search_bar(s))
                .child(body(s, compact))
                .child(bottom_bar(s))
                .child(add_folder_dialog(s))
                .child(settings_dialog(s)),
        )
}

/// A press target with the id UI tests find it by.
fn press(id: impl Into<String>, disabled: bool, on_press: impl Fn() + 'static) -> View {
    view()
        .test_id(&id.into())
        .disabled(disabled)
        .role(AccessibilityRole::Button)
        .on_press(on_press)
}

fn text_line(
    value: impl Into<String>,
    size: f32,
    weight: FontWeight,
    color: AdaptiveColor,
) -> Text {
    text(value.into())
        .font_family(theme::MONO_FONT)
        .text_size(px(size))
        .font_weight(weight)
        .text_color(color)
}

fn text_line_dyn(
    value: impl Fn() -> String + 'static,
    size: f32,
    weight: FontWeight,
    color: AdaptiveColor,
) -> Text {
    text(value)
        .font_family(theme::MONO_FONT)
        .text_size(px(size))
        .font_weight(weight)
        .text_color(color)
}

fn header(s: &State, compact: bool) -> impl IntoElement + use<> {
    view()
        .w_full()
        .h(px(if compact { 128.0 } else { 190.0 }))
        .flex_row()
        .gap(px(24.0))
        .px(px(26.0))
        .py(px(16.0))
        .bg(theme::SURFACE)
        .border_b_1()
        .border_color(theme::BORDER)
        .child(
            view()
                .flex_1()
                .justify_center()
                .gap(px(6.0))
                .child(
                    text("ONLEY")
                        .font_family(theme::DISPLAY_FONT)
                        .text_size(px(if compact { 48.0 } else { 76.0 }))
                        .text_color(theme::TEXT),
                )
                .child(when(
                    move || !compact,
                    || {
                        text("NAVIGATING A WORLD OF\nSHIFTING DATA\nTRADITIONS.")
                            .font_family(theme::DISPLAY_FONT)
                            .text_size(px(17.0))
                            .text_color(theme::TEXT)
                    },
                )),
        )
        .child(
            view()
                .w(px(if compact { 190.0 } else { 300.0 }))
                .flex_shrink_0()
                .gap(px(4.0))
                .pl(px(20.0))
                .border_l_1()
                .border_color(theme::BORDER)
                .child(stat(
                    s.t(i18n::INDEXED_FILES),
                    categories::format_count(s.indexed.get()),
                    theme::TEXT,
                    "indexed-files",
                ))
                .child(stat(
                    s.t(i18n::DUPLICATE_GROUPS),
                    categories::format_count(s.summary.get().groups),
                    theme::TEXT,
                    "group-summary",
                ))
                .child(stat(
                    s.t(i18n::RECLAIMABLE_SPACE),
                    format::bytes(s.summary.get().wasted),
                    theme::ACCENT,
                    "duplicate-summary",
                )),
        )
}

fn stat(
    label: String,
    value: String,
    color: AdaptiveColor,
    test_id: &'static str,
) -> impl IntoElement + use<> {
    view()
        .flex_1()
        .justify_end()
        .gap(px(1.0))
        .pt(px(4.0))
        .border_t_1()
        .border_color(theme::BORDER)
        .child(text_line(
            label.to_uppercase(),
            8.0,
            FontWeight::NORMAL,
            theme::MUTED,
        ))
        .child(
            text(value)
                .font_family(theme::DISPLAY_FONT)
                .text_size(px(28.0))
                .text_color(color)
                .test_id(test_id),
        )
}

fn tab_bar(s: &State) -> impl IntoElement + use<> {
    let state = s.clone();
    view()
        .w_full()
        .flex_row()
        .border_t_1()
        .border_b_1()
        .border_color(theme::BORDER)
        .child(
            view()
                .flex_1()
                .flex_row()
                .bg(theme::SURFACE)
                .test_id("category-tabs")
                .children(TABS.iter().map(|category| tab(s, *category))),
        )
        .child(
            press("settings", false, move || state.settings.set(true))
                .w(px(44.0))
                .h_full()
                .items_center()
                .justify_center()
                .bg(theme::SURFACE)
                .border_l_1()
                .border_color(theme::BORDER)
                .child(text_line("⚙", 16.0, FontWeight::NORMAL, theme::TEXT)),
        )
}

fn tab(s: &State, category: Category) -> impl IntoElement + use<> {
    let counts = s.category_counts();
    let label = s.t(category.label_key());
    let label = if category == Category::Search {
        label
    } else {
        format!(
            "{} ({:02})",
            label,
            counts.get(&category).copied().unwrap_or(0)
        )
    };
    let state = s.clone();
    let selected = s.category.get() == category;
    press(format!("tab:{}", category.key_name()), false, move || {
        state.set_category(category)
    })
    .flex_1()
    .py(px(11.0))
    .items_center()
    .justify_center()
    .bg(if selected {
        theme::TEXT
    } else {
        theme::SURFACE
    })
    .border_r_1()
    .border_color(theme::BORDER)
    .child(text_line(
        label.to_uppercase(),
        8.0,
        FontWeight::BOLD,
        if selected {
            theme::SURFACE
        } else {
            theme::TEXT
        },
    ))
}

fn search_bar(s: &State) -> impl IntoElement + use<> {
    let state = s.clone();
    let category = s.category;
    when(
        move || category.get() == Category::Search,
        move || {
            let state = state.clone();
            view()
                .w_full()
                .px(px(20.0))
                .py(px(8.0))
                .bg(theme::SURFACE)
                .border_b_1()
                .border_color(theme::BORDER)
                .child(
                    input()
                        .test_id("search-input")
                        .value({
                            let state = state.clone();
                            move || state.query.get()
                        })
                        .on_change({
                            let state = state.clone();
                            move |value: &str| state.set_query(value)
                        })
                        .placeholder(state.t(i18n::SEARCH_PLACEHOLDER))
                        .h(px(34.0))
                        .px(px(10.0))
                        .py(px(6.0))
                        .bg(theme::BACKGROUND)
                        .border_1()
                        .border_color(theme::BORDER)
                        .font_family(theme::MONO_FONT)
                        .text_size(px(11.0))
                        .text_color(theme::TEXT)
                        .placeholder_color(theme::MUTED.light),
                )
        },
    )
}

fn body(s: &State, compact: bool) -> impl IntoElement + use<> {
    view()
        .flex_1()
        .w_full()
        .flex_row()
        .child(sidebar(s, compact))
        .child(main_panel(s))
}

fn sidebar(s: &State, compact: bool) -> impl IntoElement + use<> {
    let roots = s.scan_dirs;
    let state = s.clone();
    view()
        .w(px(if compact { 200.0 } else { 260.0 }))
        .flex_shrink_0()
        .bg(theme::SURFACE)
        .border_r_1()
        .border_color(theme::BORDER)
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .px(px(12.0))
                .py(px(9.0))
                .bg(theme::TEXT)
                .child(text_line(
                    s.t(i18n::SOURCE_FOLDERS).to_uppercase(),
                    9.0,
                    FontWeight::BOLD,
                    theme::SURFACE,
                ))
                .child(view().flex_1())
                .child(
                    press("all-folders", false, move || state.set_active_root(""))
                        .px(px(4.0))
                        .py(px(2.0))
                        .child(text_line(
                            s.t(i18n::ALL_FOLDERS).to_uppercase(),
                            7.0,
                            FontWeight::NORMAL,
                            theme::SURFACE,
                        )),
                )
                .child(text_line(
                    format!("({:02})", s.scan_dirs.get().len()),
                    9.0,
                    FontWeight::NORMAL,
                    theme::SURFACE,
                )),
        )
        .child(
            view().flex_1().test_id("source-folders").children(
                roots
                    .get()
                    .iter()
                    .enumerate()
                    .map(|(index, _)| folder_row(s, index))
                    .collect::<Vec<_>>(),
            ),
        )
        .child(when(move || !compact, {
            let state = s.clone();
            move || drop_zone(&state)
        }))
        .child(scan_button(s))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .px(px(12.0))
                .py(px(7.0))
                .border_t_1()
                .border_color(theme::BORDER)
                .child(text_line(
                    s.t(i18n::LOCAL_ONLY).to_uppercase(),
                    7.0,
                    FontWeight::NORMAL,
                    theme::MUTED,
                ))
                .child(view().flex_1()),
        )
}

fn folder_row(s: &State, index: usize) -> impl IntoElement + use<> {
    let roots = s.scan_dirs.get();
    let Some(root) = roots.get(index).cloned() else {
        return view();
    };
    let active = s.active_root.get() == root;
    let state = s.clone();
    let path = root.clone();
    let remove_state = s.clone();
    let remove_path = root.clone();
    let label = display_path(&root, &s.cfg.home);
    view()
        .flex_row()
        .items_center()
        .bg(if active {
            theme::SUCCESS_SOFT
        } else {
            theme::SURFACE
        })
        .border_b_1()
        .border_color(theme::BORDER)
        .child(
            press(format!("folder:{root}"), false, move || {
                if state.active_root.get() == path {
                    state.set_active_root("");
                } else {
                    state.set_active_root(path.clone());
                }
            })
            .flex_1()
            .flex_row()
            .items_center()
            .px(px(12.0))
            .py(px(10.0))
            .child(
                text_line(label, 9.0, FontWeight::NORMAL, theme::TEXT)
                    .truncate()
                    .flex_1(),
            ),
        )
        .child(when(
            move || active,
            move || {
                let state = remove_state.clone();
                let path = remove_path.clone();
                let press_state = state.clone();
                let press_path = path.clone();
                press(format!("remove-folder:{path}"), false, move || {
                    press_state.remove_folder(&press_path)
                })
                .px(px(6.0))
                .py(px(3.0))
                .border_1()
                .border_color(theme::BORDER)
                .child(text_line(
                    state.t(i18n::REMOVE).to_uppercase(),
                    7.0,
                    FontWeight::NORMAL,
                    theme::TEXT,
                ))
            },
        ))
}

fn drop_zone(s: &State) -> impl IntoElement + use<> {
    let state = s.clone();
    let picker = s.clone();
    let typer = s.clone();
    view()
        .test_id("drop-zone")
        .h(px(110.0))
        .m(px(12.0))
        .bg(theme::SURFACE)
        .border_1()
        .border_color(theme::BORDER)
        .rounded(px(999.0))
        .items_center()
        .justify_center()
        .gap(px(4.0))
        .child(
            press("add-folder", false, move || pick_folders(&picker))
                .px(px(8.0))
                .py(px(6.0))
                .items_center()
                .gap(px(4.0))
                .child(text_line("+ ", 16.0, FontWeight::NORMAL, theme::TEXT))
                .child(text_line(
                    state.t(i18n::DROP_DIRECTORY).to_uppercase(),
                    8.0,
                    FontWeight::BOLD,
                    theme::TEXT,
                ))
                .child(text_line(
                    state.t(i18n::BROWSE_HINT).to_uppercase(),
                    7.0,
                    FontWeight::NORMAL,
                    theme::MUTED,
                )),
        )
        .child(
            press("type-path", false, move || typer.open_add_folder())
                .px(px(8.0))
                .py(px(6.0))
                .child(text_line(
                    state.t(i18n::TYPE_PATH).to_uppercase(),
                    7.0,
                    FontWeight::NORMAL,
                    theme::MUTED,
                )),
        )
}

fn pick_folders(s: &State) {
    if s.scanning.get() || s.busy.get() {
        return;
    }
    let state = s.clone();
    let owner = s.owner;
    owner.run(|| {
        let _spawned = spawn(async move {
            let picked =
                pick_paths(nsh_core::file_dialog::OpenDialog::folders().multiple(true)).await;
            match picked {
                Ok(Some(paths)) => {
                    for path in paths {
                        state.add_folder(&path.to_string_lossy());
                    }
                }
                Ok(None) => {}
                Err(err) => state.message.set(err),
            }
        });
    });
}

fn scan_button(s: &State) -> impl IntoElement + use<> {
    let state = s.clone();
    let disabled = s.scanning.get() || s.busy.get() || s.scan_dirs.get().is_empty();
    press("scan-button", disabled, move || state.start_scan())
        .h(px(44.0))
        .items_center()
        .justify_center()
        .bg(theme::TEXT)
        .child(text_line(
            format!("{} ↗", s.t(i18n::RUN_GLOBAL_SCAN).to_uppercase()),
            9.0,
            FontWeight::BOLD,
            theme::SURFACE,
        ))
}

fn main_panel(s: &State) -> impl IntoElement + use<> {
    view()
        .flex_1()
        .px(px(16.0))
        .py(px(12.0))
        .gap(px(8.0))
        .child(view().flex_row().justify_end().child(review_button(s)))
        .child(banner(s))
        .child(when(
            {
                let state = s.clone();
                move || state.confirm.get()
            },
            {
                let state = s.clone();
                move || confirm_panel(&state)
            },
        ))
        .child(when(
            {
                let state = s.clone();
                move || !state.confirm.get()
            },
            {
                let state = s.clone();
                move || results(&state)
            },
        ))
        .child(note_text(s))
}

fn results(s: &State) -> impl IntoElement + use<> {
    let scanning = s.scanning;
    when(move || scanning.get(), {
        let state = s.clone();
        move || scanning_panel(&state)
    })
    .otherwise({
        let state = s.clone();
        move || groups_panel(&state)
    })
}

fn groups_panel(s: &State) -> impl IntoElement + use<> {
    let visible = s.visible;
    let state = s.clone();
    view()
        .flex_1()
        .child(
            virtual_list(
                move || visible.with(Vec::len),
                move |at| group_row(&state, at.index()),
            )
            .estimated_item_size(px(190.0))
            .flex_1()
            .test_id("duplicate-list"),
        )
        .child(empty_state(s))
}

fn empty_state(s: &State) -> impl IntoElement + use<> {
    let available = s.db.is_some();
    let title = if available {
        s.t(i18n::NO_DUPLICATES)
    } else {
        s.t(i18n::INDEX_UNAVAILABLE)
    };
    let body = if available {
        s.t(i18n::EMPTY_BODY)
    } else {
        s.t(i18n::CLOSE_INDEX_HOLDER)
    };
    let state = s.clone();
    when(
        move || state.visible.get().is_empty() && !state.scanning.get(),
        move || {
            let title = title.clone();
            let body = body.clone();
            let state = state.clone();
            view()
                .test_id("empty-state")
                .min_h(px(240.0))
                .items_center()
                .justify_center()
                .gap(px(8.0))
                .p(px(24.0))
                .bg(theme::SURFACE)
                .border_1()
                .border_color(theme::BORDER)
                .child(
                    text(title.to_uppercase())
                        .font_family(theme::DISPLAY_FONT)
                        .text_size(px(30.0))
                        .text_color(theme::TEXT),
                )
                .child(
                    text(body)
                        .font_family(theme::MONO_FONT)
                        .text_size(px(10.0))
                        .text_color(theme::MUTED),
                )
                .child(when(
                    move || available,
                    move || {
                        let press_state = state.clone();
                        press("empty-scan", false, move || {
                            if press_state
                                .add_folder(&press_state.cfg.default_dir.to_string_lossy())
                            {
                                press_state.start_scan();
                            }
                        })
                        .px(px(12.0))
                        .py(px(7.0))
                        .bg(theme::ACCENT)
                        .border_1()
                        .border_color(theme::TEXT)
                        .child(text_line(
                            state.t(i18n::SCAN_DEFAULT_FOLDER).to_uppercase(),
                            9.0,
                            FontWeight::BOLD,
                            theme::TEXT,
                        ))
                    },
                ))
        },
    )
}

fn group_row(s: &State, index: usize) -> impl IntoElement + use<> {
    let groups = s.visible.get();
    let Some(group) = groups.get(index).cloned() else {
        return view();
    };
    let state = s.clone();
    view()
        .flex_row()
        .p(px(10.0))
        .gap(px(12.0))
        .bg(theme::SURFACE)
        .border_1()
        .border_color(theme::BORDER)
        .child(preview(&group))
        .child(
            view().flex_1().justify_center().children(
                group
                    .files
                    .iter()
                    .enumerate()
                    .map(|(file_index, file)| file_row(&state, &group, file, file_index))
                    .collect::<Vec<_>>(),
            ),
        )
}

fn preview(group: &DuplicateGroup) -> impl IntoElement + use<> {
    let first = group.files.first().cloned();
    let name = first
        .as_ref()
        .map(|file| file.name.clone())
        .unwrap_or_default();
    let extension = std::path::Path::new(&name)
        .extension()
        .map(|ext| ext.to_string_lossy().to_uppercase())
        .unwrap_or_else(|| "FILE".to_string());
    let size = first
        .as_ref()
        .map(|file| format::bytes(file.size))
        .unwrap_or_default();
    let date = first
        .as_ref()
        .map(|file| file_date(file.mtime))
        .unwrap_or_default();
    let (a, b) = gradient(&group.md5);
    view()
        .w(px(180.0))
        .flex_shrink_0()
        .gap(px(5.0))
        .child(
            view()
                .w_full()
                .h(px(116.0))
                .border_1()
                .border_color(theme::BORDER)
                .bg_linear(deg(135.0))
                .gradient_from(a)
                .gradient_to(b),
        )
        .child(
            text(name)
                .font_family(theme::MONO_FONT)
                .text_size(px(10.0))
                .font_weight(FontWeight::BOLD)
                .text_color(theme::TEXT)
                .truncate(),
        )
        .child(
            view()
                .flex_row()
                .gap(px(4.0))
                .child(tag(extension, theme::ACCENT))
                .child(tag(size, theme::SURFACE))
                .child(tag(date, theme::SURFACE)),
        )
}

fn tag(value: String, background: AdaptiveColor) -> impl IntoElement + use<> {
    view()
        .px(px(4.0))
        .py(px(2.0))
        .bg(background)
        .border_1()
        .border_color(theme::BORDER)
        .child(text_line(value, 7.0, FontWeight::BOLD, theme::TEXT))
}

fn gradient(md5: &str) -> (AdaptiveColor, AdaptiveColor) {
    let palettes: [&[u32]; 5] = [
        &[0x2E1065, 0x9333EA, 0xF59E0B],
        &[0x0F2027, 0x203A43, 0x2C5364],
        &[0xFF512F, 0xDD2476, 0xFF8A00],
        &[0x11998E, 0x38EF7D],
        &[0x1E3A8A, 0x3B82F6, 0xF472B6],
    ];
    let mut index = 0usize;
    for byte in md5.bytes().take(3) {
        index = index * 16 + (byte % 16) as usize;
    }
    let palette = palettes[index % palettes.len()];
    (adaptive(palette[0]), adaptive(palette[palette.len() - 1]))
}

fn adaptive(hex: u32) -> AdaptiveColor {
    AdaptiveColor {
        light: rgb(hex).to_hsla(),
        dark: rgb(hex).to_hsla(),
    }
}

fn file_date(mtime: i64) -> String {
    let (_, month, day) = time_from_unix(mtime);
    let months = [
        "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
    ];
    let month = months[(month as usize).saturating_sub(1).min(11)];
    format!("{month} {day:02}")
}

/// A civil date from a Unix timestamp: `(year, month, day)`.
fn time_from_unix(seconds: i64) -> (i64, i64, i64) {
    let days = seconds.div_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    (year, m, d)
}

fn file_row(
    s: &State,
    group: &DuplicateGroup,
    file: &onley_core::db::FileRecord,
    index: usize,
) -> impl IntoElement + use<> {
    let checked = s.keep.get().get(&group.md5) == Some(&file.path.to_string_lossy().into_owned());
    let state = s.clone();
    let md5 = group.md5.clone();
    let path = file.path.to_string_lossy().into_owned();
    let self_path = s.cfg.self_path.clone();
    let has_self = appmodel::protected_file(group, self_path.as_deref()).is_some();
    let is_self = self_path
        .as_ref()
        .is_some_and(|me| onley_core::selfpath::same(me, &file.path));
    let disabled = (has_self && !is_self) || s.busy.get() || s.scanning.get();
    let label = if index == 0 {
        s.t(i18n::ORIGINAL_COPY)
    } else {
        s.tf(i18n::DUPLICATE_N, &[&format!("{index:02}")])
    };
    let status = if checked {
        s.t(i18n::KEEP_BADGE)
    } else {
        s.t(i18n::DELETE_BADGE)
    };
    let display = display_path(&file.path.to_string_lossy(), &s.cfg.home);
    let file_dir_path = file.path.to_string_lossy().into_owned();
    press(format!("keep:{path}"), disabled, move || {
        if !has_self || is_self {
            state.set_keep(&md5, &path);
        }
    })
    .w_full()
    .flex_row()
    .items_center()
    .gap(px(8.0))
    .py(px(7.0))
    .px(px(6.0))
    .bg(if checked {
        theme::SUCCESS_SOFT
    } else {
        theme::SURFACE
    })
    .border_b_1()
    .border_color(theme::BORDER)
    .child(
        view()
            .flex_1()
            .gap(px(1.0))
            .child(text_line(
                label.to_uppercase(),
                8.0,
                FontWeight::BOLD,
                theme::TEXT,
            ))
            .child(
                text_line(display, 8.0, FontWeight::NORMAL, theme::MUTED)
                    .truncate()
                    .test_id(&format!("file-dir:{file_dir_path}")),
            ),
    )
    .child(text_line(
        status.to_uppercase(),
        8.0,
        FontWeight::BOLD,
        theme::TEXT,
    ))
    .child(status_box(checked))
}

fn status_box(checked: bool) -> impl IntoElement + use<> {
    view()
        .w(px(16.0))
        .h(px(16.0))
        .border_1()
        .border_color(theme::TEXT)
        .bg(if checked {
            theme::SUCCESS_SOFT
        } else {
            theme::SURFACE
        })
        .items_center()
        .justify_center()
        .child(if checked {
            text_line("✓", 10.0, FontWeight::BOLD, theme::TEXT)
        } else {
            text_line("", 10.0, FontWeight::NORMAL, theme::TEXT)
        })
}

fn bottom_bar(s: &State) -> impl IntoElement + use<> {
    let state = s.clone();
    view()
        .test_id("bottom-bar")
        .flex_row()
        .items_center()
        .gap(px(14.0))
        .px(px(20.0))
        .py(px(7.0))
        .bg(theme::TEXT)
        .child(text_line(
            s.t(i18n::AUTO_SELECT_POLICY).to_uppercase(),
            8.0,
            FontWeight::BOLD,
            theme::SURFACE,
        ))
        .child(policy_toggle(
            s,
            i18n::KEEP_NEWEST,
            "policy-newest",
            appmodel::KeepPolicy::Newest,
        ))
        .child(policy_toggle(
            s,
            i18n::KEEP_OLDEST,
            "policy-oldest",
            appmodel::KeepPolicy::Oldest,
        ))
        .child(policy_toggle(
            s,
            i18n::KEEP_BY_PATH,
            "policy-path",
            appmodel::KeepPolicy::ByPath,
        ))
        .child(
            press(
                "clear-choices",
                s.keep.get().is_empty() || s.scanning.get() || s.busy.get(),
                move || state.clear_keep(),
            )
            .px(px(6.0))
            .py(px(4.0))
            .child(text_line(
                s.t(i18n::CLEAR).to_uppercase(),
                8.0,
                FontWeight::NORMAL,
                theme::SURFACE,
            )),
        )
        .child(view().flex_1())
}

fn policy_toggle(
    s: &State,
    key: &'static str,
    test_id: &'static str,
    policy: appmodel::KeepPolicy,
) -> impl IntoElement + use<> {
    let state = s.clone();
    let selected = s.policy_selected(policy);
    press(
        test_id,
        s.scanning.get() || s.busy.get() || s.groups.get().is_empty(),
        move || state.apply_keep_policy(policy),
    )
    .flex_row()
    .items_center()
    .gap(px(6.0))
    .px(px(4.0))
    .py(px(4.0))
    .child(status_box(selected))
    .child(text_line(
        s.t(key).to_uppercase(),
        8.0,
        FontWeight::BOLD,
        theme::SURFACE,
    ))
}

fn review_button(s: &State) -> impl IntoElement + use<> {
    let state = s.clone();
    let label = s.review_label();
    press(
        "delete-files",
        s.plan().targets.is_empty() || s.scanning.get() || s.busy.get(),
        move || state.open_confirm(),
    )
    .px(px(16.0))
    .py(px(8.0))
    .items_center()
    .bg(theme::ACCENT)
    .border_1()
    .border_color(theme::TEXT)
    .child(
        text(label)
            .font_family(theme::DISPLAY_FONT)
            .text_size(px(20.0))
            .text_color(theme::TEXT)
            .truncate(),
    )
}

fn banner(s: &State) -> impl IntoElement + use<> {
    let state = s.clone();
    when(
        move || !state.message.get().is_empty(),
        move || {
            let state = state.clone();
            let dismiss = state.clone();
            view()
                .flex_row()
                .items_center()
                .gap(px(10.0))
                .px(px(10.0))
                .py(px(7.0))
                .bg(theme::ACCENT_SOFT)
                .border_b_1()
                .border_color(theme::ACCENT)
                .child(view().flex_1().child(text_line_dyn(
                    {
                        let state = state.clone();
                        move || state.message.get()
                    },
                    10.0,
                    FontWeight::NORMAL,
                    theme::TEXT,
                )))
                .child(app_button(
                    state.t(i18n::DISMISS),
                    "dismiss-error",
                    ButtonKind::Secondary,
                    false,
                    move || dismiss.message.set(String::new()),
                ))
        },
    )
}

fn scanning_panel(s: &State) -> impl IntoElement + use<> {
    let progress = s.progress.get();
    let state = s.clone();
    view()
        .flex_1()
        .items_center()
        .justify_center()
        .gap(px(10.0))
        .p(px(18.0))
        .bg(theme::SURFACE)
        .border_1()
        .border_color(theme::BORDER)
        .child(
            text(s.t(i18n::SCANNING).to_uppercase())
                .font_family(theme::DISPLAY_FONT)
                .text_size(px(34.0))
                .text_color(theme::TEXT),
        )
        .child(
            text_line(s.scan_roots_label(), 9.0, FontWeight::NORMAL, theme::MUTED)
                .test_id("scan-root"),
        )
        .child(gauge::view(s))
        .child(
            text(speed_text(progress.speed))
                .font_family(theme::DISPLAY_FONT)
                .text_size(px(32.0))
                .text_color(theme::TEXT)
                .test_id("scan-speed"),
        )
        .child(text_line(
            s.tf(
                i18n::SCAN_STATS,
                &[
                    &format::bytes(progress.hashed),
                    &s.file_count(progress.done),
                    &progress.skipped,
                    &progress.errors,
                ],
            ),
            9.0,
            FontWeight::NORMAL,
            theme::MUTED,
        ))
        .child(text_line(
            s.tf(i18n::ELAPSED, &[&format_duration(progress.elapsed)]),
            9.0,
            FontWeight::NORMAL,
            theme::MUTED,
        ))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(10.0))
                .child(app_button(
                    s.t(i18n::CANCEL_SCAN),
                    "cancel-scan",
                    ButtonKind::Secondary,
                    false,
                    move || state.cancel_scan(),
                ))
                .child(text_line(
                    s.t(i18n::CANCEL_SCAN_HINT),
                    9.0,
                    FontWeight::NORMAL,
                    theme::MUTED,
                )),
        )
}

fn note_text(s: &State) -> impl IntoElement + use<> {
    let state = s.clone();
    when(
        move || !state.note.get().is_empty(),
        move || {
            let color = if state.note_success.get() {
                theme::SUCCESS
            } else {
                theme::MUTED
            };
            text_line_dyn(
                {
                    let state = state.clone();
                    move || state.note.get()
                },
                9.0,
                FontWeight::NORMAL,
                color,
            )
            .test_id("note")
        },
    )
}

fn add_folder_dialog(s: &State) -> impl IntoElement + use<> {
    let state = s.clone();
    when(
        move || state.adding_folder.get(),
        move || {
            let state = state.clone();
            let close = state.clone();
            let paste = state.clone();
            let confirm = state.clone();
            let cancel = state.clone();
            view()
                .absolute()
                .inset(px(0.0))
                .w_full()
                .h_full()
                .items_center()
                .justify_center()
                .child(
                    view()
                        .absolute()
                        .inset(px(0.0))
                        .w_full()
                        .h_full()
                        .bg(theme::OVERLAY)
                        .on_press(move || close.close_add_folder()),
                )
                .child(
                    view()
                        .test_id("add-folder-dialog")
                        .w(px(500.0))
                        .bg(theme::SURFACE)
                        .border_2()
                        .border_color(theme::BORDER)
                        .p(px(20.0))
                        .gap(px(10.0))
                        .child(
                            text(state.t(i18n::ADD_FOLDER).to_uppercase())
                                .font_family(theme::DISPLAY_FONT)
                                .text_size(px(26.0))
                                .text_color(theme::TEXT),
                        )
                        .child(
                            view()
                                .flex_row()
                                .items_center()
                                .gap(px(8.0))
                                .child(
                                    input()
                                        .test_id("scan-path")
                                        .flex_1()
                                        .h(px(36.0))
                                        .px(px(9.0))
                                        .py(px(7.0))
                                        .bg(theme::BACKGROUND)
                                        .border_1()
                                        .border_color(theme::BORDER)
                                        .font_family(theme::MONO_FONT)
                                        .text_size(px(10.0))
                                        .text_color(theme::TEXT)
                                        .placeholder_color(theme::MUTED.light)
                                        .placeholder(state.t(i18n::ADD_FOLDER))
                                        .value({
                                            let state = state.clone();
                                            move || state.scan_path.get()
                                        })
                                        .on_change({
                                            let state = state.clone();
                                            move |value: &str| state.set_scan_path(value)
                                        })
                                        .on_submit({
                                            let state = state.clone();
                                            move |_: &str| state.confirm_add_folder()
                                        }),
                                )
                                .child(app_button(
                                    state.t(i18n::PASTE_PATH),
                                    "paste-path",
                                    ButtonKind::Secondary,
                                    false,
                                    move || paste.paste_path(),
                                )),
                        )
                        .child(when(
                            {
                                let state = state.clone();
                                move || state.path_state.get().status != PathStatus::Unchecked
                            },
                            {
                                let state = state.clone();
                                move || {
                                    text_line_dyn(
                                        {
                                            let state = state.clone();
                                            move || state.path_hint()
                                        },
                                        9.0,
                                        FontWeight::NORMAL,
                                        theme::DANGER,
                                    )
                                    .text_color({
                                        let state = state.clone();
                                        move || {
                                            if state.path_state.get().status == PathStatus::Usable {
                                                theme::SUCCESS
                                            } else {
                                                theme::DANGER
                                            }
                                        }
                                    })
                                    .test_id("path-status")
                                }
                            },
                        ))
                        .child(recent_folders(&state))
                        .child(when(
                            {
                                let state = state.clone();
                                move || !state.message.get().is_empty()
                            },
                            {
                                let state = state.clone();
                                move || {
                                    text_line_dyn(
                                        {
                                            let state = state.clone();
                                            move || state.message.get()
                                        },
                                        9.0,
                                        FontWeight::NORMAL,
                                        theme::ACCENT,
                                    )
                                    .test_id("add-folder-error")
                                }
                            },
                        ))
                        .child(
                            view()
                                .flex_row()
                                .justify_end()
                                .gap(px(8.0))
                                .child(app_button(
                                    state.t(i18n::CANCEL),
                                    "cancel-add-folder",
                                    ButtonKind::Secondary,
                                    false,
                                    move || cancel.close_add_folder(),
                                ))
                                .child(app_button(
                                    state.t(i18n::ADD),
                                    "add-folder-confirm",
                                    ButtonKind::Primary,
                                    false,
                                    move || confirm.confirm_add_folder(),
                                )),
                        ),
                )
        },
    )
}

fn recent_folders(s: &State) -> impl IntoElement + use<> {
    let state = s.clone();
    when(
        move || !state.recent.get().is_empty(),
        move || {
            let state = state.clone();
            view()
                .gap(px(2.0))
                .child(text_line(
                    state.t(i18n::RECENT_FOLDERS).to_uppercase(),
                    8.0,
                    FontWeight::BOLD,
                    theme::MUTED,
                ))
                .children(
                    state
                        .recent
                        .get()
                        .into_iter()
                        .map(|path| recent_row(&state, path))
                        .collect::<Vec<_>>(),
                )
        },
    )
}

fn recent_row(s: &State, path: String) -> impl IntoElement + use<> {
    let state = s.clone();
    let display = display_path(&path, &s.cfg.home);
    let value = path.clone();
    press(format!("recent-folder:{path}"), false, move || {
        state.set_scan_path(value.clone())
    })
    .px(px(6.0))
    .py(px(4.0))
    .child(
        text_line(display, 9.0, FontWeight::NORMAL, theme::TEXT)
            .truncate()
            .flex_1(),
    )
}

fn settings_dialog(s: &State) -> impl IntoElement + use<> {
    let state = s.clone();
    when(
        move || state.settings.get(),
        move || {
            let state = state.clone();
            let close = state.clone();
            let close_button = state.clone();
            view()
                .absolute()
                .inset(px(0.0))
                .w_full()
                .h_full()
                .items_center()
                .justify_center()
                .child(
                    view()
                        .absolute()
                        .inset(px(0.0))
                        .w_full()
                        .h_full()
                        .bg(theme::OVERLAY)
                        .on_press(move || close.settings.set(false)),
                )
                .child(
                    view()
                        .test_id("settings-dialog")
                        .w(px(420.0))
                        .bg(theme::SURFACE)
                        .border_2()
                        .border_color(theme::BORDER)
                        .p(px(20.0))
                        .gap(px(12.0))
                        .child(
                            text(state.t(i18n::SETTINGS).to_uppercase())
                                .font_family(theme::DISPLAY_FONT)
                                .text_size(px(26.0))
                                .text_color(theme::TEXT),
                        )
                        .child(text_line(
                            state.t(i18n::LANGUAGE).to_uppercase(),
                            8.0,
                            FontWeight::BOLD,
                            theme::MUTED,
                        ))
                        .child(
                            view()
                                .flex_row()
                                .gap(px(8.0))
                                .child(language_option(
                                    &state,
                                    "English",
                                    i18n::Lang::English,
                                    "lang-en",
                                ))
                                .child(language_option(
                                    &state,
                                    "简体中文",
                                    i18n::Lang::Chinese,
                                    "lang-zh",
                                )),
                        )
                        .child(view().flex_row().justify_end().child(app_button(
                            state.t(i18n::CLOSE),
                            "settings-close",
                            ButtonKind::Secondary,
                            false,
                            move || close_button.settings.set(false),
                        ))),
                )
        },
    )
}

fn language_option(
    s: &State,
    label: &'static str,
    lang: i18n::Lang,
    test_id: &'static str,
) -> impl IntoElement + use<> {
    let state = s.clone();
    let selected = s.lang.get() == lang;
    press(test_id, false, move || state.set_language(lang))
        .flex_1()
        .items_center()
        .justify_center()
        .px(px(12.0))
        .py(px(9.0))
        .bg(if selected {
            theme::TEXT
        } else {
            theme::SURFACE
        })
        .border_1()
        .border_color(theme::BORDER)
        .child(text_line(
            label,
            10.0,
            FontWeight::BOLD,
            if selected {
                theme::SURFACE
            } else {
                theme::TEXT
            },
        ))
}

fn confirm_panel(s: &State) -> impl IntoElement + use<> {
    let state = s.clone();
    let plan = state.plan();
    let mut targets: Vec<String> = plan
        .targets
        .iter()
        .take(5)
        .map(|file| file.path.to_string_lossy().into_owned())
        .collect();
    if plan.targets.len() > 5 {
        targets.push(format!(
            "+ {}",
            state.tf(i18n::AND_MORE, &[&(plan.targets.len() - 5)])
        ));
    }
    let cancel = state.clone();
    let delete = state.clone();
    view()
        .test_id("confirm-dialog")
        .w_full()
        .max_w(px(560.0))
        .bg(theme::SURFACE)
        .border_2()
        .border_color(theme::BORDER)
        .p(px(20.0))
        .gap(px(10.0))
        .child(
            text(state.t(i18n::DELETE_DUPLICATES).to_uppercase())
                .font_family(theme::DISPLAY_FONT)
                .text_size(px(30.0))
                .text_color(theme::TEXT),
        )
        .child(text_line(
            state.tf(
                i18n::DELETE_CONFIRM,
                &[
                    &state.file_count(plan.targets.len()),
                    &format::bytes(plan.bytes),
                ],
            ),
            11.0,
            FontWeight::NORMAL,
            theme::TEXT,
        ))
        .child(text_line(
            state.t(i18n::CANNOT_UNDO),
            9.0,
            FontWeight::NORMAL,
            theme::MUTED,
        ))
        .children(
            targets
                .into_iter()
                .map(|path| text_line(path, 9.0, FontWeight::NORMAL, theme::MUTED))
                .collect::<Vec<_>>(),
        )
        .child(
            view()
                .flex_row()
                .justify_end()
                .gap(px(8.0))
                .child(app_button(
                    state.t(i18n::CANCEL),
                    "cancel-delete",
                    ButtonKind::Secondary,
                    false,
                    move || cancel.confirm.set(false),
                ))
                .child(app_button(
                    state.t(i18n::DELETE_PERMANENTLY),
                    "delete-confirm",
                    ButtonKind::Danger,
                    false,
                    move || delete.confirm_delete(),
                )),
        )
}

fn app_button(
    label: String,
    test_id: &'static str,
    kind: ButtonKind,
    disabled: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let (background, foreground) = match kind {
        ButtonKind::Primary | ButtonKind::Danger => (theme::ACCENT, theme::TEXT),
        ButtonKind::Secondary => (theme::SURFACE, theme::TEXT),
    };
    press(test_id, disabled, on_press)
        .px(px(12.0))
        .py(px(7.0))
        .items_center()
        .justify_center()
        .bg(background)
        .border_1()
        .border_color(theme::TEXT)
        .child(text_line(
            label.to_uppercase(),
            9.0,
            FontWeight::BOLD,
            foreground,
        ))
}

fn speed_text(bytes_per_second: f64) -> String {
    format!("{}/s", format::bytes(bytes_per_second as i64))
}

fn format_duration(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}:{:02}", secs / 60, secs % 60)
    } else {
        format!("{}:{:02}:{:02}", secs / 3600, (secs / 60) % 60, secs % 60)
    }
}

impl State {
    /// Reports whether the policy is the one every visible group follows.
    pub fn policy_selected(&self, policy: appmodel::KeepPolicy) -> bool {
        if self.keep_policy.get() != policy {
            return false;
        }
        let groups = self.scoped.get();
        if groups.is_empty() {
            return false;
        }
        let keep: HashMap<String, String> = self.keep.get();
        for group in &groups {
            let Some(path) = keep.get(&group.md5) else {
                return false;
            };
            if let Some(protected) = appmodel::protected_file(group, self.cfg.self_path.as_deref())
            {
                if !onley_core::selfpath::same(&protected.path, std::path::Path::new(path)) {
                    return false;
                }
                continue;
            }
            if appmodel::choose_keep(group, policy)
                .is_none_or(|chosen| chosen.path.to_string_lossy() != *path)
            {
                return false;
            }
        }
        true
    }

    /// The label of the review button.
    pub fn review_label(&self) -> String {
        self.tf(
            i18n::REVIEW_DELETE_N,
            &[&self.file_count(self.plan().targets.len())],
        )
        .to_uppercase()
    }
}
