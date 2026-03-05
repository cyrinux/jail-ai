use crate::error::{JailError, Result};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use portable_pty::{CommandBuilder, PtySize};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Tabs},
    Terminal,
};
use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use tracing::debug;

const SCROLLBACK: usize = 5000;

struct Pane {
    parser: Arc<Mutex<vt100::Parser>>,
    writer: Box<dyn Write + Send>,
    _child: Box<dyn portable_pty::Child + Send + Sync>,
    _pty: Box<dyn portable_pty::MasterPty>,
    title: String,
    done: Arc<std::sync::atomic::AtomicBool>,
}

impl Pane {
    fn new(title: &str, jail_name: &str, command: &[String], size: PtySize) -> Result<Self> {
        let pty_system = portable_pty::native_pty_system();

        let pair = pty_system
            .openpty(size)
            .map_err(|e| JailError::Backend(format!("Failed to open PTY: {e}")))?;

        let mut cmd = CommandBuilder::new("podman");
        cmd.arg("exec");
        cmd.arg("-it");
        cmd.arg(jail_name);
        for arg in command {
            cmd.arg(arg);
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| JailError::Backend(format!("Failed to spawn command: {e}")))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| JailError::Backend(format!("Failed to get PTY writer: {e}")))?;

        let parser = Arc::new(Mutex::new(vt100::Parser::new(
            size.rows, size.cols, SCROLLBACK,
        )));

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| JailError::Backend(format!("Failed to clone PTY reader: {e}")))?;

        let parser_clone = Arc::clone(&parser);
        let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let done_clone = Arc::clone(&done);

        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            let mut reader = reader;
            loop {
                match std::io::Read::read(&mut reader, &mut buf) {
                    Ok(0) => {
                        done_clone.store(true, std::sync::atomic::Ordering::SeqCst);
                        break;
                    }
                    Ok(n) => {
                        if let Ok(mut p) = parser_clone.lock() {
                            p.process(&buf[..n]);
                        }
                    }
                    Err(_) => {
                        done_clone.store(true, std::sync::atomic::Ordering::SeqCst);
                        break;
                    }
                }
            }
        });

        Ok(Self {
            parser,
            writer,
            _child: child,
            _pty: pair.master,
            title: title.to_string(),
            done,
        })
    }

    fn resize(&mut self, rows: u16, cols: u16) {
        let _ = self._pty.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
        if let Ok(mut p) = self.parser.lock() {
            p.screen_mut().set_size(rows, cols);
        }
    }

    fn write_input(&mut self, data: &[u8]) {
        let _ = self.writer.write_all(data);
        let _ = self.writer.flush();
    }

    fn is_done(&self) -> bool {
        self.done.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn render_into_text(&self, rows: u16, cols: u16) -> Vec<Line<'static>> {
        let parser = match self.parser.lock() {
            Ok(p) => p,
            Err(_) => return vec![],
        };

        let screen = parser.screen();
        let mut lines = Vec::with_capacity(rows as usize);

        for row in 0..rows {
            let mut spans: Vec<Span<'static>> = Vec::new();
            let mut current_style = Style::default();
            let mut current_str = String::new();

            for col in 0..cols {
                let (fg, bg, bold, italic, underline, ch) =
                    if let Some(cell) = screen.cell(row, col) {
                        (
                            vt100_color_to_ratatui(cell.fgcolor()),
                            vt100_color_to_ratatui(cell.bgcolor()),
                            cell.bold(),
                            cell.italic(),
                            cell.underline(),
                            {
                                let s = cell.contents().to_string();
                                if s.is_empty() {
                                    " ".to_string()
                                } else {
                                    s
                                }
                            },
                        )
                    } else {
                        (
                            Color::Reset,
                            Color::Reset,
                            false,
                            false,
                            false,
                            " ".to_string(),
                        )
                    };

                let mut style = Style::default().fg(fg).bg(bg);
                if bold {
                    style = style.add_modifier(Modifier::BOLD);
                }
                if italic {
                    style = style.add_modifier(Modifier::ITALIC);
                }
                if underline {
                    style = style.add_modifier(Modifier::UNDERLINED);
                }

                if style == current_style {
                    current_str.push_str(&ch);
                } else {
                    if !current_str.is_empty() {
                        spans.push(Span::styled(current_str.clone(), current_style));
                        current_str.clear();
                    }
                    current_style = style;
                    current_str = ch;
                }
            }

            if !current_str.is_empty() {
                spans.push(Span::styled(current_str, current_style));
            }

            lines.push(Line::from(spans));
        }

        let cursor_row = screen.cursor_position().0;
        let cursor_col = screen.cursor_position().1;
        if (cursor_row as usize) < lines.len() {
            let line = &mut lines[cursor_row as usize];
            let col = cursor_col as usize;
            let spans = &mut line.spans;

            let mut char_pos = 0usize;
            let mut found = false;
            for span in spans.iter_mut() {
                let span_len = span.content.chars().count();
                if char_pos + span_len > col {
                    span.style = span.style.bg(Color::White).fg(Color::Black);
                    found = true;
                    break;
                }
                char_pos += span_len;
            }

            if !found && col == char_pos {
                spans.push(Span::styled(
                    " ",
                    Style::default().bg(Color::White).fg(Color::Black),
                ));
            }
        }

        lines
    }
}

fn vt100_color_to_ratatui(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(0) => Color::Black,
        vt100::Color::Idx(1) => Color::Red,
        vt100::Color::Idx(2) => Color::Green,
        vt100::Color::Idx(3) => Color::Yellow,
        vt100::Color::Idx(4) => Color::Blue,
        vt100::Color::Idx(5) => Color::Magenta,
        vt100::Color::Idx(6) => Color::Cyan,
        vt100::Color::Idx(7) => Color::Gray,
        vt100::Color::Idx(8) => Color::DarkGray,
        vt100::Color::Idx(9) => Color::LightRed,
        vt100::Color::Idx(10) => Color::LightGreen,
        vt100::Color::Idx(11) => Color::LightYellow,
        vt100::Color::Idx(12) => Color::LightBlue,
        vt100::Color::Idx(13) => Color::LightMagenta,
        vt100::Color::Idx(14) => Color::LightCyan,
        vt100::Color::Idx(15) => Color::White,
        vt100::Color::Idx(n) => Color::Indexed(n),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Agent = 0,
    Shell = 1,
}

pub struct Tui {
    jail_name: String,
    agent_command: Vec<String>,
}

impl Tui {
    pub fn new(jail_name: impl Into<String>, agent_command: Vec<String>) -> Self {
        Self {
            jail_name: jail_name.into(),
            agent_command,
        }
    }

    pub fn run(self) -> Result<()> {
        let mut stdout = io::stdout();

        terminal::enable_raw_mode()
            .map_err(|e| JailError::Backend(format!("Failed to enable raw mode: {e}")))?;
        execute!(stdout, EnterAlternateScreen, cursor::Hide)
            .map_err(|e| JailError::Backend(format!("Failed to enter alternate screen: {e}")))?;

        let backend = CrosstermBackend::new(io::stdout());
        let mut terminal = Terminal::new(backend)
            .map_err(|e| JailError::Backend(format!("Failed to create terminal: {e}")))?;

        let result = self.event_loop(&mut terminal);

        terminal::disable_raw_mode()
            .map_err(|e| JailError::Backend(format!("Failed to disable raw mode: {e}")))?;
        execute!(io::stdout(), LeaveAlternateScreen, cursor::Show)
            .map_err(|e| JailError::Backend(format!("Failed to leave alternate screen: {e}")))?;

        result
    }

    fn event_loop(&self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        let size = terminal
            .size()
            .map_err(|e| JailError::Backend(format!("Failed to get terminal size: {e}")))?;

        let pane_rows = size.height.saturating_sub(3);
        let pane_cols = size.width;

        let pty_size = PtySize {
            rows: pane_rows,
            cols: pane_cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        let agent_label = self
            .agent_command
            .first()
            .cloned()
            .unwrap_or_else(|| "agent".to_string());

        let mut agent_pane =
            Pane::new(&agent_label, &self.jail_name, &self.agent_command, pty_size)?;

        let shell_cmd = vec!["/usr/bin/zsh".to_string()];
        let mut shell_pane = Pane::new("shell", &self.jail_name, &shell_cmd, pty_size)?;

        let mut active_tab = Tab::Agent;
        let mut prefix_mode = false;
        let mut current_rows = pane_rows;
        let mut current_cols = pane_cols;

        loop {
            let term_size = terminal
                .size()
                .map_err(|e| JailError::Backend(format!("Failed to get terminal size: {e}")))?;

            let new_rows = term_size.height.saturating_sub(3);
            let new_cols = term_size.width;

            if new_rows != current_rows || new_cols != current_cols {
                agent_pane.resize(new_rows, new_cols);
                shell_pane.resize(new_rows, new_cols);
                current_rows = new_rows;
                current_cols = new_cols;
            }

            if agent_pane.is_done() && shell_pane.is_done() {
                debug!("Both panes exited, leaving TUI");
                break;
            }

            let agent_lines = agent_pane.render_into_text(current_rows, current_cols);
            let shell_lines = shell_pane.render_into_text(current_rows, current_cols);
            let tab_index = active_tab as usize;

            terminal
                .draw(|frame| {
                    let area = frame.area();
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Length(3), Constraint::Min(0)])
                        .split(area);

                    let tab_titles: Vec<Line> = vec![
                        Line::from(format!(" F1 {} ", agent_pane.title.to_uppercase())),
                        Line::from(format!(" F2 {} ", shell_pane.title.to_uppercase())),
                    ];

                    let tabs = Tabs::new(tab_titles)
                        .block(Block::default().borders(Borders::ALL).title(Span::styled(
                            " jail-ai TUI  F1/F2: switch tab  Ctrl+B d: quit ",
                            Style::default().fg(Color::DarkGray),
                        )))
                        .select(tab_index)
                        .highlight_style(
                            Style::default()
                                .fg(Color::Black)
                                .bg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        )
                        .style(Style::default().fg(Color::DarkGray));

                    frame.render_widget(tabs, chunks[0]);

                    let lines = if active_tab == Tab::Agent {
                        &agent_lines
                    } else {
                        &shell_lines
                    };

                    let text = Text::from(lines.clone());
                    let paragraph =
                        Paragraph::new(text).block(Block::default().borders(Borders::NONE));
                    frame.render_widget(paragraph, chunks[1]);

                    if prefix_mode {
                        let msg = Paragraph::new(" Ctrl+B: d quit  1 agent  2 shell ")
                            .style(Style::default().fg(Color::Black).bg(Color::Yellow));
                        let w = 36u16;
                        let h = 1u16;
                        let x = area.width.saturating_sub(w) / 2;
                        let y = area.height.saturating_sub(3);
                        frame.render_widget(msg, Rect::new(x, y, w, h));
                    }
                })
                .map_err(|e| JailError::Backend(format!("Failed to draw frame: {e}")))?;

            // Drain all pending key events in one tick so fast typing never starves TUI keys.
            loop {
                if !event::poll(Duration::from_millis(16))
                    .map_err(|e| JailError::Backend(format!("Event poll failed: {e}")))?
                {
                    break;
                }

                let ev = event::read()
                    .map_err(|e| JailError::Backend(format!("Event read failed: {e}")))?;

                match ev {
                    Event::Key(key) if key.kind != KeyEventKind::Press => {}

                    Event::Key(key) if prefix_mode => {
                        prefix_mode = false;
                        match key.code {
                            KeyCode::Char('d') | KeyCode::Char('D') => {
                                return Ok(());
                            }
                            KeyCode::Char('1') => {
                                active_tab = Tab::Agent;
                            }
                            KeyCode::Char('2') => {
                                active_tab = Tab::Shell;
                            }
                            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                prefix_mode = true;
                            }
                            _ => {
                                let bytes = key_event_to_bytes(key);
                                active_pane_mut(&mut agent_pane, &mut shell_pane, active_tab)
                                    .write_input(&bytes);
                            }
                        }
                    }

                    Event::Key(KeyEvent {
                        code: KeyCode::Char('b'),
                        modifiers,
                        kind: KeyEventKind::Press,
                        ..
                    }) if modifiers.contains(KeyModifiers::CONTROL) => {
                        prefix_mode = true;
                    }

                    Event::Key(KeyEvent {
                        code: KeyCode::F(1),
                        kind: KeyEventKind::Press,
                        ..
                    }) => {
                        active_tab = Tab::Agent;
                    }

                    Event::Key(KeyEvent {
                        code: KeyCode::F(2),
                        kind: KeyEventKind::Press,
                        ..
                    }) => {
                        active_tab = Tab::Shell;
                    }

                    Event::Key(key) => {
                        let bytes = key_event_to_bytes(key);
                        if !bytes.is_empty() {
                            active_pane_mut(&mut agent_pane, &mut shell_pane, active_tab)
                                .write_input(&bytes);
                        }
                    }

                    Event::Resize(cols, rows) => {
                        let new_rows = rows.saturating_sub(3);
                        agent_pane.resize(new_rows, cols);
                        shell_pane.resize(new_rows, cols);
                        current_rows = new_rows;
                        current_cols = cols;
                    }

                    _ => {}
                }
            }
        }

        Ok(())
    }
}

fn active_pane_mut<'a>(agent: &'a mut Pane, shell: &'a mut Pane, tab: Tab) -> &'a mut Pane {
    if tab == Tab::Agent {
        agent
    } else {
        shell
    }
}

fn key_event_to_bytes(key: KeyEvent) -> Vec<u8> {
    match key.code {
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                let c = c.to_ascii_lowercase();
                if c.is_ascii_lowercase() {
                    vec![(c as u8) - b'a' + 1]
                } else if c == ' ' {
                    vec![0]
                } else {
                    c.to_string().into_bytes()
                }
            } else if key.modifiers.contains(KeyModifiers::ALT) {
                let mut v = vec![0x1b];
                v.extend(c.to_string().into_bytes());
                v
            } else {
                c.to_string().into_bytes()
            }
        }
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Delete => vec![0x1b, b'[', b'3', b'~'],
        KeyCode::Tab => vec![b'\t'],
        KeyCode::BackTab => vec![0x1b, b'[', b'Z'],
        KeyCode::Esc => vec![0x1b],
        KeyCode::Up => vec![0x1b, b'[', b'A'],
        KeyCode::Down => vec![0x1b, b'[', b'B'],
        KeyCode::Right => vec![0x1b, b'[', b'C'],
        KeyCode::Left => vec![0x1b, b'[', b'D'],
        KeyCode::Home => vec![0x1b, b'[', b'H'],
        KeyCode::End => vec![0x1b, b'[', b'F'],
        KeyCode::PageUp => vec![0x1b, b'[', b'5', b'~'],
        KeyCode::PageDown => vec![0x1b, b'[', b'6', b'~'],
        KeyCode::Insert => vec![0x1b, b'[', b'2', b'~'],
        KeyCode::F(1) => vec![0x1b, b'O', b'P'],
        KeyCode::F(2) => vec![0x1b, b'O', b'Q'],
        KeyCode::F(3) => vec![0x1b, b'O', b'R'],
        KeyCode::F(4) => vec![0x1b, b'O', b'S'],
        KeyCode::F(n) if n >= 5 => {
            vec![0x1b, b'[', b'1', b'5' + (n - 5), b'~']
        }
        _ => vec![],
    }
}
