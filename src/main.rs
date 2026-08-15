mod app;
mod core;
mod ui;

use clap::Parser;
use color_eyre::Result;

use crate::core::SortMode;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Tree with native Ratatui theme editor and formatting"
)]
pub struct Cli {
    #[arg(short, long)]
    depth: Option<usize>,
    #[arg(long)]
    no_clipboard: bool,
    #[arg(long)]
    no_ignore: bool,
    #[arg(long, short, value_enum, default_value_t = SortMode::Name)]
    pub sort_mode: SortMode,
    #[arg(short, long)]
    pub max_entries: Option<usize>,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    let mut terminal = ratatui::init();

    let mut app = app::App::new(cli);
    let app_result = app.run(&mut terminal);

    ratatui::restore();

    app_result
}
