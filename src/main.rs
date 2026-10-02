use std::{
    env,
    ffi::OsString,
    fs,
    fs::File,
    io::{self, BufWriter},
    path::{Path, PathBuf},
    process::{Command, ExitCode},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use cpal::{
    Device, SampleFormat, Stream, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Position, Rect},
    prelude::{CrosstermBackend, Frame, Terminal},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};
use serde::{Deserialize, Serialize};

const APP_NAME: &str = "taodio";
const CONFIG_DIR: &str = ".config/taodio";
const CONFIG_FILE: &str = "config.json";
const FORMATS: &[&str] = &["m4a", "aiff"];
const LANGUAGES: &[(&str, &str)] = &[("zh", "中文"), ("ja", "日本語"), ("en", "English")];
const TITLE_ART: &[&str] = &[
    " ___                  ",
    "  |  _.  _   _| o  _  ",
    "  | (_| (_) (_| | (_) ",
    "                      ",
];
const ACCENT: Color = Color::Blue;
const TEXT: Color = Color::White;
const MUTED: Color = Color::White;
const BORDER: Color = Color::White;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{APP_NAME}: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args_os().skip(1);
    let command = args.next();

    match command.as_ref().map(|arg| arg.to_string_lossy()).as_deref() {
        None => run_tui(),
        Some("record") => {
            let config = ensure_config()?;
            let opts = StopOptions::parse(args, &config)?;
            record_until_enter(opts, lang_from_code(&config.settings.language))
        }
        Some("start") => {
            Err("`taodio start` is deprecated for the native recorder. Use `taodio record` or open `taodio`.".to_string())
        }
        Some("stop") => {
            Err("`taodio stop` is deprecated for the native recorder. Use `taodio record` or open `taodio`.".to_string())
        }
        Some("status") => status(),
        Some("devices") => print_devices(),
        Some("path") => {
            let config = ensure_config()?;
            let opts = PathOptions::parse(args, &config)?;
            println!("{}", opts.output_path()?.display());
            Ok(())
        }
        Some("init") => {
            save_config(&default_config())?;
            println!(
                "{} {}",
                tr(Lang::Zh, "config_written"),
                config_path()?.display()
            );
            Ok(())
        }
        Some("config") => {
            ensure_config()?;
            println!("{}", config_path()?.display());
            Ok(())
        }
        Some("help" | "-h" | "--help") => {
            print_help();
            Ok(())
        }
        Some(other) => Err(format!("unknown command `{other}`\n\nRun `taodio help`.")),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    version: u8,
    settings: Settings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Settings {
    language: String,
    save_format: String,
    #[serde(default)]
    input_device: Option<String>,
}

fn default_config() -> Config {
    Config {
        version: 1,
        settings: Settings {
            language: "zh".to_string(),
            save_format: "m4a".to_string(),
            input_device: None,
        },
    }
}

#[derive(Clone, Copy)]
enum Lang {
    Zh,
    Ja,
    En,
}

fn lang_from_code(code: &str) -> Lang {
    match code {
        "ja" => Lang::Ja,
        "en" => Lang::En,
        _ => Lang::Zh,
    }
}

fn language_label(code: &str) -> &'static str {
    LANGUAGES
        .iter()
        .find(|(candidate, _)| *candidate == code)
        .map(|(_, label)| *label)
        .unwrap_or("中文")
}

fn tr(lang: Lang, key: &str) -> &'static str {
    match (lang, key) {
        (Lang::Zh, "menu_record") => "开始录音",
        (Lang::Zh, "menu_settings") => "设置",
        (Lang::Zh, "menu_config") => "查看配置",
        (Lang::Zh, "menu_quit") => "退出",
        (Lang::Zh, "help") => "↑/↓ 选择 · Enter 确认 · q 退出",
        (Lang::Zh, "record_title") => "录音中",
        (Lang::Zh, "press_enter") => "按 Enter 停止并保存",
        (Lang::Zh, "recording_started") => "已开始录音",
        (Lang::Zh, "saved") => "已保存",
        (Lang::Zh, "status") => "状态",
        (Lang::Zh, "format") => "保存格式",
        (Lang::Zh, "language") => "语言",
        (Lang::Zh, "input_device") => "输入设备",
        (Lang::Zh, "system_default") => "系统默认",
        (Lang::Zh, "settings_hint") => "←/→ 修改，变更会立即写入 ~/.config/taodio/config.json",
        (Lang::Zh, "settings_footer") => "q/Esc 返回",
        (Lang::Zh, "config_title") => "配置",
        (Lang::Zh, "config_written") => "已写入配置:",
        (Lang::Zh, "default_dir") => "默认保存目录",
        (Lang::Zh, "current_dir") => "当前打开目录",
        (Lang::Zh, "idle") => "空闲",
        (Lang::Zh, "open") => "有录音窗口",
        (Lang::Zh, "stop_hint") => "停止并保存: taodio stop",
        (Lang::Zh, "saving") => "正在保存",
        (Lang::Zh, "rename_title") => "重命名",
        (Lang::Zh, "filename") => "文件名",
        (Lang::Zh, "rename_hint") => "Enter 确认 · Esc 保留原名",

        (Lang::Ja, "menu_record") => "録音開始",
        (Lang::Ja, "menu_settings") => "設定",
        (Lang::Ja, "menu_config") => "設定を表示",
        (Lang::Ja, "menu_quit") => "終了",
        (Lang::Ja, "help") => "↑/↓ 選択 · Enter 決定 · q 終了",
        (Lang::Ja, "record_title") => "録音中",
        (Lang::Ja, "press_enter") => "Enter で停止して保存",
        (Lang::Ja, "recording_started") => "録音を開始しました",
        (Lang::Ja, "saved") => "保存しました",
        (Lang::Ja, "status") => "状態",
        (Lang::Ja, "format") => "保存形式",
        (Lang::Ja, "language") => "言語",
        (Lang::Ja, "input_device") => "入力デバイス",
        (Lang::Ja, "system_default") => "システム既定",
        (Lang::Ja, "settings_hint") => "←/→ で変更。~/.config/taodio/config.json に保存されます",
        (Lang::Ja, "settings_footer") => "q/Esc 戻る",
        (Lang::Ja, "config_title") => "設定",
        (Lang::Ja, "config_written") => "設定を書き込みました:",
        (Lang::Ja, "default_dir") => "既定の保存先",
        (Lang::Ja, "current_dir") => "現在のディレクトリ",
        (Lang::Ja, "idle") => "待機中",
        (Lang::Ja, "open") => "録音ウィンドウあり",
        (Lang::Ja, "stop_hint") => "停止して保存: taodio stop",
        (Lang::Ja, "saving") => "保存中",
        (Lang::Ja, "rename_title") => "名前を変更",
        (Lang::Ja, "filename") => "ファイル名",
        (Lang::Ja, "rename_hint") => "Enter 決定 · Esc 元の名前を保持",

        (Lang::En, "menu_record") => "Start recording",
        (Lang::En, "menu_settings") => "Settings",
        (Lang::En, "menu_config") => "View config",
        (Lang::En, "menu_quit") => "Quit",
        (Lang::En, "help") => "↑/↓ select · Enter confirm · q quit",
        (Lang::En, "record_title") => "Recording",
        (Lang::En, "press_enter") => "Press Enter to stop and save",
        (Lang::En, "recording_started") => "Recording started",
        (Lang::En, "saved") => "Saved",
        (Lang::En, "status") => "Status",
        (Lang::En, "format") => "Save format",
        (Lang::En, "language") => "Language",
        (Lang::En, "input_device") => "Input device",
        (Lang::En, "system_default") => "System default",
        (Lang::En, "settings_hint") => "Use ←/→ to change; saved to ~/.config/taodio/config.json",
        (Lang::En, "settings_footer") => "q/Esc back",
        (Lang::En, "config_title") => "Config",
        (Lang::En, "config_written") => "Config written:",
        (Lang::En, "default_dir") => "Default save directory",
        (Lang::En, "current_dir") => "current directory",
        (Lang::En, "idle") => "idle",
        (Lang::En, "open") => "recording window open",
        (Lang::En, "stop_hint") => "Stop and save: taodio stop",
        (Lang::En, "saving") => "Saving",
        (Lang::En, "rename_title") => "Rename",
        (Lang::En, "filename") => "Filename",
        (Lang::En, "rename_hint") => "Enter confirm · Esc keep original",
        _ => "",
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Menu,
    Recording,
    Rename,
    Settings,
    ConfigView,
}

struct App {
    config: Config,
    screen: Screen,
    menu_index: usize,
    settings_index: usize,
    message: String,
    should_quit: bool,
    recording_started: bool,
    saved_path: Option<PathBuf>,
    active_recording: Option<ActiveRecording>,
    cancel_confirmation: bool,
    cancel_selected: bool,
    rename_input: String,
}

impl App {
    fn new(config: Config) -> Self {
        Self {
            config,
            screen: Screen::Menu,
            menu_index: 0,
            settings_index: 0,
            message: String::new(),
            should_quit: false,
            recording_started: false,
            saved_path: None,
            active_recording: None,
            cancel_confirmation: false,
            cancel_selected: false,
            rename_input: String::new(),
        }
    }

    fn lang(&self) -> Lang {
        lang_from_code(&self.config.settings.language)
    }
}

struct ActiveRecording {
    stream: Stream,
    writer: SharedWavWriter,
    temp_path: PathBuf,
    output_path: PathBuf,
}

type SharedWavWriter = Arc<Mutex<Option<hound::WavWriter<BufWriter<File>>>>>;

fn run_tui() -> Result<(), String> {
    let config = ensure_config()?;
    let mut app = App::new(config);
    enable_raw_mode().map_err(|err| err.to_string())?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).map_err(|err| err.to_string())?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|err| err.to_string())?;

    let result = app_loop(&mut terminal, &mut app);

    disable_raw_mode().map_err(|err| err.to_string())?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen).map_err(|err| err.to_string())?;
    terminal.show_cursor().map_err(|err| err.to_string())?;
    result
}

fn app_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<(), String> {
    while !app.should_quit {
        terminal
            .draw(|frame| draw(frame, app))
            .map_err(|err| err.to_string())?;
        if let Event::Key(key) = event::read().map_err(|err| err.to_string())? {
            if key.kind != KeyEventKind::Release {
                handle_key(app, key)?;
            }
        }
    }
    Ok(())
}

fn handle_key(app: &mut App, key: KeyEvent) -> Result<(), String> {
    let code = key.code;
    if app.cancel_confirmation {
        return handle_cancel_key(app, code);
    }
    match app.screen {
        Screen::Menu => handle_menu_key(app, code),
        Screen::Recording => handle_recording_key(app, code),
        Screen::Rename => handle_rename_key(app, key),
        Screen::Settings => handle_settings_key(app, code),
        Screen::ConfigView => {
            if matches!(code, KeyCode::Char('q') | KeyCode::Esc) {
                app.screen = Screen::Menu;
            }
            Ok(())
        }
    }
}

fn handle_menu_key(app: &mut App, code: KeyCode) -> Result<(), String> {
    let item_count = 4;
    match code {
        KeyCode::Up => app.menu_index = app.menu_index.saturating_sub(1),
        KeyCode::Down => app.menu_index = (app.menu_index + 1).min(item_count - 1),
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Enter => match app.menu_index {
            0 => {
                let opts = StopOptions::from_config(&app.config);
                let recording = start_native_recording(opts)?;
                app.recording_started = true;
                app.active_recording = Some(recording);
                app.saved_path = None;
                app.message = tr(app.lang(), "recording_started").to_string();
                app.screen = Screen::Recording;
            }
            1 => app.screen = Screen::Settings,
            2 => app.screen = Screen::ConfigView,
            _ => app.should_quit = true,
        },
        _ => {}
    }
    Ok(())
}

fn handle_recording_key(app: &mut App, code: KeyCode) -> Result<(), String> {
    match code {
        KeyCode::Esc if app.recording_started => {
            app.cancel_confirmation = true;
            app.cancel_selected = false;
        }
        KeyCode::Enter if app.recording_started => {
            app.message = tr(app.lang(), "saving").to_string();
            let recording = app
                .active_recording
                .take()
                .ok_or_else(|| "recording state was lost".to_string())?;
            let path = finish_native_recording(recording)?;
            app.saved_path = Some(path.clone());
            app.rename_input = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_default()
                .to_string();
            app.message = format!("{}: {}", tr(app.lang(), "saved"), path.display());
            app.recording_started = false;
            app.screen = Screen::Rename;
        }
        KeyCode::Char('q') | KeyCode::Esc if !app.recording_started => app.screen = Screen::Menu,
        _ => {}
    }
    Ok(())
}

fn handle_rename_key(app: &mut App, key: KeyEvent) -> Result<(), String> {
    let code = key.code;
    if (matches!(code, KeyCode::Char('u' | 'U')) && key.modifiers.contains(KeyModifiers::CONTROL))
        || (code == KeyCode::Backspace
            && key
                .modifiers
                .intersects(KeyModifiers::SUPER | KeyModifiers::CONTROL))
    {
        app.rename_input.clear();
        return Ok(());
    }
    match code {
        KeyCode::Enter => {
            if let Some(path) = app.saved_path.clone() {
                match rename_recording(&path, &app.rename_input) {
                    Ok(renamed) => {
                        app.saved_path = Some(renamed.clone());
                        app.message = format!("{}: {}", tr(app.lang(), "saved"), renamed.display());
                        app.screen = Screen::Menu;
                    }
                    Err(err) => {
                        app.message = err;
                    }
                }
            } else {
                app.screen = Screen::Menu;
            }
        }
        KeyCode::Esc => app.screen = Screen::Menu,
        KeyCode::Backspace => {
            app.rename_input.pop();
        }
        KeyCode::Char(ch)
            if !matches!(ch, '/' | ':')
                && !key.modifiers.intersects(
                    KeyModifiers::CONTROL | KeyModifiers::SUPER | KeyModifiers::ALT,
                ) =>
        {
            app.rename_input.push(ch);
        }
        _ => {}
    }
    Ok(())
}

fn handle_settings_key(app: &mut App, code: KeyCode) -> Result<(), String> {
    match code {
        KeyCode::Up => app.settings_index = app.settings_index.saturating_sub(1),
        KeyCode::Down => app.settings_index = (app.settings_index + 1).min(2),
        KeyCode::Left => change_setting(app, -1)?,
        KeyCode::Right => change_setting(app, 1)?,
        KeyCode::Char('q') | KeyCode::Esc => app.screen = Screen::Menu,
        _ => {}
    }
    Ok(())
}

fn handle_cancel_key(app: &mut App, code: KeyCode) -> Result<(), String> {
    match code {
        KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
            app.cancel_selected = !app.cancel_selected;
        }
        KeyCode::Esc => app.cancel_confirmation = false,
        KeyCode::Enter if !app.cancel_selected => app.cancel_confirmation = false,
        KeyCode::Enter => {
            if let Some(recording) = app.active_recording.take() {
                drop(recording.stream);
                recording
                    .writer
                    .lock()
                    .map_err(|_| "recording writer lock was poisoned".to_string())?
                    .take();
                fs::remove_file(&recording.temp_path)
                    .map_err(|err| format!("failed to discard recording: {err}"))?;
            }
            app.recording_started = false;
            app.cancel_confirmation = false;
            app.screen = Screen::Menu;
        }
        _ => {}
    }
    Ok(())
}

fn draw_cancel_confirmation(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let (title, message, keep, discard) = match app.lang() {
        Lang::Zh => ("取消录音？", "当前录音将被丢弃。", "继续录音", "丢弃录音"),
        Lang::Ja => (
            "録音を中止しますか？",
            "現在の録音は破棄されます。",
            "録音を続ける",
            "録音を破棄",
        ),
        Lang::En => (
            "Cancel recording?",
            "This recording will be discarded.",
            "Continue",
            "Discard",
        ),
    };
    let popup = centered(area, 48, 9);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BORDER));
    frame.render_widget(block, popup);
    let options = Line::from(vec![
        Span::styled(
            format!("{} {keep}", if !app.cancel_selected { ">" } else { " " }),
            Style::default().fg(if !app.cancel_selected { ACCENT } else { TEXT }),
        ),
        Span::raw("     "),
        Span::styled(
            format!("{} {discard}", if app.cancel_selected { ">" } else { " " }),
            Style::default().fg(if app.cancel_selected { ACCENT } else { TEXT }),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(
                title,
                Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
            ),
            Line::from(""),
            Line::from(message),
            Line::from(""),
            options,
        ])
        .style(Style::default().fg(TEXT))
        .alignment(Alignment::Center),
        Rect {
            x: popup.x + 1,
            y: popup.y + 2,
            width: popup.width.saturating_sub(2),
            height: popup.height.saturating_sub(3),
        },
    );
}

fn change_setting(app: &mut App, delta: isize) -> Result<(), String> {
    match app.settings_index {
        0 => {
            let devices = input_device_options();
            let current =
                current_device_index(&devices, app.config.settings.input_device.as_deref());
            let index = wrap(current as isize + delta, devices.len());
            app.config.settings.input_device = devices[index].clone();
        }
        1 => {
            let index = cycle_index(FORMATS, &app.config.settings.save_format, delta);
            app.config.settings.save_format = FORMATS[index].to_string();
        }
        _ => {
            let current = LANGUAGES
                .iter()
                .position(|(code, _)| *code == app.config.settings.language)
                .unwrap_or(0);
            let index = wrap(current as isize + delta, LANGUAGES.len());
            app.config.settings.language = LANGUAGES[index].0.to_string();
        }
    }
    save_config(&app.config)?;
    Ok(())
}

fn draw(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    frame.render_widget(Clear, area);
    match app.screen {
        Screen::Menu => draw_menu(frame, app, area),
        Screen::Recording => draw_recording(frame, app, area),
        Screen::Rename => draw_rename(frame, app, area),
        Screen::Settings => draw_settings(frame, app, area),
        Screen::ConfigView => draw_config(frame, app, area),
    }
    if app.cancel_confirmation {
        draw_cancel_confirmation(frame, app, area);
    }
}

fn draw_menu(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let lang = app.lang();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Length(8),
            Constraint::Length(3),
        ])
        .split(centered(area, 38, 17));

    draw_title(frame, chunks[0]);

    let items = [
        tr(lang, "menu_record"),
        tr(lang, "menu_settings"),
        tr(lang, "menu_config"),
        tr(lang, "menu_quit"),
    ];
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(BORDER)),
        chunks[1],
    );
    let start_y = chunks[1].y + 1;
    for (index, item) in items.iter().enumerate() {
        let selected = index == app.menu_index;
        let line = Line::from(vec![
            Span::styled(
                if selected { "> " } else { "  " },
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                *item,
                if selected {
                    Style::default().fg(TEXT).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(TEXT)
                },
            ),
        ]);
        frame.render_widget(
            Paragraph::new(line),
            Rect {
                x: chunks[1].x + 4,
                y: start_y + index as u16,
                width: chunks[1].width.saturating_sub(8),
                height: 1,
            },
        );
    }

    frame.render_widget(
        Paragraph::new(Span::styled(tr(lang, "help"), Style::default().fg(MUTED)))
            .alignment(Alignment::Center),
        chunks[2],
    );
}

fn draw_recording(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let lang = app.lang();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Min(6),
            Constraint::Length(3),
        ])
        .split(centered(area, 72, 18));

    draw_title(frame, chunks[0]);
    let status = if app.recording_started {
        tr(lang, "press_enter").to_string()
    } else {
        app.saved_path
            .as_ref()
            .map(|path| format!("{}: {}", tr(lang, "saved"), path.display()))
            .unwrap_or_else(|| app.message.clone())
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                tr(lang, "record_title"),
                Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(status),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        )
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true }),
        chunks[1],
    );
    let footer = if app.recording_started {
        tr(lang, "press_enter")
    } else {
        tr(lang, "settings_footer")
    };
    frame.render_widget(
        Paragraph::new(footer).alignment(Alignment::Center),
        chunks[2],
    );
}

fn draw_rename(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let lang = app.lang();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Length(3),
            Constraint::Length(7),
            Constraint::Length(3),
        ])
        .split(centered(area, 58, 19));

    draw_title(frame, chunks[0]);
    frame.render_widget(
        Paragraph::new(Span::styled(
            tr(lang, "rename_title"),
            Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
        chunks[1],
    );

    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(BORDER)),
        chunks[2],
    );
    let label_area = Rect {
        x: chunks[2].x + 4,
        y: chunks[2].y + 2,
        width: 12,
        height: 1,
    };
    frame.render_widget(
        Paragraph::new(Span::styled(
            tr(lang, "filename"),
            Style::default().fg(MUTED),
        )),
        label_area,
    );
    let input_area = Rect {
        x: chunks[2].x + 18,
        y: chunks[2].y + 2,
        width: chunks[2].width.saturating_sub(24),
        height: 1,
    };
    let mut visible_input = app.rename_input.as_str();
    while Line::from(visible_input).width() >= input_area.width as usize
        && !visible_input.is_empty()
    {
        visible_input = &visible_input[visible_input.chars().next().unwrap().len_utf8()..];
    }
    frame.render_widget(
        Paragraph::new(Span::styled(visible_input, Style::default().fg(TEXT))),
        input_area,
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            "─".repeat(input_area.width as usize),
            Style::default().fg(BORDER),
        )),
        Rect {
            x: input_area.x,
            y: input_area.y + 1,
            width: input_area.width,
            height: 1,
        },
    );
    let cursor_x = input_area
        .x
        .saturating_add(Line::from(visible_input).width() as u16)
        .min(input_area.x + input_area.width.saturating_sub(1));
    frame.set_cursor_position(Position {
        x: cursor_x,
        y: input_area.y,
    });
    frame.render_widget(
        Paragraph::new(Span::styled(
            tr(lang, "rename_hint"),
            Style::default().fg(MUTED),
        ))
        .alignment(Alignment::Center),
        chunks[3],
    );
    if !app.message.is_empty() {
        let msg_area = Rect {
            x: chunks[2].x + 4,
            y: chunks[2].y + 4,
            width: chunks[2].width.saturating_sub(8),
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(Span::styled(&app.message, Style::default().fg(MUTED)))
                .alignment(Alignment::Center),
            msg_area,
        );
    }
}

fn draw_settings(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let lang = app.lang();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Length(3),
            Constraint::Length(9),
            Constraint::Length(4),
        ])
        .split(centered(area, 66, 22));

    draw_title(frame, chunks[0]);
    frame.render_widget(
        Paragraph::new(Span::styled(
            tr(lang, "menu_settings"),
            Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
        chunks[1],
    );

    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(BORDER)),
        chunks[2],
    );
    let device_value = device_label(lang, app.config.settings.input_device.as_deref());
    let rows = [
        (tr(lang, "input_device"), device_value.as_str()),
        (tr(lang, "format"), app.config.settings.save_format.as_str()),
        (
            tr(lang, "language"),
            language_label(&app.config.settings.language),
        ),
    ];
    for (index, (label, value)) in rows.iter().enumerate() {
        draw_setting_row(
            frame,
            chunks[2],
            index as u16,
            label,
            value,
            app.settings_index == index,
        );
    }
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                tr(lang, "settings_hint"),
                Style::default().fg(MUTED),
            )),
            Line::from(Span::styled(
                tr(lang, "settings_footer"),
                Style::default().fg(MUTED),
            )),
        ])
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true }),
        chunks[3],
    );
}

fn draw_setting_row(
    frame: &mut Frame<'_>,
    area: Rect,
    index: u16,
    label: &str,
    value: &str,
    selected: bool,
) {
    let y = area.y + 2 + index * 2;
    let marker_style = Style::default().fg(ACCENT).add_modifier(Modifier::BOLD);
    let text_style = if selected {
        Style::default().fg(TEXT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(TEXT)
    };
    let value_style = if selected {
        Style::default().fg(TEXT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(MUTED)
    };
    frame.render_widget(
        Paragraph::new(Span::styled(if selected { ">" } else { " " }, marker_style)),
        Rect {
            x: area.x + 3,
            y,
            width: 2,
            height: 1,
        },
    );
    frame.render_widget(
        Paragraph::new(Span::styled(label.to_string(), text_style)),
        Rect {
            x: area.x + 7,
            y,
            width: 18,
            height: 1,
        },
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[ ", Style::default().fg(MUTED)),
            Span::styled(value.to_string(), value_style),
            Span::styled(" ]", Style::default().fg(MUTED)),
        ])),
        Rect {
            x: area.x + 30,
            y,
            width: area.width.saturating_sub(34),
            height: 1,
        },
    );
}

fn input_device_options() -> Vec<Option<String>> {
    let mut options = vec![None];
    options.extend(input_device_names().into_iter().map(Some));
    options
}

fn input_device_names() -> Vec<String> {
    let host = cpal::default_host();
    let mut names = host
        .input_devices()
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|device| device.name().ok())
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names
}

fn print_devices() -> Result<(), String> {
    let names = input_device_names();
    if names.is_empty() {
        println!("No input devices found.");
        return Ok(());
    }
    for name in names {
        println!("{name}");
    }
    Ok(())
}

fn current_device_index(devices: &[Option<String>], current: Option<&str>) -> usize {
    devices
        .iter()
        .position(|device| device.as_deref() == current)
        .unwrap_or(0)
}

fn device_label(lang: Lang, device: Option<&str>) -> String {
    device
        .map(ToString::to_string)
        .unwrap_or_else(|| tr(lang, "system_default").to_string())
}

fn draw_config(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let lang = app.lang();
    let text = serde_json::to_string_pretty(&app.config).unwrap_or_else(|_| "{}".to_string());
    frame.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(tr(lang, "config_title")),
            )
            .wrap(Wrap { trim: false }),
        centered(area, 84, 24),
    );
}

fn draw_title(frame: &mut Frame<'_>, area: Rect) {
    let lines = TITLE_ART
        .iter()
        .map(|line| {
            Line::from(Span::styled(
                *line,
                Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
            ))
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), area);
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    let left = (area.width.saturating_sub(width)) / 2;
    let top = (area.height.saturating_sub(height)) / 2;
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(top),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(left),
            Constraint::Length(width),
            Constraint::Min(0),
        ])
        .split(vertical[1])[1]
}

fn cycle_index(values: &[&str], current: &str, delta: isize) -> usize {
    let index = values
        .iter()
        .position(|value| *value == current)
        .unwrap_or(0);
    wrap(index as isize + delta, values.len())
}

fn wrap(value: isize, len: usize) -> usize {
    value.rem_euclid(len as isize) as usize
}

#[derive(Debug)]
struct StopOptions {
    path: PathOptions,
    force: bool,
}

impl StopOptions {
    fn from_config(config: &Config) -> Self {
        Self {
            path: PathOptions::from_config(config),
            force: false,
        }
    }

    fn parse(args: impl Iterator<Item = OsString>, config: &Config) -> Result<Self, String> {
        let mut path = PathOptions::from_config(config);
        let mut force = false;
        let mut iter = args.peekable();

        while let Some(arg) = iter.next() {
            match arg.to_string_lossy().as_ref() {
                "-o" | "--output" => path.output = Some(next_value(&mut iter, "--output")?),
                "--dir" => path.dir = Some(next_value(&mut iter, "--dir")?),
                "--name" => path.name = Some(next_value(&mut iter, "--name")?),
                "--format" => path.format = Some(parse_format(next_value(&mut iter, "--format")?)?),
                "--device" => {
                    path.device = Some(
                        next_value(&mut iter, "--device")?
                            .to_string_lossy()
                            .to_string(),
                    )
                }
                "-f" | "--force" => force = true,
                other => return Err(format!("unknown stop option `{other}`")),
            }
        }

        Ok(Self { path, force })
    }
}

#[derive(Debug, Default)]
struct PathOptions {
    output: Option<OsString>,
    dir: Option<OsString>,
    name: Option<OsString>,
    format: Option<String>,
    device: Option<String>,
}

impl PathOptions {
    fn from_config(config: &Config) -> Self {
        Self {
            format: Some(config.settings.save_format.clone()),
            device: config.settings.input_device.clone(),
            ..Self::default()
        }
    }

    fn parse(args: impl Iterator<Item = OsString>, config: &Config) -> Result<Self, String> {
        let mut opts = Self::from_config(config);
        let mut iter = args.peekable();

        while let Some(arg) = iter.next() {
            match arg.to_string_lossy().as_ref() {
                "-o" | "--output" => opts.output = Some(next_value(&mut iter, "--output")?),
                "--dir" => opts.dir = Some(next_value(&mut iter, "--dir")?),
                "--name" => opts.name = Some(next_value(&mut iter, "--name")?),
                "--format" => opts.format = Some(parse_format(next_value(&mut iter, "--format")?)?),
                "--device" => {
                    opts.device = Some(
                        next_value(&mut iter, "--device")?
                            .to_string_lossy()
                            .to_string(),
                    )
                }
                other => return Err(format!("unknown path option `{other}`")),
            }
        }

        Ok(opts)
    }

    fn output_path(&self) -> Result<PathBuf, String> {
        if let Some(output) = &self.output {
            return Ok(expand_home(PathBuf::from(output)));
        }

        let dir = self.dir.as_ref().map(PathBuf::from).unwrap_or(
            env::current_dir().map_err(|err| format!("failed to read current directory: {err}"))?,
        );
        let name = self
            .name
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| default_filename(self.format()));

        Ok(expand_home(dir).join(ensure_extension(name, self.format())))
    }

    fn format(&self) -> &str {
        self.format.as_deref().unwrap_or("m4a")
    }
}

fn parse_format(value: OsString) -> Result<String, String> {
    let format = value.to_string_lossy().trim().to_ascii_lowercase();
    if FORMATS.contains(&format.as_str()) {
        Ok(format)
    } else {
        Err(format!("unsupported format `{format}`. Use m4a or aiff."))
    }
}

fn ensure_extension(mut path: PathBuf, format: &str) -> PathBuf {
    if path.extension().is_none() {
        path.set_extension(format);
    }
    path
}

fn rename_recording(path: &Path, input: &str) -> Result<PathBuf, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(path.to_path_buf());
    }
    let parent = path
        .parent()
        .ok_or_else(|| format!("invalid recording path: {}", path.display()))?;
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("m4a");
    let mut target = parent.join(trimmed);
    if target.extension().is_none() {
        target.set_extension(extension);
    }
    if target == path {
        return Ok(path.to_path_buf());
    }
    if target.exists() {
        return Err(format!("{} already exists", target.display()));
    }
    fs::rename(path, &target)
        .map_err(|err| format!("failed to rename {}: {err}", path.display()))?;
    Ok(target)
}

fn record_until_enter(opts: StopOptions, lang: Lang) -> Result<(), String> {
    let recording = start_native_recording(opts)?;
    println!("{}", tr(lang, "recording_started"));
    println!("{}", tr(lang, "press_enter"));

    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .map_err(|err| format!("failed to wait for Enter: {err}"))?;

    println!("{}", tr(lang, "saving"));
    let path = finish_native_recording(recording)?;
    println!("{} {}", tr(lang, "saved"), path.display());
    Ok(())
}

fn start_native_recording(opts: StopOptions) -> Result<ActiveRecording, String> {
    let output_path = opts.path.output_path()?;
    if output_path.exists() && !opts.force {
        return Err(format!(
            "{} already exists. Use --force to overwrite it.",
            output_path.display()
        ));
    }

    let parent = output_path
        .parent()
        .ok_or_else(|| format!("invalid output path: {}", output_path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;

    let device = choose_input_device(opts.path.device.as_deref())?;
    let supported = device
        .default_input_config()
        .map_err(|err| {
            format!(
                "failed to read input device config: {err}. Check macOS microphone permission for your terminal app."
            )
        })?;
    let sample_format = supported.sample_format();
    let stream_config = supported.config();
    let temp_path = temp_wav_path();
    let spec = hound::WavSpec {
        channels: stream_config.channels,
        sample_rate: stream_config.sample_rate.0,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let writer = hound::WavWriter::create(&temp_path, spec)
        .map_err(|err| format!("failed to create temporary recording file: {err}"))?;
    let writer = Arc::new(Mutex::new(Some(writer)));
    let stream = build_input_stream(&device, &stream_config, sample_format, writer.clone())?;
    stream
        .play()
        .map_err(|err| format!("failed to start input stream: {err}"))?;

    Ok(ActiveRecording {
        stream,
        writer,
        temp_path,
        output_path,
    })
}

fn finish_native_recording(recording: ActiveRecording) -> Result<PathBuf, String> {
    drop(recording.stream);
    if let Some(writer) = recording
        .writer
        .lock()
        .map_err(|_| "recording writer lock was poisoned".to_string())?
        .take()
    {
        writer
            .finalize()
            .map_err(|err| format!("failed to finalize audio data: {err}"))?;
    }
    convert_audio(&recording.temp_path, &recording.output_path)?;
    let _ = fs::remove_file(&recording.temp_path);
    Ok(recording.output_path)
}

fn choose_input_device(name: Option<&str>) -> Result<Device, String> {
    let host = cpal::default_host();
    if let Some(name) = name {
        for device in host
            .input_devices()
            .map_err(|err| format!("failed to list input devices: {err}"))?
        {
            if device.name().ok().as_deref() == Some(name) {
                return Ok(device);
            }
        }
        return Err(format!("input device `{name}` was not found"));
    }
    host.default_input_device()
        .ok_or_else(|| "no default input device found".to_string())
}

fn build_input_stream(
    device: &Device,
    config: &StreamConfig,
    sample_format: SampleFormat,
    writer: SharedWavWriter,
) -> Result<Stream, String> {
    let err_fn = |err| eprintln!("{APP_NAME}: input stream error: {err}");
    match sample_format {
        SampleFormat::F32 => device
            .build_input_stream(
                config,
                move |data: &[f32], _| write_f32_samples(data, &writer),
                err_fn,
                None,
            )
            .map_err(|err| format!("failed to build input stream: {err}")),
        SampleFormat::I16 => device
            .build_input_stream(
                config,
                move |data: &[i16], _| {
                    let converted = data
                        .iter()
                        .map(|sample| *sample as f32 / i16::MAX as f32)
                        .collect::<Vec<_>>();
                    write_f32_samples(&converted, &writer);
                },
                err_fn,
                None,
            )
            .map_err(|err| format!("failed to build input stream: {err}")),
        SampleFormat::U16 => device
            .build_input_stream(
                config,
                move |data: &[u16], _| {
                    let converted = data
                        .iter()
                        .map(|sample| (*sample as f32 / u16::MAX as f32) * 2.0 - 1.0)
                        .collect::<Vec<_>>();
                    write_f32_samples(&converted, &writer);
                },
                err_fn,
                None,
            )
            .map_err(|err| format!("failed to build input stream: {err}")),
        other => Err(format!("unsupported input sample format: {other:?}")),
    }
}

fn write_f32_samples(samples: &[f32], writer: &SharedWavWriter) {
    if let Ok(mut guard) = writer.lock() {
        if let Some(writer) = guard.as_mut() {
            for sample in samples {
                let _ = writer.write_sample(sample.clamp(-1.0, 1.0));
            }
        }
    }
}

fn convert_audio(input: &Path, output: &Path) -> Result<(), String> {
    let format = output
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("m4a")
        .to_ascii_lowercase();
    let mut command = Command::new("afconvert");
    match format.as_str() {
        "m4a" => {
            command.args(["-f", "m4af", "-d", "aac"]);
        }
        "aiff" => {
            command.args(["-f", "AIFF", "-d", "BEI16"]);
        }
        _ => return Err(format!("unsupported output format: {format}")),
    }
    command.arg(input).arg(output);
    let output_data = command
        .output()
        .map_err(|err| format!("failed to run afconvert: {err}"))?;
    if output_data.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output_data.stderr);
    Err(format!("afconvert failed: {}", stderr.trim()))
}

fn temp_wav_path() -> PathBuf {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    env::temp_dir().join(format!("taodio-{seconds}.wav"))
}

fn status() -> Result<(), String> {
    println!("idle");
    Ok(())
}

fn ensure_config() -> Result<Config, String> {
    let path = config_path()?;
    if path.exists() {
        let data = fs::read_to_string(&path)
            .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
        let mut config = serde_json::from_str::<Config>(&data)
            .map_err(|err| format!("failed to parse {}: {err}", path.display()))?;
        normalize_config(&mut config);
        save_config(&config)?;
        return Ok(config);
    }

    let config = default_config();
    save_config(&config)?;
    Ok(config)
}

fn normalize_config(config: &mut Config) {
    if !LANGUAGES
        .iter()
        .any(|(code, _)| *code == config.settings.language)
    {
        config.settings.language = "zh".to_string();
    }
    if !FORMATS.contains(&config.settings.save_format.as_str()) {
        config.settings.save_format = "m4a".to_string();
    }
}

fn save_config(config: &Config) -> Result<(), String> {
    let path = config_path()?;
    let parent = path
        .parent()
        .ok_or_else(|| format!("invalid config path: {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    let data = serde_json::to_string_pretty(config)
        .map_err(|err| format!("failed to serialize config: {err}"))?;
    fs::write(&path, format!("{data}\n"))
        .map_err(|err| format!("failed to write {}: {err}", path.display()))
}

fn config_path() -> Result<PathBuf, String> {
    let home = home_dir().ok_or_else(|| "HOME is not set".to_string())?;
    Ok(home.join(CONFIG_DIR).join(CONFIG_FILE))
}

fn next_value(
    iter: &mut std::iter::Peekable<impl Iterator<Item = OsString>>,
    flag: &str,
) -> Result<OsString, String> {
    iter.next()
        .ok_or_else(|| format!("missing value after {flag}"))
}

fn default_filename(format: &str) -> PathBuf {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    PathBuf::from(format!("taodio-{seconds}.{format}"))
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}

fn expand_home(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    if text == "~" {
        return home_dir().unwrap_or(path);
    }
    if let Some(rest) = text.strip_prefix("~/") {
        return home_dir().map(|home| home.join(rest)).unwrap_or(path);
    }
    path
}

fn print_help() {
    println!(
        r#"taodio

Usage:
  taodio
      Open the main menu.

  taodio record [--output FILE | --dir DIR --name NAME] [--format m4a|aiff] [--force]
      Start recording now, then stop and save when Enter is pressed.

  taodio record --device "MacBook Pro Microphone"
      Record with a specific input device.

  taodio config
      Print the config path.

  taodio devices
      List available input devices.

Examples:
  taodio
  taodio devices
  taodio record --name lecture-01
  taodio record --dir ~/Desktop --name today --device "MacBook Air麦克风"
"#
    );
}
