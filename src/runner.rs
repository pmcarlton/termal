// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Thomas Junier
// Modifications (c) 2026 Peter Carlton

use std::{
    fmt,
    fs::File,
    io::{stdout, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

use log::info;

use crate::alignment::Alignment;
use crate::app::{App, TermalConfig};
use crate::seq::clustal::read_clustal_file;
use crate::seq::fasta::read_fasta_file;
use crate::seq::stockholm::read_stockholm_file;
use crate::tree::{parse_newick, tree_lines_and_order, TreeNode};
use crate::ui::{key_handling::handle_key_press, render::render_ui, UI};

use clap::{Parser, ValueEnum};

use crossterm::{
    event::{self, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};

use ratatui::{
    prelude::{CrosstermBackend, Rect, Terminal},
    TerminalOptions, Viewport,
};

use crate::errors::TermalError;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None) ]
struct Cli {
    /// Alignment file
    aln_fname: Option<String>,

    /// Show key bindings and exit successfully
    #[arg(short = 'b', long = "show-bindings")]
    show_bindings: bool,

    /// Info mode (no TUI)
    #[arg(short, long)]
    info: bool,

    /// Sequence file format
    #[arg(short, long = "format", default_value_t = SeqFileFormat::FastA,
        help = "Sequence file format [fasta|clustal|stockholm] (or just f|c|s); default: fasta",
        hide_default_value = true,
        hide_possible_values = true,
    )]
    format: SeqFileFormat,

    /// Gecos color map
    #[arg(short, long = "color-map")]
    color_map: Option<String>,

    /// Fixed terminal width (mostly used for testing/debugging)
    #[arg(short, long, requires = "height")]
    width: Option<u16>,

    /// Fixed terminal height ("tall" -- -h is already used)
    #[arg(short = 't', long, requires = "width")]
    height: Option<u16>,

    /// Start with labels pane hidden
    #[arg(short = 'L', long)]
    hide_labels_pane: bool,

    /// Start with bottom pane hidden
    #[arg(short = 'B', long)]
    hide_bottom_pane: bool,

    /// (Currently no effect)
    #[arg(short = 'D', long)]
    debug: bool,

    /// User-supplied order (filename)
    #[arg(short = 'o', long)]
    user_order: Option<String>,

    // TODO: superseded by BW colormap
    /// Disable color
    #[arg(short = 'C', long = "no-color")]
    no_color: bool,

    /// Disable scrollbars (mostly for testing)
    #[arg(long = "no-scrollbars")]
    no_scrollbars: bool,

    /// Poll wait time [ms]
    #[clap(long = "poll-wait-time", default_value_t = 50)]
    poll_wait_time: u64,

    /// Panic (for testing)
    #[clap(long = "panic")]
    panic: bool,

    // TODO: the ZB can be disabled at runtime (or at least it should)
    /// Do not show zoom box (zooming itself is not disabled)
    #[arg(long = "no-zoom-box")]
    no_zoombox: bool,

    // TODO: this is only ever used when the bottom pane is at the bottom of the terminal, which is
    // practically never.
    //
    /// Do not show zoom box guides (only useful if zoom box not shown)
    #[arg(long = "no-zb-guides")]
    no_zb_guides: bool,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum SeqFileFormat {
    #[clap(name = "fasta")]
    #[clap(alias = "f")]
    FastA,
    #[clap(name = "clustal")]
    #[clap(alias = "c")]
    Clustal,
    #[clap(name = "stockholm")]
    #[clap(alias = "s")]
    Stockholm,
}

impl fmt::Display for SeqFileFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            SeqFileFormat::FastA => "fasta",
            SeqFileFormat::Clustal => "clustal",
            SeqFileFormat::Stockholm => "stockholm",
        };
        write!(f, "{}", s)
    }
}

// pub fn read_fasta_file<P: AsRef<Path>>(path: P) -> Result<SeqFile, std::io::Error> {
fn read_user_ordering(fname: &str) -> Result<Vec<String>, std::io::Error> {
    let uord_file = File::open(fname)?;
    let reader = BufReader::new(uord_file);
    reader.lines().collect()
}

fn find_termal_config() -> Option<PathBuf> {
    if let Ok(home) = std::env::var("HOME") {
        let path = PathBuf::from(home).join(".termalconfig");
        if path.exists() {
            return Some(path);
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        let path = cwd.join(".termalconfig");
        if path.exists() {
            return Some(path);
        }
    }
    None
}

fn needs_alignment(seq_file: &crate::seq::file::SeqFile) -> bool {
    let mut iter = seq_file.iter();
    let Some(first) = iter.next() else {
        return false;
    };
    let first_len = first.sequence.len();
    iter.any(|rec| rec.sequence.len() != first_len)
}

struct AutoAlignResult {
    seq_file: crate::seq::file::SeqFile,
    tree: Option<TreeNode>,
    tree_newick: Option<String>,
    tree_lines: Vec<String>,
    tree_panel_width: u16,
    tree_error: Option<String>,
}

fn align_fasta_with_mafft(
    input_path: &Path,
    mafft_bin_dir: Option<&Path>,
) -> Result<AutoAlignResult, TermalError> {
    let mafft_bin_dir = mafft_bin_dir.ok_or_else(|| {
        TermalError::Format(String::from(
            "Unaligned FASTA requires mafft. Install mafft and set mafft_bin_dir in .termalconfig.",
        ))
    })?;
    let mut input_tmp = std::env::temp_dir();
    let unique_in = format!("termal-mafft-auto-{}.in.fa", std::process::id());
    input_tmp.push(unique_in);
    std::fs::copy(input_path, &input_tmp)?;

    let mut output_path = std::env::temp_dir();
    let unique_out = format!("termal-mafft-auto-{}.out.fa", std::process::id());
    output_path.push(unique_out);

    println!("Unaligned FASTA detected; running mafft --maxiterate 1000 --localpair...");
    stdout().flush().ok();

    let tool_path = mafft_bin_dir.join("mafft");
    let output_file = File::create(&output_path)?;
    let status = Command::new(tool_path)
        .arg("--maxiterate")
        .arg("1000")
        .arg("--localpair")
        .arg("--treeout")
        .arg("--reorder")
        .arg(&input_tmp)
        .stdout(Stdio::from(output_file))
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| TermalError::Format(format!("Failed to run mafft: {}", e)))?;
    if !status.success() {
        return Err(TermalError::Format(String::from("mafft failed")));
    }
    let aligned = read_fasta_file(&output_path)?;

    let mut tree_error = None;
    let mut tree = None;
    let mut tree_newick = None;
    let mut tree_lines = Vec::new();
    let mut tree_panel_width = 0;
    let tree_path = PathBuf::from(format!("{}.tree", input_tmp.display()));
    match std::fs::read_to_string(&tree_path) {
        Ok(tree_text) => match parse_newick(&tree_text) {
            Ok(parsed) => {
                if let Ok((lines, _order)) = tree_lines_and_order(&parsed) {
                    tree_panel_width = lines
                        .iter()
                        .map(|line| line.chars().count())
                        .max()
                        .unwrap_or(0)
                        .min(u16::MAX as usize) as u16;
                    tree_lines = lines;
                }
                tree = Some(parsed);
                tree_newick = Some(tree_text);
            }
            Err(e) => {
                tree_error = Some(format!("Failed to parse mafft tree: {}", e));
            }
        },
        Err(e) => {
            tree_error = Some(format!("Failed to read mafft tree: {}", e));
        }
    }

    std::fs::remove_file(&input_tmp).ok();
    std::fs::remove_file(&output_path).ok();
    std::fs::remove_file(&tree_path).ok();
    Ok(AutoAlignResult {
        seq_file: aligned,
        tree,
        tree_newick,
        tree_lines,
        tree_panel_width,
        tree_error,
    })
}

pub fn run() -> Result<(), TermalError> {
    env_logger::init();
    info!("Starting log");

    let cli = Cli::parse();
    if cli.panic {
        panic!("User-requested panic");
    }

    if cli.show_bindings {
        println!("{}", crate::ui::USER_GUIDE);
        return Ok(());
    }

    if let Some(seq_filename) = &cli.aln_fname {
        let mut config_err: Option<String> = None;
        let mut config: Option<TermalConfig> = None;
        if let Some(path) = find_termal_config() {
            match TermalConfig::from_file(&path) {
                Ok(cfg) => config = Some(cfg),
                Err(e) => {
                    config_err = Some(format!("Error reading {}: {}", path.display(), e));
                }
            }
        }
        let mut auto_tree: Option<(TreeNode, String, Vec<String>, u16)> = None;
        let mut auto_tree_err: Option<String> = None;
        let mut app = if Path::new(seq_filename).extension().and_then(|s| s.to_str())
            == Some("trml")
        {
            App::from_session_file(Path::new(seq_filename))?
        } else {
            let seq_file = match cli.format {
                SeqFileFormat::FastA => {
                    let seq_file = read_fasta_file(seq_filename)?;
                    if needs_alignment(&seq_file) {
                        let aligned = align_fasta_with_mafft(
                            Path::new(seq_filename),
                            config
                                .as_ref()
                                .and_then(|cfg| cfg.tools.mafft_bin_dir.as_deref()),
                        )?;
                        if let Some(tree) = aligned.tree {
                            if let Some(tree_text) = aligned.tree_newick {
                                auto_tree = Some((
                                    tree,
                                    tree_text,
                                    aligned.tree_lines,
                                    aligned.tree_panel_width,
                                ));
                            }
                        }
                        auto_tree_err = aligned.tree_error;
                        aligned.seq_file
                    } else {
                        seq_file
                    }
                }
                SeqFileFormat::Clustal => read_clustal_file(seq_filename)?,
                SeqFileFormat::Stockholm => read_stockholm_file(seq_filename)?,
            };
            let alignment = Alignment::from_file(seq_file);
            let mut ordering_err_msg: Option<String> = None;
            let mut user_ordering = match cli.user_order {
                Some(fname) => {
                    // TODO: should be called from_path()
                    let get_ord_vec = read_user_ordering(&fname);
                    match get_ord_vec {
                        Ok(ord_vec) => Some(ord_vec),
                        Err(_) => {
                            ordering_err_msg =
                                Some(format!("Error reading ordering file {}", fname));
                            None // => App ignores bad user ordering
                        }
                    }
                }
                None => None,
            };
            // Check for discrepancies beween the user-specied ordering and alignment headers. The two
            // sets should be identical.
            if let Some(ref ord_vec) = user_ordering {
                let mut uo_clone = ord_vec.clone();
                let mut ah_clone = alignment.headers.clone();
                uo_clone.sort();
                ah_clone.sort();
                if uo_clone != ah_clone {
                    ordering_err_msg = Some(String::from("Discrepancies in ordering vs alignment"));
                    // App must ignore bad user ordering
                    user_ordering = None;
                }
            };
            let mut app = App::new(seq_filename, alignment, user_ordering);
            if let Some(msg) = ordering_err_msg {
                app.error_msg(msg);
            }
            app
        };

        if let Some((tree, tree_newick, tree_lines, tree_panel_width)) = auto_tree.take() {
            app.set_tree_for_current_view(tree, tree_newick, tree_lines, tree_panel_width);
        }
        if let Some(msg) = auto_tree_err.take() {
            app.error_msg(msg);
        }
        if let Some(msg) = config_err.take() {
            app.error_msg(msg);
        }
        if let Some(config) = config.take() {
            app.set_search_color_config(config.search_colors);
            app.set_emboss_bin_dir(config.tools.emboss_bin_dir);
            app.set_mafft_bin_dir(config.tools.mafft_bin_dir);
        }
        app.refresh_saved_searches_public();
        app.recompute_current_seq_search();

        if cli.info {
            info!("Running in debug mode.");
            app.output_info(); // TODO: can't this be done using info_msg()?
            return Ok(());
        }

        stdout().execute(EnterAlternateScreen)?;
        enable_raw_mode()?;

        let backend = CrosstermBackend::new(stdout());
        let viewport: Viewport;
        // Fix viewport dimensions IFF supplied (mainly for tests)
        //
        if let Some(width) = cli.width {
            // height must be defined too (see 'requires' in struct Cli above)
            let height = cli.height.unwrap();
            viewport = Viewport::Fixed(Rect::new(0, 0, width, height));
        } else {
            viewport = Viewport::Fullscreen;
        }
        let mut terminal = Terminal::with_options(backend, TerminalOptions { viewport })?;
        terminal.clear()?;

        let mut app_ui = UI::new(&mut app);
        if cli.no_scrollbars {
            app_ui.disable_scrollbars();
        }
        if cli.no_color {
            app_ui.set_monochrome();
        }
        if cli.no_zoombox {
            app_ui.set_zoombox(false);
        }
        if cli.no_zb_guides {
            app_ui.set_zoombox_guides(false);
        }
        if cli.hide_labels_pane {
            app_ui.set_left_pane_width(0);
        }
        if cli.hide_bottom_pane {
            app_ui.set_bottom_pane_height(0);
        }
        if let Some(path) = cli.color_map {
            app_ui.add_user_colormap(&path);
            app_ui.prev_colormap();
        }

        let poll_wait = Duration::from_millis(cli.poll_wait_time);
        terminal.draw(|f| render_ui(f, &mut app_ui))?;

        // main loop
        loop {
            // Wait for an event (or timeout)
            if event::poll(poll_wait)? {
                match event::read()? {
                    event::Event::Key(key) if key.kind == KeyEventKind::Press => {
                        app_ui.clear_dirty();
                        let done = handle_key_press(&mut app_ui, key);
                        if done {
                            break;
                        }
                        if app_ui.take_dirty() {
                            terminal.draw(|f| render_ui(f, &mut app_ui))?;
                        }
                    }
                    event::Event::Resize(_, _) => {
                        terminal.draw(|f| render_ui(f, &mut app_ui))?;
                    }
                    _ => {}
                }
            }
        }

        stdout().execute(LeaveAlternateScreen)?;
        disable_raw_mode()?;

        if let Some(msg) = app_ui.take_exit_message() {
            println!("{}", msg);
        }

        Ok(())
    } else {
        panic!("Expected filename argument");
    }
}
