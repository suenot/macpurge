//! Terminal UI for macpurge (ratatui). Entry point: [`run`].

mod loader;
pub mod model;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use macpurge::UsageNode;
use model::{App, Cmd, Item, Section};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

enum Msg {
    Loaded(Section, Result<Vec<Item>, String>),
    Usage(Result<UsageNode, String>),
    Done { status: String, reload: Option<Section> },
}

struct Runtime {
    tx: Sender<Msg>,
    usage_tree: Option<UsageNode>,
}

impl Runtime {
    fn spawn(&self, f: impl FnOnce(Sender<Msg>) + Send + 'static) {
        let tx = self.tx.clone();
        std::thread::spawn(move || f(tx));
    }

    fn exec(&mut self, app: &mut App, cmd: Cmd) -> bool {
        match cmd {
            Cmd::Quit => return true,
            Cmd::Load(Section::DiskUsage) => {
                app.usage_path.clear();
                self.usage_tree = None;
                self.spawn(|tx| {
                    let _ = tx.send(Msg::Usage(loader::load_usage().map_err(|e| e.to_string())));
                });
            }
            Cmd::Load(s) => self.spawn(move |tx| {
                let _ = tx.send(Msg::Loaded(s, loader::load(s).map_err(|e| e.to_string())));
            }),
            Cmd::OpenUsage(p) => self.show_usage(app, &p),
            Cmd::Clean { paths, dry_run } => {
                let section = app.section;
                app.status = if dry_run { "Estimating…".into() } else { "Moving to Trash…".into() };
                self.spawn(move |tx| {
                    let status = loader::clean(&paths, dry_run);
                    let _ = tx.send(Msg::Done { status, reload: (!dry_run).then_some(section) });
                });
            }
            Cmd::RunRecipe { id, dry_run } => {
                app.status = "Running recipe…".into();
                self.spawn(move |tx| {
                    let status = loader::run_recipe_msg(&id, dry_run);
                    let _ = tx.send(Msg::Done { status, reload: None });
                });
            }
            Cmd::ToggleSchedule => self.spawn(|tx| {
                let _ = tx.send(Msg::Done { status: loader::toggle_schedule(), reload: Some(Section::Schedule) });
            }),
            Cmd::Commit(p) => self.git_task(app, p, loader::commit, "Committing…"),
            Cmd::Push(p) => self.git_task(app, p, loader::push, "Pushing…"),
            Cmd::Archive(p) => self.git_task(app, p, loader::archive, "Archiving…"),
            Cmd::Reveal(p) => loader::reveal(&p),
        }
        false
    }

    fn git_task(&self, app: &mut App, p: PathBuf, f: fn(&Path) -> String, busy: &'static str) {
        app.status = busy.into();
        let reload = (app.section == Section::Projects).then_some(Section::Projects);
        self.spawn(move |tx| {
            let status = f(&p);
            let _ = tx.send(Msg::Done { status, reload });
        });
    }

    fn show_usage(&self, app: &mut App, path: &Path) {
        let Some(tree) = &self.usage_tree else { return };
        let node = if path.as_os_str().is_empty() { Some(tree) } else { tree.find(path) };
        if let Some(n) = node {
            app.set_items(Section::DiskUsage, loader::usage_items(n));
        }
    }

    fn handle(&mut self, app: &mut App, msg: Msg) -> Vec<Cmd> {
        let mut follow = Vec::new();
        match msg {
            Msg::Loaded(s, Ok(items)) => app.set_items(s, items),
            Msg::Loaded(s, Err(e)) => {
                app.loading.insert(s, false);
                app.status = format!("{}: {e}", s.title());
            }
            Msg::Usage(Ok(tree)) => {
                app.set_items(Section::DiskUsage, loader::usage_items(&tree));
                self.usage_tree = Some(tree);
            }
            Msg::Usage(Err(e)) => {
                app.loading.insert(Section::DiskUsage, false);
                app.status = format!("Disk Usage: {e}");
            }
            Msg::Done { status, reload } => {
                app.status = status;
                if let Some(s) = reload {
                    app.loading.insert(s, true);
                    follow.push(Cmd::Load(s));
                }
            }
        }
        follow
    }
}

pub fn run() -> Result<()> {
    // Restore the terminal even when something panics.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        default_hook(info);
    }));

    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    let mut term = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let result = event_loop(&mut term);
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    term.show_cursor()?;
    result
}

fn event_loop(term: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    let (tx, rx): (Sender<Msg>, Receiver<Msg>) = channel();
    let mut rt = Runtime { tx, usage_tree: None };
    let mut app = App::new();
    if let Some(c) = app.select_section(Section::DiskUsage) {
        rt.exec(&mut app, c);
    }

    loop {
        app.tick = app.tick.wrapping_add(1);
        term.draw(|f| ui::draw(f, &app))?;

        let mut cmds = Vec::new();
        if event::poll(Duration::from_millis(80))? {
            if let Event::Key(k) = event::read()? {
                if k.kind == KeyEventKind::Press {
                    app.status.clear();
                    cmds.extend(app.on_key(k));
                }
            }
        }
        while let Ok(m) = rx.try_recv() {
            cmds.extend(rt.handle(&mut app, m));
        }
        for c in cmds {
            if rt.exec(&mut app, c) {
                return Ok(());
            }
        }
    }
}
