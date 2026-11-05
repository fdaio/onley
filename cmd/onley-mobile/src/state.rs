//! The screen's state, without the view.

use crate::i18n::{self, Lang, Strings};
use crate::model::{self, Group, KeepPolicy, Summary};
use crate::scan::{self, ScanProgress};
use crate::source::{self, Access, PhotoLibrary};
use nsh_core::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

static NEXT_LIBRARY: Mutex<Option<Arc<dyn PhotoLibrary>>> = Mutex::new(None);

/// Gives the next app instance its library; tests use it.
pub fn set_library(library: Arc<dyn PhotoLibrary>) {
    *NEXT_LIBRARY.lock().unwrap() = Some(library);
}

#[derive(Clone)]
pub struct State {
    pub library: Arc<dyn PhotoLibrary>,
    pub access: Signal<Access>,
    pub scanning: Signal<bool>,
    pub progress: Signal<ScanProgress>,
    pub groups: Signal<Vec<Group>>,
    pub keep: Signal<HashMap<String, String>>,
    pub policy: Signal<KeepPolicy>,
    pub summary: Signal<Summary>,
    pub message: Signal<String>,
    pub note: Signal<String>,
    pub note_success: Signal<bool>,
    pub confirm: Signal<bool>,
    pub busy: Signal<bool>,
    pub lang: Signal<Lang>,
    pub cancel: Rc<RefCell<Option<Arc<AtomicBool>>>>,
    pub owner: Owner,
}

impl State {
    pub fn new() -> Self {
        let library = NEXT_LIBRARY
            .lock()
            .unwrap()
            .take()
            .unwrap_or_else(source::open);
        let access = library.access();
        let state = Self {
            library,
            access: signal(access),
            scanning: signal(false),
            progress: signal(ScanProgress::default()),
            groups: signal(Vec::new()),
            keep: signal(HashMap::new()),
            policy: signal(KeepPolicy::default()),
            summary: signal(Summary::default()),
            message: signal(String::new()),
            note: signal(String::new()),
            note_success: signal(false),
            confirm: signal(false),
            busy: signal(false),
            lang: signal(Lang::detect()),
            cancel: Rc::new(RefCell::new(None)),
            owner: Owner::current(),
        };
        state
    }

    pub fn spawn_task(&self, future: impl std::future::Future<Output = ()> + 'static) {
        let owner = self.owner;
        owner.run(|| {
            let _spawned = spawn(future);
        });
    }

    pub fn t(&self) -> &'static Strings {
        i18n::strings(self.lang.get())
    }

    pub fn toggle_lang(&self) {
        self.lang.set(self.lang.get().toggle());
    }

    /// Asks for access if needed, then scans.
    pub fn scan(&self) {
        if self.scanning.get() || self.busy.get() {
            return;
        }
        if self.access.get().can_read() {
            self.spawn_scan();
            return;
        }
        let state = self.clone();
        self.spawn_task(async move {
            state.request_access().await;
            if state.access.get().can_read() {
                state.spawn_scan();
            }
        });
    }

    async fn request_access(&self) {
        let (tx, rx) = mpsc::channel();
        let library = self.library.clone();
        std::thread::spawn(move || {
            let _ = tx.send(library.request());
        });
        loop {
            match rx.try_recv() {
                Ok(access) => {
                    self.access.set(access);
                    break;
                }
                Err(TryRecvError::Empty) => timer(Duration::from_millis(100)).await,
                Err(TryRecvError::Disconnected) => break,
            }
        }
    }

    fn spawn_scan(&self) {
        let state = self.clone();
        self.spawn_task(async move {
            state.run_scan().await;
        });
    }

    async fn run_scan(&self) {
        self.scanning.set(true);
        self.busy.set(true);
        self.message.set(String::new());
        self.set_note("", false);
        self.progress.set(ScanProgress::default());

        let cancel = Arc::new(AtomicBool::new(false));
        self.cancel.replace(Some(cancel.clone()));
        let (tx, rx) = mpsc::channel();
        let library = self.library.clone();
        let handle = std::thread::spawn(move || {
            let mut progress = |event: ScanProgress| {
                let _ = tx.send(Message::Progress(event));
            };
            let result = scan::scan(library.as_ref(), &cancel, &mut progress);
            let _ = tx.send(Message::Finished(result));
        });

        let mut finished = false;
        while !finished {
            while let Ok(message) = rx.try_recv() {
                match message {
                    Message::Progress(event) => self.progress.set(event),
                    Message::Finished(result) => {
                        finished = true;
                        self.apply_result(result);
                    }
                }
            }
            if !finished {
                timer(Duration::from_millis(50)).await;
            }
        }
        let _ = handle.join();
        self.scanning.set(false);
        self.busy.set(false);
        self.cancel.replace(None);
    }

    fn apply_result(&self, result: Result<scan::ScanOutcome, String>) {
        match result {
            Ok(outcome) => {
                let groups = model::group_duplicates(outcome.assets);
                self.groups.set(groups);
                self.apply_policy(self.policy.get());
                if outcome.failed > 0 {
                    let text = self.t();
                    self.message.set(format!(
                        "{} {} {}",
                        outcome.failed, text.failed, text.scan_failed
                    ));
                } else {
                    self.message.set(self.t().scan_done.to_owned());
                }
                self.set_note("", false);
            }
            Err(err) => {
                self.message.set(err);
            }
        }
    }

    /// Recomputes the copy each group keeps.
    pub fn apply_policy(&self, policy: KeepPolicy) {
        self.policy.set(policy);
        let mut keep = HashMap::new();
        for group in self.groups.get() {
            if let Some(id) = group.kept_id(policy) {
                keep.insert(group.hash.clone(), id.to_owned());
            }
        }
        self.keep.set(keep);
        self.refresh_summary();
    }

    pub fn set_policy(&self, policy: KeepPolicy) {
        self.apply_policy(policy);
    }

    pub fn set_keep(&self, hash: &str, id: &str) {
        let mut keep = self.keep.get();
        keep.insert(hash.to_owned(), id.to_owned());
        self.keep.set(keep);
    }

    pub fn kept(&self, group: &Group) -> Option<String> {
        self.keep
            .get()
            .get(&group.hash)
            .cloned()
            .or_else(|| group.kept_id(self.policy.get()).map(str::to_owned))
    }

    fn refresh_summary(&self) {
        self.summary.set(model::summarize(&self.groups.get()));
    }

    /// The copies a cleanup removes now.
    pub fn targets(&self) -> Vec<String> {
        let mut targets = Vec::new();
        for group in self.groups.get() {
            let kept = self.kept(&group);
            for asset in &group.assets {
                if Some(&asset.id) != kept.as_ref() {
                    targets.push(asset.id.clone());
                }
            }
        }
        targets
    }

    pub fn ask_delete(&self) {
        if self.targets().is_empty() {
            return;
        }
        self.confirm.set(true);
    }

    pub fn cancel_delete(&self) {
        self.confirm.set(false);
    }

    /// Deletes every copy except the kept one.
    pub fn delete(&self) {
        self.confirm.set(false);
        let targets = self.targets();
        if targets.is_empty() {
            return;
        }
        self.busy.set(true);
        let state = self.clone();
        self.spawn_task(async move {
            let (tx, rx) = mpsc::channel();
            let library = state.library.clone();
            let ids = targets.clone();
            std::thread::spawn(move || {
                let _ = tx.send(library.delete(&ids));
            });
            loop {
                match rx.try_recv() {
                    Ok(result) => {
                        state.finish_delete(&targets, result);
                        break;
                    }
                    Err(TryRecvError::Empty) => timer(Duration::from_millis(50)).await,
                    Err(TryRecvError::Disconnected) => break,
                }
            }
            state.busy.set(false);
        });
    }

    fn finish_delete(&self, targets: &[String], result: Result<usize, String>) {
        match result {
            Ok(deleted) => {
                let removed: std::collections::HashSet<&String> = targets.iter().collect();
                let mut groups = self.groups.get();
                for group in &mut groups {
                    group.assets.retain(|asset| !removed.contains(&asset.id));
                }
                groups.retain(|group| group.assets.len() > 1);
                self.groups.set(groups);
                let mut keep = self.keep.get();
                keep.retain(|hash, _| self.groups.get().iter().any(|group| &group.hash == hash));
                self.keep.set(keep);
                self.refresh_summary();
                self.set_note(
                    format!("{} {} {}", deleted, self.t().copies, self.t().deleted),
                    true,
                );
                self.message.set(String::new());
            }
            Err(err) => {
                self.set_note(format!("{}: {err}", self.t().delete_failed), false);
            }
        }
    }

    pub fn cancel_scan(&self) {
        if let Some(cancel) = self.cancel.borrow().as_ref() {
            cancel.store(true, Ordering::Relaxed);
        }
    }

    pub fn set_note(&self, text: impl Into<String>, success: bool) {
        self.note.set(text.into());
        self.note_success.set(success);
    }

    pub fn access_message(&self) -> String {
        let t = self.t();
        match self.access.get() {
            Access::Authorized => String::new(),
            Access::Limited => t.access_limited.to_owned(),
            Access::Denied => t.access_denied.to_owned(),
            Access::Unknown => t.access_unknown.to_owned(),
            Access::Unsupported => t.access_unsupported.to_owned(),
        }
    }
}

enum Message {
    Progress(ScanProgress),
    Finished(Result<scan::ScanOutcome, String>),
}
