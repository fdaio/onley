//! The phone screen.

use crate::model::{self, Group, KeepPolicy};
use crate::state::State;
use nsh_core::{
    element::{AccessibilityRole, AdaptiveColor, FontWeight, rgb, rgba},
    geometry::px,
    prelude::*,
};

const BACKGROUND: AdaptiveColor = tone(0xF4F1EB);
const SURFACE: AdaptiveColor = tone(0xFFFFFF);
const BORDER: AdaptiveColor = tone(0x111111);
const TEXT: AdaptiveColor = tone(0x111111);
const MUTED: AdaptiveColor = tone(0x6B6B6B);
const ACCENT: AdaptiveColor = tone(0xFF00A0);
const ACCENT_SOFT: AdaptiveColor = tone_a(0xFF00A0, 0.12);
const SUCCESS: AdaptiveColor = tone(0x1F6B3A);
const OVERLAY: AdaptiveColor = tone_a(0x111111, 0.55);

const fn tone(hex: u32) -> AdaptiveColor {
    AdaptiveColor {
        light: rgb(hex).to_hsla(),
        dark: rgb(hex).to_hsla(),
    }
}

const fn tone_a(hex: u32, alpha: f32) -> AdaptiveColor {
    let value = rgba(hex << 8 | (alpha * 255.0 + 0.5) as u32);
    AdaptiveColor {
        light: value.to_hsla(),
        dark: value.to_hsla(),
    }
}

pub fn root(s: &State) -> impl IntoElement + use<> {
    let state = s.clone();
    view().flex_1().w_full().h_full().child(dynamic(move || {
        let s = state.clone();
        let _ = s.lang.get();
        let _ = s.access.get();
        let _ = s.scanning.get();
        let _ = s.progress.get();
        let _ = s.groups.get();
        let _ = s.keep.get();
        let _ = s.policy.get();
        let _ = s.summary.get();
        let _ = s.message.get();
        let _ = s.note.get();
        let _ = s.note_success.get();
        let _ = s.confirm.get();
        let _ = s.busy.get();
        main_view(&s)
    }))
}

fn main_view(s: &State) -> impl IntoElement + use<> {
    view()
        .flex_1()
        .w_full()
        .h_full()
        .bg(BACKGROUND)
        .p_safe_area()
        .test_id("root")
        .child(
            view()
                .flex_1()
                .w_full()
                .h_full()
                .child(header(s))
                .child(summary_card(s))
                .child(access_banner(s))
                .child(message_banner(s))
                .child(progress_card(s))
                .child(content(s))
                .child(note_line(s))
                .child(bottom_bar(s)),
        )
        .child(confirm_overlay(s))
}

fn press(id: impl Into<String>, disabled: bool, on_press: impl Fn() + 'static) -> View {
    view()
        .test_id(&id.into())
        .disabled(disabled)
        .role(AccessibilityRole::Button)
        .on_press(on_press)
}

fn label(value: &str, size: f32, weight: FontWeight, color: AdaptiveColor) -> Text {
    text(value.to_owned())
        .text_size(px(size))
        .font_weight(weight)
        .text_color(color)
}

fn header(s: &State) -> impl IntoElement + use<> {
    let t = s.t();
    let state = s.clone();
    view()
        .flex_row()
        .w_full()
        .items_center()
        .justify_between()
        .px(px(20.0))
        .pt(px(10.0))
        .pb(px(14.0))
        .child(
            view()
                .gap(px(2.0))
                .child(label(t.app_name, 28.0, FontWeight::BOLD, TEXT))
                .child(label(t.tagline, 12.0, FontWeight::NORMAL, MUTED)),
        )
        .child(
            press("language", false, move || state.toggle_lang())
                .px(px(12.0))
                .py(px(7.0))
                .border_1()
                .border_color(BORDER)
                .rounded(px(999.0))
                .child(label(s.lang.get().label(), 11.0, FontWeight::BOLD, TEXT)),
        )
}

fn summary_card(s: &State) -> impl IntoElement + use<> {
    let t = s.t();
    let summary = s.summary.get();
    view()
        .mx(px(20.0))
        .p(px(16.0))
        .rounded(px(16.0))
        .bg(SURFACE)
        .border_1()
        .border_color(BORDER)
        .flex_row()
        .items_center()
        .gap(px(12.0))
        .child(stat(summary.groups.to_string(), t.groups))
        .child(divider())
        .child(stat(summary.copies.to_string(), t.copies))
        .child(divider())
        .child(stat(model::bytes(summary.reclaimable), t.reclaimable))
}

fn stat(value: String, caption: &'static str) -> impl IntoElement {
    view()
        .flex_1()
        .gap(px(2.0))
        .child(label(&value, 20.0, FontWeight::BOLD, TEXT))
        .child(label(caption, 10.0, FontWeight::NORMAL, MUTED))
}

fn divider() -> impl IntoElement {
    view().w(px(1.0)).h(px(28.0)).bg(BORDER)
}

fn access_banner(s: &State) -> impl IntoElement + use<> {
    let text = s.access_message();
    let state = s.clone();
    let t = s.t();
    let can_ask =
        !s.access.get().can_read() && s.access.get() != crate::source::Access::Unsupported;
    let has_text = !text.is_empty();
    when(
        move || has_text,
        move || {
            let text = text.clone();
            let grant = state.clone();
            view()
                .mx(px(20.0))
                .mt(px(10.0))
                .p(px(14.0))
                .rounded(px(14.0))
                .bg(ACCENT_SOFT)
                .border_1()
                .border_color(ACCENT)
                .gap(px(10.0))
                .child(label(&text, 12.0, FontWeight::NORMAL, TEXT))
                .child(when(
                    move || can_ask,
                    move || {
                        let grant = grant.clone();
                        action_button(t.grant_access, "grant", true, false, move || grant.scan())
                    },
                ))
        },
    )
}

fn message_banner(s: &State) -> impl IntoElement + use<> {
    let message = s.message.get();
    let has_message = !message.is_empty();
    when(
        move || has_message,
        move || {
            let message = message.clone();
            view()
                .mx(px(20.0))
                .mt(px(10.0))
                .p(px(12.0))
                .rounded(px(12.0))
                .bg(SURFACE)
                .border_1()
                .border_color(BORDER)
                .child(label(&message, 11.0, FontWeight::NORMAL, MUTED))
        },
    )
}

fn progress_card(s: &State) -> impl IntoElement + use<> {
    let scanning = s.scanning.get();
    let progress = s.progress.get();
    let state = s.clone();
    let t = s.t();
    let current = progress.current.clone();
    let has_current = !current.is_empty();
    when(
        move || scanning,
        move || {
            let state = state.clone();
            let current = current.clone();
            let fraction = if progress.total == 0 {
                0.0
            } else {
                progress.done as f32 / progress.total as f32
            };
            view()
                .mx(px(20.0))
                .mt(px(10.0))
                .p(px(14.0))
                .rounded(px(14.0))
                .bg(SURFACE)
                .border_1()
                .border_color(BORDER)
                .gap(px(8.0))
                .child(
                    view()
                        .flex_row()
                        .justify_between()
                        .child(label(t.scanning, 11.0, FontWeight::BOLD, TEXT))
                        .child(label(
                            &format!("{} / {}", progress.done, progress.total),
                            11.0,
                            FontWeight::BOLD,
                            MUTED,
                        )),
                )
                .child(
                    view()
                        .h(px(6.0))
                        .w(px(320.0))
                        .bg(ACCENT_SOFT)
                        .rounded(px(999.0))
                        .child(
                            view()
                                .h(px(6.0))
                                .w(px(320.0 * fraction.clamp(0.0, 1.0)))
                                .bg(ACCENT)
                                .rounded(px(999.0)),
                        ),
                )
                .child(when(
                    move || has_current,
                    move || {
                        let current = current.clone();
                        label(&current, 10.0, FontWeight::NORMAL, MUTED)
                    },
                ))
                .child(action_button(t.cancel, "cancel", false, false, move || {
                    state.cancel_scan()
                }))
        },
    )
}

fn content(s: &State) -> impl IntoElement + use<> {
    let state = s.clone();
    let empty = s.clone();
    view()
        .flex_1()
        .w_full()
        .child(when(
            move || state.groups.get().is_empty(),
            move || empty_state(&empty),
        ))
        .child(when(move || !state.groups.get().is_empty(), {
            let list = state.clone();
            move || group_list(&list, list.groups.get())
        }))
}

fn empty_state(s: &State) -> impl IntoElement + use<> {
    let t = s.t();
    view()
        .flex_1()
        .w_full()
        .items_center()
        .justify_center()
        .px(px(36.0))
        .gap(px(10.0))
        .child(label(t.no_duplicates, 20.0, FontWeight::BOLD, TEXT))
        .child(label(t.empty_hint, 12.0, FontWeight::NORMAL, MUTED))
}

fn group_list(s: &State, groups: Vec<Group>) -> impl IntoElement + use<> {
    let state = s.clone();
    scroll().flex_1().w_full().child(
        view().px(px(20.0)).py(px(12.0)).gap(px(12.0)).children(
            groups
                .into_iter()
                .map(move |group| group_card(&state, group)),
        ),
    )
}

fn group_card(s: &State, group: Group) -> impl IntoElement + use<> {
    let state = s.clone();
    let t = s.t();
    let kept = s.kept(&group).unwrap_or_default();
    let hash = group.hash.clone();
    view()
        .p(px(14.0))
        .rounded(px(16.0))
        .bg(SURFACE)
        .border_1()
        .border_color(BORDER)
        .gap(px(10.0))
        .child(
            view()
                .flex_row()
                .justify_between()
                .items_center()
                .child(label(
                    &format!("{} {}", group.assets.len(), t.copies),
                    14.0,
                    FontWeight::BOLD,
                    TEXT,
                ))
                .child(label(
                    &model::bytes(group.reclaimable()),
                    14.0,
                    FontWeight::BOLD,
                    ACCENT,
                )),
        )
        .child(
            view().gap(px(8.0)).children(
                group
                    .assets
                    .into_iter()
                    .map(move |asset| asset_row(&state, &hash, asset, &kept)),
            ),
        )
}

fn asset_row(
    s: &State,
    hash: &str,
    asset: crate::model::Asset,
    kept: &str,
) -> impl IntoElement + use<> {
    let state = s.clone();
    let t = s.t();
    let selected = asset.id == kept;
    let hash = hash.to_owned();
    let id = asset.id.clone();
    let size = asset.size;
    let width = asset.width;
    let height = asset.height;
    let modified = asset.modified;
    let name = asset.name.clone();
    press(format!("keep-{}", asset.id), false, move || {
        state.set_keep(&hash, &id)
    })
    .flex_row()
    .items_center()
    .gap(px(10.0))
    .p(px(10.0))
    .rounded(px(12.0))
    .bg(if selected { ACCENT_SOFT } else { BACKGROUND })
    .border_1()
    .border_color(if selected { ACCENT } else { BORDER })
    .child(
        view()
            .w(px(16.0))
            .h(px(16.0))
            .rounded(px(999.0))
            .border_2()
            .border_color(if selected { ACCENT } else { MUTED })
            .bg(if selected { ACCENT } else { SURFACE }),
    )
    .child(
        view()
            .flex_1()
            .gap(px(2.0))
            .child(label(
                &name,
                13.0,
                if selected {
                    FontWeight::BOLD
                } else {
                    FontWeight::NORMAL
                },
                TEXT,
            ))
            .child(label(
                &details(size, width, height, modified),
                10.0,
                FontWeight::NORMAL,
                MUTED,
            )),
    )
    .child(label(
        if selected { t.kept } else { t.keep },
        9.0,
        FontWeight::BOLD,
        if selected { ACCENT } else { MUTED },
    ))
}

fn details(size: i64, width: u32, height: u32, modified: i64) -> String {
    let mut parts = vec![model::bytes(size)];
    if width > 0 && height > 0 {
        parts.push(format!("{width}×{height}"));
    }
    parts.push(date(modified));
    parts.join(" · ")
}

fn note_line(s: &State) -> impl IntoElement + use<> {
    let note = s.note.get();
    let success = s.note_success.get();
    let has_note = !note.is_empty();
    when(
        move || has_note,
        move || {
            let note = note.clone();
            view().px(px(20.0)).py(px(6.0)).child(label(
                &note,
                11.0,
                FontWeight::NORMAL,
                if success { SUCCESS } else { ACCENT },
            ))
        },
    )
}

fn bottom_bar(s: &State) -> impl IntoElement + use<> {
    let t = s.t();
    let state = s.clone();
    let scanning = s.scanning.get();
    let busy = s.busy.get();
    let targets = s.targets().len();
    let policy = s.policy.get();
    let newest = state.clone();
    let oldest = state.clone();
    let action = state.clone();
    let (text, id, primary) = if scanning {
        (t.cancel.to_owned(), "cancel-action", false)
    } else if targets > 0 {
        (
            format!("{} {} {}", t.delete_copies, targets, t.copies),
            "delete",
            true,
        )
    } else {
        (t.scan.to_owned(), "scan", true)
    };
    let on_press: Box<dyn Fn()> = if scanning {
        Box::new(move || action.cancel_scan())
    } else if targets > 0 {
        Box::new(move || action.ask_delete())
    } else {
        Box::new(move || action.scan())
    };
    view()
        .w_full()
        .px(px(20.0))
        .py(px(12.0))
        .border_t_1()
        .border_color(BORDER)
        .bg(BACKGROUND)
        .gap(px(10.0))
        .child(
            view()
                .flex_row()
                .gap(px(8.0))
                .child(policy_button(
                    t.keep_newest,
                    "policy-newest",
                    policy == KeepPolicy::Newest,
                    move || newest.set_policy(KeepPolicy::Newest),
                ))
                .child(policy_button(
                    t.keep_oldest,
                    "policy-oldest",
                    policy == KeepPolicy::Oldest,
                    move || oldest.set_policy(KeepPolicy::Oldest),
                )),
        )
        .child(action_button(&text, id, primary, busy, on_press))
}

fn policy_button(
    text: &'static str,
    id: &'static str,
    selected: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    press(id, false, on_press)
        .px(px(12.0))
        .py(px(8.0))
        .rounded(px(999.0))
        .border_1()
        .border_color(if selected { TEXT } else { BORDER })
        .bg(if selected { TEXT } else { SURFACE })
        .child(label(
            text,
            9.0,
            FontWeight::BOLD,
            if selected { SURFACE } else { TEXT },
        ))
}

fn action_button(
    text: &str,
    id: &'static str,
    primary: bool,
    disabled: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    press(id, disabled, on_press)
        .w_full()
        .items_center()
        .justify_center()
        .px(px(14.0))
        .py(px(13.0))
        .rounded(px(14.0))
        .bg(if primary { ACCENT } else { SURFACE })
        .border_1()
        .border_color(BORDER)
        .opacity(if disabled { 0.5 } else { 1.0 })
        .child(label(&text.to_uppercase(), 12.0, FontWeight::BOLD, TEXT))
}

fn confirm_overlay(s: &State) -> impl IntoElement + use<> {
    let state = s.clone();
    let cancel = s.clone();
    let delete = s.clone();
    let t = s.t();
    when(
        move || state.confirm.get(),
        move || {
            let cancel = cancel.clone();
            let backdrop = cancel.clone();
            let delete = delete.clone();
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
                        .bg(OVERLAY)
                        .on_press(move || backdrop.cancel_delete()),
                )
                .child(
                    view()
                        .test_id("confirm-dialog")
                        .w(px(320.0))
                        .bg(SURFACE)
                        .border_2()
                        .border_color(BORDER)
                        .rounded(px(18.0))
                        .p(px(20.0))
                        .gap(px(10.0))
                        .child(label(t.confirm_title, 20.0, FontWeight::BOLD, TEXT))
                        .child(label(t.confirm_body, 12.0, FontWeight::NORMAL, MUTED))
                        .child(
                            view()
                                .flex_row()
                                .gap(px(8.0))
                                .child(
                                    press("cancel-delete", false, move || cancel.cancel_delete())
                                        .flex_1()
                                        .items_center()
                                        .justify_center()
                                        .px(px(12.0))
                                        .py(px(12.0))
                                        .rounded(px(12.0))
                                        .bg(SURFACE)
                                        .border_1()
                                        .border_color(BORDER)
                                        .child(label(t.back, 11.0, FontWeight::BOLD, TEXT)),
                                )
                                .child(
                                    press("delete-confirm", false, move || delete.delete())
                                        .flex_1()
                                        .items_center()
                                        .justify_center()
                                        .px(px(12.0))
                                        .py(px(12.0))
                                        .rounded(px(12.0))
                                        .bg(ACCENT)
                                        .border_1()
                                        .border_color(BORDER)
                                        .child(label(t.delete, 11.0, FontWeight::BOLD, TEXT)),
                                ),
                        ),
                )
        },
    )
}

/// `YYYY-MM-DD` from seconds since the Unix epoch.
fn date(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Howard Hinnant's `civil_from_days`.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_dates() {
        assert_eq!(date(0), "1970-01-01");
        assert_eq!(date(1_700_000_000), "2023-11-14");
    }
}
