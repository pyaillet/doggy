use std::{path::PathBuf, rc::Rc};

use chrono::{TimeZone, Utc};
use color_eyre::Result;

use directories::ProjectDirs;
use lazy_static::lazy_static;

#[cfg(feature = "otel")]
use opentelemetry::global;

use tracing::error;
use tracing_error::ErrorLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

use crate::components::Component;

use ratatui::{
    prelude::*,
    widgets::{
        block::Title, Block, Borders, Cell, Clear, LineGauge, Padding, Paragraph, Row, Table, Wrap,
    },
};

pub static GIT_COMMIT_HASH: &str = env!("DOGGY_GIT_INFO");

lazy_static! {
    pub static ref PROJECT_NAME: String = env!("CARGO_CRATE_NAME").to_uppercase().to_string();
    pub static ref DATA_FOLDER: Option<PathBuf> =
        std::env::var(format!("{}_DATA", PROJECT_NAME.clone()))
            .ok()
            .map(PathBuf::from);
    pub static ref CONFIG_FOLDER: Option<PathBuf> =
        std::env::var(format!("{}_CONFIG", PROJECT_NAME.clone()))
            .ok()
            .map(PathBuf::from);
    pub static ref LOG_ENV: String = format!("{}_LOGLEVEL", PROJECT_NAME.clone());
    pub static ref LOG_FILE: String = format!("{}.log", env!("CARGO_PKG_NAME"));
}

const GENERAL_BINDINGS: [(&str, &str); 5] = [
    ("q", "Quit"),
    (":", "Change resource"),
    ("/", "Filter resources"),
    ("?", "Help"),
    ("ESC", "Cancel/Previous screen"),
];

const NAVIGATION_BINDINGS: [(&str, &str); 4] = [
    ("j", "Down"),
    ("k", "Up"),
    ("PageUp", "Page up"),
    ("PageDown", "Page down"),
];

fn project_directory() -> Option<ProjectDirs> {
    ProjectDirs::from("org", "pyaillet", env!("CARGO_PKG_NAME"))
}

pub const NONE: &str = "<none>";

macro_rules! get_or_not_found {
    ($property:expr) => {
        $property
            .as_ref()
            .and_then(|s| Some(s.as_str()))
            .unwrap_or(crate::utils::NONE)
            .to_string()
    };
    ($property:expr, $extractor:expr) => {
        $property
            .as_ref()
            .and_then($extractor)
            .unwrap_or(crate::utils::NONE)
            .to_string()
    };
}

pub(crate) use get_or_not_found;

pub(crate) fn table<'a, const SIZE: usize>(
    title: String,
    headers: [&'a str; SIZE],
    items: Vec<Row<'a>>,
    constraints: &'static [Constraint; SIZE],
    style: Option<Style>,
) -> Table<'a> {
    let normal_style = style.unwrap_or_default();
    let selected_style = normal_style.reversed();
    let header_cells = headers
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().bold()));
    let header = ratatui::widgets::Row::new(header_cells)
        .style(normal_style)
        .height(1);
    Table::new(items, constraints)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(selected_style)
}

pub fn default_layout(size: Rect) -> Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Max(3), Constraint::Min(5), Constraint::Max(1)])
        .split(size)
}

pub fn centered_rect(size_x: u16, size_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Max((r.height - size_y) / 2),
            Constraint::Min(size_y),
            Constraint::Max((r.height - size_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Max((r.width - size_x) / 2),
            Constraint::Min(size_x),
            Constraint::Max((r.width - size_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn toast<'a, T>(f: &mut Frame<'_>, title: T, msg: &str, timeout: usize, ttl: usize)
where
    T: Into<Title<'a>>,
{
    let width = 60;

    let lg = LineGauge::default()
        .block(Block::default().borders(Borders::NONE))
        .label("")
        .gauge_style(
            Style::default()
                .fg(Color::White)
                .bg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .line_set(symbols::line::THICK)
        .ratio(((timeout - ttl) as f64) / (timeout as f64));

    let text = vec![
        Line::from(msg),
        Line::from(""),
        Line::from(vec![
            Span::from("Press "),
            Span::styled("ESC", Style::new().bold()),
            Span::from(" to cancel"),
        ]),
    ];
    let paragraph = Paragraph::new(text)
        .wrap(Wrap { trim: false })
        .alignment(Alignment::Center);
    let line_count: u16 = paragraph
        .line_count(width - 4)
        .try_into()
        .expect("Too much lines");

    let block = Block::default()
        .title(title)
        .padding(Padding::new(1, 1, 1, 1))
        .borders(Borders::ALL);
    let area = centered_rect(width, line_count + 4, f.size());
    let pg_area = Rect::new(
        area.x,
        area.y + area.height - 2,
        area.width - 1,
        area.height,
    );

    f.render_widget(Clear, area); //this clears out the background
    f.render_widget(paragraph.block(block), area);
    f.render_widget(lg, pg_area);
}

pub fn help_screen(f: &mut Frame<'_>, component: &Component) {
    let area = default_layout(f.size())[1];

    let block = Block::default()
        .title("Help")
        .padding(Padding::new(1, 1, 1, 1))
        .borders(Borders::ALL);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(33),
        ])
        .split(area);

    let column_block = Block::default()
        .padding(Padding::new(1, 1, 1, 1))
        .borders(Borders::NONE);

    f.render_widget(Clear, area); //this clears out the background
    f.render_widget(block, area);

    if let Some(bindings) = component.get_bindings() {
        let resource = binding_to_help(bindings, component.get_name());
        f.render_widget(resource.block(column_block.clone()), columns[0]);
    }

    let general = binding_to_help(&GENERAL_BINDINGS, "General");
    f.render_widget(general.block(column_block.clone()), columns[1]);

    let navigation = binding_to_help(&NAVIGATION_BINDINGS, "Navigation");
    f.render_widget(navigation.block(column_block), columns[2]);
}

fn binding_to_help<'a, 'b, T>(bindings: T, title: &'static str) -> Paragraph<'a>
where
    T: IntoIterator<Item = &'b (&'b str, &'b str)>,
    'b: 'a,
{
    let title = vec![Line::from(title.bold()), Line::from("")];

    let texts: Vec<Line<'a>> = title
        .into_iter()
        .chain(
            bindings
                .into_iter()
                .map(|(k, a)| Line::from(format!("{: <10} : {}", format!("<{}>", k), a))),
        )
        .collect();
    Paragraph::new(texts)
}

pub fn initialize_panic_handler() -> Result<()> {
    let (panic_hook, eyre_hook) = color_eyre::config::HookBuilder::default()
        .panic_section(format!(
            "This is a bug. Consider reporting it at {}",
            env!("CARGO_PKG_REPOSITORY")
        ))
        .capture_span_trace_by_default(false)
        .display_location_section(false)
        .display_env_section(false)
        .into_hooks();
    eyre_hook.install()?;
    std::panic::set_hook(Box::new(move |panic_info| {
        if let Ok(mut t) = crate::tui::Tui::new() {
            if let Err(r) = t.exit() {
                error!("Unable to exit Terminal: {:?}", r);
            }
        }

        #[cfg(not(debug_assertions))]
        {
            use human_panic::{handle_dump, print_msg, Metadata};
            let meta = Metadata {
                version: env!("CARGO_PKG_VERSION").into(),
                name: env!("CARGO_PKG_NAME").into(),
                authors: env!("CARGO_PKG_AUTHORS").replace(':', ", ").into(),
                homepage: env!("CARGO_PKG_HOMEPAGE").into(),
            };

            let file_path = handle_dump(&meta, panic_info);
            // prints human-panic message
            print_msg(file_path, &meta)
                .expect("human-panic: printing error message to console failed");
            eprintln!("{}", panic_hook.panic_report(panic_info)); // prints color-eyre stack trace to stderr
        }
        let msg = format!("{}", panic_hook.panic_report(panic_info));
        log::error!("Error: {}", strip_ansi_escapes::strip_str(msg));

        #[cfg(debug_assertions)]
        {
            // Better Panic stacktrace that is only enabled when debugging.
            better_panic::Settings::auto()
                .most_recent_first(false)
                .lineno_suffix(true)
                .verbosity(better_panic::Verbosity::Full)
                .create_panic_handler()(panic_info);
        }

        std::process::exit(libc::EXIT_FAILURE);
    }));
    Ok(())
}

pub fn get_data_dir() -> PathBuf {
    let directory = if let Some(s) = DATA_FOLDER.clone() {
        s
    } else if let Some(proj_dirs) = project_directory() {
        proj_dirs.data_local_dir().to_path_buf()
    } else {
        PathBuf::from(".").join(".data")
    };
    directory
}

pub fn initialize_logging() -> Result<()> {
    let directory = get_data_dir();
    std::fs::create_dir_all(directory.clone())?;

    let log_path = directory.join(LOG_FILE.clone());
    let log_file = std::fs::File::create(log_path)?;

    std::env::set_var(
        "RUST_LOG",
        std::env::var("RUST_LOG")
            .or_else(|_| std::env::var(LOG_ENV.clone()))
            .unwrap_or_else(|_| format!("{}=info", env!("CARGO_CRATE_NAME"))),
    );

    // The SubscriberExt and SubscriberInitExt traits are needed to extend the
    // Registry to accept `opentelemetry (the OpenTelemetryLayer type).
    let file_subscriber = tracing_subscriber::fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_writer(log_file)
        .with_target(false)
        .with_ansi(false)
        .with_filter(tracing_subscriber::filter::EnvFilter::from_default_env());
    let tracing_registry = tracing_subscriber::registry()
        .with(file_subscriber)
        .with(ErrorLayer::default());

    #[cfg(feature = "otel")]
    let tracing_registry = {
        // Allows you to pass along context (i.e., trace IDs) across services
        global::set_text_map_propagator(opentelemetry_jaeger::Propagator::new());
        // Sets up the machinery needed to export data to Jaeger
        // There are other OTel crates that provide pipelines for the vendors
        // mentioned earlier.
        let tracer = opentelemetry_jaeger::new_pipeline()
            .with_service_name("doggy")
            .install_simple()?;

        // Create a tracing layer with the configured tracer
        let opentelemetry = tracing_opentelemetry::layer().with_tracer(tracer);
        tracing_registry.with(opentelemetry)
    };

    tracing_registry.init();
    Ok(())
}

pub trait Age {
    fn age(&self) -> String;
}

impl Age for i64 {
    fn age(&self) -> String {
        let now = Utc::now();
        let created = Utc
            .timestamp_opt(*self, 0)
            .single()
            .expect("Unable to convert to timestamp");
        let delta = now - created;
        match delta {
            _ if delta.num_seconds() < 60 => format!("{}s", delta.num_seconds()),
            _ if delta.num_minutes() < 60 => format!("{}m", delta.num_minutes()),
            _ if delta.num_hours() < 24 => format!("{}h", delta.num_hours()),
            _ => format!("{}d", delta.num_days()),
        }
    }
}
