use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, KeyEventKind};

use ratatui::{
    backend::{Backend, TestBackend},
    buffer::Buffer,
    prelude::{Rect, Terminal},
    TerminalOptions,
    Viewport,
};

use termal_msa::{
    alignment::Alignment,
    app::App,
    ui::{
        key_handling, 
        render,
        render::render_ui,
        UI,
    },
    seq::fasta,
};

pub fn render(app: &mut App, w: u16, h: u16) -> Buffer {
    let backend = TestBackend::new(w, h);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut ui = UI::new(app);
    terminal.draw(|f| render_ui(f, &mut ui)).expect("draw");
    terminal.backend().buffer().clone()
}

pub fn buffer_text(buf: &Buffer) -> String {
    let area = buf.area;
    let mut out = String::new();
    for y in 0..area.height {
        for x in 0..area.width {
            out.push(buf.get(x, y)
                .symbol()
                .chars()
                .next()
                .unwrap_or(' '));
            }
        out.push('\n');
    }
    out
}

pub fn keypress(c: char) -> KeyEvent {
    KeyEvent {
        code: KeyCode::Char(c),
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: crossterm::event::KeyEventState::NONE,
    }
}

pub fn with_rig<F>(
    path: &str,
    term_width: u16,
    term_height: u16,
    mut f: F) 
    where
        F: FnMut(&mut UI, &mut Terminal<TestBackend>),
{
    let seq_file = fasta::read_fasta_file("tests/data/test-motion.msa").expect("read");
    let aln = Alignment::from_file(seq_file);
    let mut app = App::new("TEST", aln, None);
    let mut ui = UI::new(&mut app);

    let backend = TestBackend::new(term_width, term_height);
    let viewport = Viewport::Fixed(Rect::new(0, 0, term_width, term_height));
    let mut terminal = Terminal::with_options(backend, TerminalOptions { viewport })
        .expect("creating test-backend terminal");
    // Initial draw
    terminal.draw(|f| render::render_ui(f, &mut ui)).expect("initial draw");

    // Events and assertions here
    f(&mut ui, &mut terminal);
}
