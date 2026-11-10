//! Rendering. Reads `App`, draws widgets; contains no state changes.

use super::loader::tilde;
use super::model::{App, Confirm, Focus, ListModel, Mode, Section};
use macpurge::utils::format_size;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

const AMBER: Color = Color::Rgb(245, 158, 11);
const SAND: Color = Color::Rgb(253, 225, 184);
const VIOLET: Color = Color::Rgb(139, 92, 246);
const DIM: Color = Color::Rgb(120, 116, 130);
const SPINNER: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];

pub fn draw(f: &mut Frame, app: &App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(1)])
        .split(f.area());
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(22), Constraint::Min(30)])
        .split(rows[0]);

    draw_sidebar(f, app, cols[0]);
    draw_main(f, app, cols[1]);
    draw_footer(f, app, rows[1]);

    match &app.mode {
        Mode::Help => draw_help(f),
        Mode::Confirm(c) => draw_confirm(f, c),
        _ => {}
    }
}

fn draw_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = Section::ALL
        .iter()
        .map(|s| {
            let busy = app.is_loading(*s);
            let mark = if busy { SPINNER[app.tick % SPINNER.len()] } else { " " };
            let style = if *s == app.section {
                Style::default().fg(AMBER).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            ListItem::new(Line::from(vec![Span::styled(format!(" {} ", mark), Style::default().fg(VIOLET)), Span::styled(s.title(), style)]))
        })
        .collect();
    let border = if app.focus == Focus::Sidebar { AMBER } else { DIM };
    let title = Line::from(vec![Span::styled(" ◔ ", Style::default().fg(AMBER)), Span::styled("macpurge ", Style::default().fg(SAND).add_modifier(Modifier::BOLD))]);
    let mut state = ListState::default();
    state.select(Section::ALL.iter().position(|s| *s == app.section));
    f.render_stateful_widget(
        List::new(items)
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(border)).title(title))
            .highlight_style(Style::default().bg(Color::Rgb(40, 32, 20))),
        area,
        &mut state,
    );
}

fn bar(frac: f64, width: usize) -> String {
    let n = (frac.clamp(0.0, 1.0) * width as f64).round() as usize;
    format!("{}{}", "█".repeat(n), "░".repeat(width - n))
}

fn draw_main(f: &mut Frame, app: &App, area: Rect) {
    let border = if app.focus == Focus::List { AMBER } else { DIM };
    let mut title = vec![Span::styled(format!(" {} ", app.section.title()), Style::default().fg(SAND).add_modifier(Modifier::BOLD))];
    if app.is_loading(app.section) {
        title.push(Span::styled(format!("{} scanning ", SPINNER[app.tick % SPINNER.len()]), Style::default().fg(VIOLET)));
    }
    let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(border)).title(Line::from(title));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let Some(list) = app.list() else {
        let msg = if app.is_loading(app.section) { "Scanning…" } else { "Nothing loaded. Press r to scan." };
        f.render_widget(Paragraph::new(msg).style(Style::default().fg(DIM)).alignment(Alignment::Center), inner);
        return;
    };

    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(1)])
        .split(inner);
    f.render_widget(Paragraph::new(summary(app, list)), parts[0]);
    draw_list(f, app, list, parts[1]);
}

fn summary(app: &App, list: &ListModel) -> Vec<Line<'static>> {
    let mut l1 = vec![
        Span::styled(format_size(list.total_size()), Style::default().fg(AMBER).add_modifier(Modifier::BOLD)),
        Span::styled(format!(" in {} items", list.items.len()), Style::default().fg(DIM)),
    ];
    let sel = list.selected_paths().len();
    if sel > 0 {
        l1.push(Span::styled(format!("   {} selected · {}", sel, format_size(list.selected_size())), Style::default().fg(VIOLET)));
    }
    if !list.filter.is_empty() || app.mode == Mode::Filter {
        l1.push(Span::styled(format!("   /{}", list.filter), Style::default().fg(SAND)));
    }
    let crumb = if app.section == Section::DiskUsage {
        app.usage_path.last().map(|p| tilde(p)).unwrap_or_else(|| "~".into())
    } else {
        app.section.hint().to_string()
    };
    vec![Line::from(l1), Line::from(Span::styled(crumb, Style::default().fg(DIM)))]
}

fn draw_list(f: &mut Frame, app: &App, list: &ListModel, area: Rect) {
    let max = list.max_size().max(1);
    let vis = list.visible();
    let width = area.width as usize;
    let show_size = list.max_size() > 0;
    let items: Vec<ListItem> = vis
        .iter()
        .map(|&i| {
            let it = &list.items[i];
            let check = if it.path.is_none() {
                " "
            } else if it.selected {
                "◉"
            } else {
                "○"
            };
            let mut spans = vec![Span::styled(format!("{check} "), Style::default().fg(if it.selected { AMBER } else { DIM }))];
            if show_size {
                spans.push(Span::styled(bar(it.size as f64 / max as f64, 8), Style::default().fg(AMBER)));
                spans.push(Span::styled(format!(" {:>9} ", format_size(it.size)), Style::default().fg(SAND)));
            }
            let used = 2 + if show_size { 8 + 11 } else { 0 };
            let tag_w = if it.tag.is_empty() { 0 } else { it.tag.chars().count() + 3 };
            let label_w = (width.saturating_sub(used + tag_w + 2)).clamp(10, 60);
            let label: String = if it.label.chars().count() > label_w {
                let mut s: String = it.label.chars().take(label_w - 1).collect();
                s.push('…');
                s
            } else {
                it.label.clone()
            };
            let suffix = if it.is_dir { "/" } else { "" };
            spans.push(Span::raw(format!("{label}{suffix}")));
            if !it.tag.is_empty() {
                spans.push(Span::styled(format!("  [{}]", it.tag), Style::default().fg(VIOLET)));
            }
            if !it.detail.is_empty() {
                spans.push(Span::styled(format!("  {}", it.detail), Style::default().fg(DIM)));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();
    let mut state = ListState::default();
    if !vis.is_empty() {
        state.select(Some(list.cursor.min(vis.len() - 1)));
    }
    let hl = if app.focus == Focus::List { Color::Rgb(52, 38, 18) } else { Color::Rgb(30, 26, 22) };
    f.render_stateful_widget(List::new(items).highlight_style(Style::default().bg(hl)), area, &mut state);
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let text = if !app.status.is_empty() {
        Line::from(Span::styled(format!(" {}", app.status), Style::default().fg(SAND)))
    } else {
        Line::from(vec![
            Span::styled(" j/k", Style::default().fg(AMBER)),
            Span::styled(" move  ", Style::default().fg(DIM)),
            Span::styled("Tab", Style::default().fg(AMBER)),
            Span::styled(" pane  ", Style::default().fg(DIM)),
            Span::styled("/", Style::default().fg(AMBER)),
            Span::styled(" filter  ", Style::default().fg(DIM)),
            Span::styled("r", Style::default().fg(AMBER)),
            Span::styled(" rescan  ", Style::default().fg(DIM)),
            Span::styled("?", Style::default().fg(AMBER)),
            Span::styled(" help  ", Style::default().fg(DIM)),
            Span::styled("q", Style::default().fg(AMBER)),
            Span::styled(" quit", Style::default().fg(DIM)),
        ])
    };
    f.render_widget(Paragraph::new(text), area);
}

fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    Rect { x: area.x + (area.width - w) / 2, y: area.y + (area.height - h) / 2, width: w, height: h }
}

fn popup(f: &mut Frame, title: &str, lines: Vec<Line<'static>>, w: u16, h: u16) {
    let r = centered(f.area(), w, h);
    f.render_widget(Clear, r);
    f.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(AMBER))
                .title(Span::styled(format!(" {title} "), Style::default().fg(SAND).add_modifier(Modifier::BOLD))),
        ),
        r,
    );
}

fn draw_confirm(f: &mut Frame, c: &Confirm) {
    let (title, body) = match c {
        Confirm::Trash { paths, size } => (
            "Move to Trash",
            format!("{} item(s), {}.\nThey go to the Trash and can be restored.", paths.len(), format_size(*size)),
        ),
        Confirm::Archive { path } => ("Archive project", format!("{}\nZip it into the archive dir, then move the source to the Trash.", tilde(path))),
        Confirm::Recipe { id } => ("Run recipe", format!("Run '{id}' now? Press Esc and use D for a dry run first.")),
    };
    let lines = vec![
        Line::from(""),
        Line::from(body),
        Line::from(""),
        Line::from(vec![Span::styled("  y / Enter ", Style::default().fg(AMBER)), Span::raw("confirm   "), Span::styled("any other key ", Style::default().fg(DIM)), Span::raw("cancel")]),
    ];
    popup(f, title, lines, 64, 8);
}

fn draw_help(f: &mut Frame) {
    let k = |a: &str, b: &str| Line::from(vec![Span::styled(format!("  {a:<14}"), Style::default().fg(AMBER)), Span::raw(b.to_string())]);
    let lines = vec![
        Line::from(""),
        k("j k / ↑ ↓", "move"),
        k("Tab, h l", "switch sidebar / list"),
        k("Enter", "open folder · toggle · run"),
        k("Backspace", "go up (Disk Usage)"),
        k("Space", "select item"),
        k("a", "select all / none"),
        k("d", "move selected to Trash (asks first)"),
        k("D", "dry run, deletes nothing"),
        k("o", "reveal in Finder"),
        k("/", "filter (Esc clears)"),
        k("r", "rescan section"),
        k("c p A", "Projects: commit · push · archive"),
        k("q, Ctrl-C", "quit"),
        Line::from(""),
        Line::from(Span::styled("  press any key to close", Style::default().fg(DIM))),
    ];
    popup(f, "macpurge keys", lines, 56, 19);
}
