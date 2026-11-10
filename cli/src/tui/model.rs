//! Pure TUI state: no IO, no terminal. Everything here is unit-testable.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Section {
    DiskUsage,
    Projects,
    DevArtifacts,
    Packages,
    System,
    Applications,
    DevTools,
    AiModels,
    Recipes,
    Schedule,
    Settings,
}

impl Section {
    pub const ALL: [Section; 11] = [
        Section::DiskUsage,
        Section::Projects,
        Section::DevArtifacts,
        Section::Packages,
        Section::System,
        Section::Applications,
        Section::DevTools,
        Section::AiModels,
        Section::Recipes,
        Section::Schedule,
        Section::Settings,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Section::DiskUsage => "Disk Usage",
            Section::Projects => "Projects",
            Section::DevArtifacts => "Dev Artifacts",
            Section::Packages => "Packages",
            Section::System => "System",
            Section::Applications => "Applications",
            Section::DevTools => "Dev Tools",
            Section::AiModels => "AI Models",
            Section::Recipes => "Recipes",
            Section::Schedule => "Schedule",
            Section::Settings => "Settings",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Section::DiskUsage => "Enter open · Backspace up · o reveal in Finder",
            Section::Projects => "c commit · p push · A archive",
            Section::Recipes => "Enter run · D dry-run",
            Section::Schedule => "Enter enable / disable",
            Section::Settings => "edit with `macpurge config`",
            _ => "Space select · a all · d trash · D dry-run",
        }
    }

    /// Sections whose items can be moved to the Trash.
    pub fn deletable(self) -> bool {
        !matches!(self, Section::Recipes | Section::Schedule | Section::Settings)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Item {
    pub label: String,
    pub detail: String,
    pub tag: String,
    pub path: Option<PathBuf>,
    pub size: u64,
    pub is_dir: bool,
    pub selected: bool,
    pub id: Option<String>,
}

#[derive(Debug, Default)]
pub struct ListModel {
    pub items: Vec<Item>,
    pub cursor: usize,
    pub filter: String,
}

impl ListModel {
    pub fn new(items: Vec<Item>) -> Self {
        Self { items, cursor: 0, filter: String::new() }
    }

    /// Indices of items matching the filter, in order.
    pub fn visible(&self) -> Vec<usize> {
        let f = self.filter.to_lowercase();
        self.items
            .iter()
            .enumerate()
            .filter(|(_, it)| {
                f.is_empty()
                    || it.label.to_lowercase().contains(&f)
                    || it.detail.to_lowercase().contains(&f)
                    || it.tag.to_lowercase().contains(&f)
            })
            .map(|(i, _)| i)
            .collect()
    }

    pub fn move_cursor(&mut self, delta: isize) {
        let n = self.visible().len();
        if n == 0 {
            self.cursor = 0;
            return;
        }
        self.cursor = (self.cursor as isize + delta).clamp(0, n as isize - 1) as usize;
    }

    pub fn current_index(&self) -> Option<usize> {
        self.visible().get(self.cursor).copied()
    }

    pub fn current(&self) -> Option<&Item> {
        self.current_index().map(|i| &self.items[i])
    }

    pub fn toggle_current(&mut self) {
        if let Some(i) = self.current_index() {
            if self.items[i].path.is_some() {
                self.items[i].selected = !self.items[i].selected;
            }
        }
    }

    /// Select all visible deletable items, or clear when they are all selected.
    pub fn toggle_all(&mut self) {
        let vis: Vec<usize> = self.visible().into_iter().filter(|&i| self.items[i].path.is_some()).collect();
        let all = !vis.is_empty() && vis.iter().all(|&i| self.items[i].selected);
        for i in vis {
            self.items[i].selected = !all;
        }
    }

    pub fn selected_paths(&self) -> Vec<PathBuf> {
        self.items.iter().filter(|i| i.selected).filter_map(|i| i.path.clone()).collect()
    }

    pub fn selected_size(&self) -> u64 {
        self.items.iter().filter(|i| i.selected).map(|i| i.size).sum()
    }

    pub fn total_size(&self) -> u64 {
        self.items.iter().map(|i| i.size).sum()
    }

    pub fn max_size(&self) -> u64 {
        self.items.iter().map(|i| i.size).max().unwrap_or(0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Focus {
    Sidebar,
    List,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Filter,
    Help,
    Confirm(Confirm),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Confirm {
    Trash { paths: Vec<PathBuf>, size: u64 },
    Archive { path: PathBuf },
    Recipe { id: String },
}

/// Side effects the event loop must perform; the model itself never does IO.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cmd {
    Quit,
    Load(Section),
    Clean { paths: Vec<PathBuf>, dry_run: bool },
    RunRecipe { id: String, dry_run: bool },
    ToggleSchedule,
    Commit(PathBuf),
    Push(PathBuf),
    Archive(PathBuf),
    Reveal(PathBuf),
    OpenUsage(PathBuf),
}

pub struct App {
    pub section: Section,
    pub focus: Focus,
    pub mode: Mode,
    pub lists: HashMap<Section, ListModel>,
    pub loading: HashMap<Section, bool>,
    pub status: String,
    /// Disk Usage drill-down: path of the node currently shown (None = root).
    pub usage_path: Vec<PathBuf>,
    pub tick: usize,
}

impl App {
    pub fn new() -> Self {
        Self {
            section: Section::DiskUsage,
            focus: Focus::Sidebar,
            mode: Mode::Normal,
            lists: HashMap::new(),
            loading: HashMap::new(),
            status: String::new(),
            usage_path: Vec::new(),
            tick: 0,
        }
    }

    pub fn list(&self) -> Option<&ListModel> {
        self.lists.get(&self.section)
    }

    fn list_mut(&mut self) -> Option<&mut ListModel> {
        self.lists.get_mut(&self.section)
    }

    pub fn is_loading(&self, s: Section) -> bool {
        self.loading.get(&s).copied().unwrap_or(false)
    }

    pub fn set_items(&mut self, s: Section, items: Vec<Item>) {
        let filter = self.lists.get(&s).map(|l| l.filter.clone()).unwrap_or_default();
        let mut m = ListModel::new(items);
        m.filter = filter;
        self.lists.insert(s, m);
        self.loading.insert(s, false);
    }

    /// Switch section; returns a load command the first time a section is opened.
    pub fn select_section(&mut self, s: Section) -> Option<Cmd> {
        self.section = s;
        if !self.lists.contains_key(&s) && !self.is_loading(s) {
            self.loading.insert(s, true);
            return Some(Cmd::Load(s));
        }
        None
    }

    fn sidebar_index(&self) -> usize {
        Section::ALL.iter().position(|s| *s == self.section).unwrap_or(0)
    }

    pub fn on_key(&mut self, key: KeyEvent) -> Vec<Cmd> {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return vec![Cmd::Quit];
        }
        match self.mode.clone() {
            Mode::Help => {
                self.mode = Mode::Normal;
                vec![]
            }
            Mode::Filter => {
                match key.code {
                    KeyCode::Esc => {
                        if let Some(l) = self.list_mut() {
                            l.filter.clear();
                            l.cursor = 0;
                        }
                        self.mode = Mode::Normal;
                    }
                    KeyCode::Enter => self.mode = Mode::Normal,
                    KeyCode::Backspace => {
                        if let Some(l) = self.list_mut() {
                            l.filter.pop();
                            l.cursor = 0;
                        }
                    }
                    KeyCode::Char(c) => {
                        if let Some(l) = self.list_mut() {
                            l.filter.push(c);
                            l.cursor = 0;
                        }
                    }
                    _ => {}
                }
                vec![]
            }
            Mode::Confirm(c) => match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    self.mode = Mode::Normal;
                    match c {
                        Confirm::Trash { paths, .. } => vec![Cmd::Clean { paths, dry_run: false }],
                        Confirm::Archive { path } => vec![Cmd::Archive(path)],
                        Confirm::Recipe { id } => vec![Cmd::RunRecipe { id, dry_run: false }],
                    }
                }
                _ => {
                    self.mode = Mode::Normal;
                    self.status = "Cancelled".into();
                    vec![]
                }
            },
            Mode::Normal => self.on_key_normal(key),
        }
    }

    fn on_key_normal(&mut self, key: KeyEvent) -> Vec<Cmd> {
        let mut out = Vec::new();
        match key.code {
            KeyCode::Char('q') => return vec![Cmd::Quit],
            KeyCode::Char('?') => self.mode = Mode::Help,
            KeyCode::Char('/') => {
                self.focus = Focus::List;
                self.mode = Mode::Filter;
            }
            KeyCode::Tab => {
                self.focus = if self.focus == Focus::Sidebar { Focus::List } else { Focus::Sidebar };
            }
            KeyCode::BackTab => {
                self.focus = if self.focus == Focus::Sidebar { Focus::List } else { Focus::Sidebar };
            }
            KeyCode::Char('r') => {
                self.loading.insert(self.section, true);
                out.push(Cmd::Load(self.section));
            }
            KeyCode::Down | KeyCode::Char('j') => out.extend(self.move_by(1)),
            KeyCode::Up | KeyCode::Char('k') => out.extend(self.move_by(-1)),
            KeyCode::Right | KeyCode::Char('l') if self.focus == Focus::Sidebar => self.focus = Focus::List,
            KeyCode::Left | KeyCode::Char('h') if self.focus == Focus::List => self.focus = Focus::Sidebar,
            KeyCode::Char('g') => {
                if let Some(l) = self.list_mut() {
                    l.cursor = 0;
                }
            }
            KeyCode::Char('G') => {
                if let Some(l) = self.list_mut() {
                    l.cursor = l.visible().len().saturating_sub(1);
                }
            }
            KeyCode::Char(' ') if self.focus == Focus::List => {
                if let Some(l) = self.list_mut() {
                    l.toggle_current();
                    l.move_cursor(1);
                }
            }
            KeyCode::Char('a') if self.focus == Focus::List => {
                if let Some(l) = self.list_mut() {
                    l.toggle_all();
                }
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                let dry = key.code == KeyCode::Char('D');
                out.extend(self.trash_selected(dry));
            }
            KeyCode::Enter => out.extend(self.enter()),
            KeyCode::Backspace | KeyCode::Esc => {
                if self.section == Section::DiskUsage && self.focus == Focus::List && !self.usage_path.is_empty() {
                    self.usage_path.pop();
                    out.push(Cmd::OpenUsage(self.usage_path.last().cloned().unwrap_or_default()));
                } else if self.focus == Focus::List {
                    self.focus = Focus::Sidebar;
                }
            }
            KeyCode::Char('o') => {
                if let Some(p) = self.list().and_then(|l| l.current()).and_then(|i| i.path.clone()) {
                    out.push(Cmd::Reveal(p));
                }
            }
            KeyCode::Char('c') if self.section == Section::Projects => {
                if let Some(p) = self.current_path() {
                    out.push(Cmd::Commit(p));
                }
            }
            KeyCode::Char('p') if self.section == Section::Projects => {
                if let Some(p) = self.current_path() {
                    out.push(Cmd::Push(p));
                }
            }
            KeyCode::Char('A') if self.section == Section::Projects => {
                if let Some(p) = self.current_path() {
                    self.mode = Mode::Confirm(Confirm::Archive { path: p });
                }
            }
            _ => {}
        }
        out
    }

    fn current_path(&self) -> Option<PathBuf> {
        self.list().and_then(|l| l.current()).and_then(|i| i.path.clone())
    }

    fn move_by(&mut self, delta: isize) -> Vec<Cmd> {
        if self.focus == Focus::Sidebar {
            let idx = (self.sidebar_index() as isize + delta).clamp(0, Section::ALL.len() as isize - 1) as usize;
            return self.select_section(Section::ALL[idx]).into_iter().collect();
        }
        if let Some(l) = self.list_mut() {
            l.move_cursor(delta);
        }
        vec![]
    }

    fn trash_selected(&mut self, dry: bool) -> Vec<Cmd> {
        if !self.section.deletable() {
            return vec![];
        }
        let Some(l) = self.list() else { return vec![] };
        let mut paths = l.selected_paths();
        let mut size = l.selected_size();
        if paths.is_empty() {
            // nothing marked: act on the item under the cursor
            if let Some(it) = l.current() {
                if let Some(p) = &it.path {
                    paths.push(p.clone());
                    size = it.size;
                }
            }
        }
        if paths.is_empty() {
            self.status = "Nothing to clean here".into();
            return vec![];
        }
        if dry {
            return vec![Cmd::Clean { paths, dry_run: true }];
        }
        self.mode = Mode::Confirm(Confirm::Trash { paths, size });
        vec![]
    }

    fn enter(&mut self) -> Vec<Cmd> {
        if self.focus == Focus::Sidebar {
            self.focus = Focus::List;
            return vec![];
        }
        match self.section {
            Section::DiskUsage => {
                if let Some(it) = self.list().and_then(|l| l.current()).cloned() {
                    if it.is_dir {
                        if let Some(p) = it.path {
                            self.usage_path.push(p.clone());
                            return vec![Cmd::OpenUsage(p)];
                        }
                    }
                }
                vec![]
            }
            Section::Recipes => {
                if let Some(id) = self.list().and_then(|l| l.current()).and_then(|i| i.id.clone()) {
                    self.mode = Mode::Confirm(Confirm::Recipe { id });
                }
                vec![]
            }
            Section::Schedule => vec![Cmd::ToggleSchedule],
            _ => {
                if let Some(l) = self.list_mut() {
                    l.toggle_current();
                }
                vec![]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(c: KeyCode) -> KeyEvent {
        KeyEvent::new(c, KeyModifiers::NONE)
    }

    fn item(name: &str, size: u64) -> Item {
        Item { label: name.into(), size, path: Some(PathBuf::from(format!("/tmp/{name}"))), ..Default::default() }
    }

    #[test]
    fn filter_and_cursor() {
        let mut l = ListModel::new(vec![item("alpha", 1), item("beta", 2), item("alps", 3)]);
        l.filter = "al".into();
        assert_eq!(l.visible(), vec![0, 2]);
        l.move_cursor(5);
        assert_eq!(l.current().unwrap().label, "alps");
        l.move_cursor(-9);
        assert_eq!(l.cursor, 0);
    }

    #[test]
    fn selection_and_sizes() {
        let mut l = ListModel::new(vec![item("a", 10), item("b", 20)]);
        l.toggle_current();
        assert_eq!(l.selected_size(), 10);
        l.toggle_all();
        assert_eq!(l.selected_paths().len(), 2);
        l.toggle_all();
        assert!(l.selected_paths().is_empty());
    }

    #[test]
    fn items_without_path_cannot_be_selected() {
        let mut l = ListModel::new(vec![Item { label: "info".into(), ..Default::default() }]);
        l.toggle_current();
        l.toggle_all();
        assert!(l.selected_paths().is_empty());
    }

    #[test]
    fn first_visit_loads_once() {
        let mut app = App::new();
        assert_eq!(app.select_section(Section::Packages), Some(Cmd::Load(Section::Packages)));
        assert_eq!(app.select_section(Section::Packages), None);
    }

    #[test]
    fn trash_requires_confirmation_then_cleans() {
        let mut app = App::new();
        app.section = Section::Packages;
        app.focus = Focus::List;
        app.set_items(Section::Packages, vec![item("npm", 100)]);
        assert!(app.on_key(key(KeyCode::Char(' '))).is_empty());
        assert!(app.on_key(key(KeyCode::Char('d'))).is_empty());
        assert!(matches!(app.mode, Mode::Confirm(Confirm::Trash { size: 100, .. })));
        let cmds = app.on_key(key(KeyCode::Char('y')));
        assert_eq!(cmds, vec![Cmd::Clean { paths: vec![PathBuf::from("/tmp/npm")], dry_run: false }]);
    }

    #[test]
    fn confirm_cancelled_by_any_other_key() {
        let mut app = App::new();
        app.section = Section::Packages;
        app.focus = Focus::List;
        app.set_items(Section::Packages, vec![item("npm", 100)]);
        app.on_key(key(KeyCode::Char('d')));
        assert!(app.on_key(key(KeyCode::Char('n'))).is_empty());
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn dry_run_skips_confirmation() {
        let mut app = App::new();
        app.section = Section::System;
        app.focus = Focus::List;
        app.set_items(Section::System, vec![item("trash", 5)]);
        let cmds = app.on_key(key(KeyCode::Char('D')));
        assert!(matches!(cmds[0], Cmd::Clean { dry_run: true, .. }));
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn usage_drill_and_back() {
        let mut app = App::new();
        app.focus = Focus::List;
        app.set_items(Section::DiskUsage, vec![Item { is_dir: true, ..item("projects", 9) }]);
        let c = app.on_key(key(KeyCode::Enter));
        assert_eq!(c, vec![Cmd::OpenUsage(PathBuf::from("/tmp/projects"))]);
        let c = app.on_key(key(KeyCode::Backspace));
        assert!(matches!(c[0], Cmd::OpenUsage(_)));
        assert!(app.usage_path.is_empty());
    }

    #[test]
    fn filter_mode_typing() {
        let mut app = App::new();
        app.set_items(Section::DiskUsage, vec![item("abc", 1), item("xyz", 2)]);
        app.on_key(key(KeyCode::Char('/')));
        app.on_key(key(KeyCode::Char('x')));
        assert_eq!(app.list().unwrap().visible(), vec![1]);
        app.on_key(key(KeyCode::Esc));
        assert_eq!(app.list().unwrap().visible().len(), 2);
    }

    #[test]
    fn sidebar_navigation_loads_sections() {
        let mut app = App::new();
        let cmds = app.on_key(key(KeyCode::Char('j')));
        assert_eq!(app.section, Section::Projects);
        assert_eq!(cmds, vec![Cmd::Load(Section::Projects)]);
    }

    #[test]
    fn q_and_ctrl_c_quit() {
        let mut app = App::new();
        assert_eq!(app.on_key(key(KeyCode::Char('q'))), vec![Cmd::Quit]);
        let c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(app.on_key(c), vec![Cmd::Quit]);
    }
}
