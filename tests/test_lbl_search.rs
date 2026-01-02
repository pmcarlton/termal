mod common;

use crossterm::event::KeyCode;

use crate::common::utils;

use termal_msa::ui::{
    key_handling,
    render,
};

const screen_width: u16 = 80;
const screen_height: u16 = 50;

#[test]
fn test_label_search() {
    utils::with_rig("tests/data/test-motion.msa",
        screen_width, screen_height,
        |mut ui, terminal| {
        let key_double_quote = utils::keypress('"');
        let last_line_y = screen_height - 1;

        // Pressing " should cause "Label search:" to appear on last line

        key_handling::handle_key_press(ui, key_double_quote);
        // Don't forget to draw the UI after the key event...
        terminal.draw(|f| render::render_ui(f, &mut ui)).expect("update");
        let buffer = terminal.backend().buffer();
        let last_line = utils::screen_line(&buffer, last_line_y);

        assert!(
            last_line.contains("Label search:"),
            "\"Label search\" not found on last line: {}",
            last_line
        );

        // Pressing K, F, and J should add 'KFJ' to the modeline argument 
        //
        key_handling::handle_key_press(ui, utils::keypress('K'));
        key_handling::handle_key_press(ui, utils::keypress('F'));
        key_handling::handle_key_press(ui, utils::keypress('J'));
        terminal.draw(|f| render::render_ui(f, &mut ui)).expect("update");
        let buffer = terminal.backend().buffer();
        let last_line = utils::screen_line(&buffer, last_line_y);

        assert!(
            last_line.contains("Label search: KFJ"),
            "\"Label search: KFJ\" not found on last line: {}",
            last_line
        );

        // Pressing Enter should cause (1) a jump to the 1st matching seq (219) and (2) the text
        // "match #1/8" to appear in the modeline. The 1st match happens to be 14 lines from screen
        // bottom.

        let first_match_line_y = screen_height - 14;
        key_handling::handle_key_press(ui, KeyCode::Enter.into());
        terminal.draw(|f| render::render_ui(f, &mut ui)).expect("update");
        let buffer = terminal.backend().buffer();
        let first_match_line = utils::screen_line(&buffer, first_match_line_y);

        assert!(
            first_match_line.contains("219│KFJ"), // might as well check line #
            "\"KFJ\" not found on l. {}: {}", first_match_line_y,
            first_match_line
        );

        let last_line = utils::screen_line(&buffer, last_line_y);

        assert!(
            last_line.contains("match #1/8"),
            "\"match #1/8\" not found on last line: {}", 
            last_line
        );

        // Pressing 'n' should cause the modeline to change to "match #2/8"

        key_handling::handle_key_press(ui, utils::keypress('n'));
        terminal.draw(|f| render::render_ui(f, &mut ui)).expect("update");
        let buffer = terminal.backend().buffer();
        let last_line = utils::screen_line(&buffer, last_line_y);

        assert!(
            last_line.contains("match #2/8"),
            "\"match #2/8\" not found on last line: {}", 
            last_line
        );

        // Pressing 'n' another 7 times should cause the modeline to cycle back to "match #1/8"

        key_handling::handle_key_press(ui, utils::keypress('n'));
        key_handling::handle_key_press(ui, utils::keypress('n'));
        key_handling::handle_key_press(ui, utils::keypress('n'));
        key_handling::handle_key_press(ui, utils::keypress('n'));
        key_handling::handle_key_press(ui, utils::keypress('n'));
        key_handling::handle_key_press(ui, utils::keypress('n'));
        key_handling::handle_key_press(ui, utils::keypress('n'));
        terminal.draw(|f| render::render_ui(f, &mut ui)).expect("update");
        let buffer = terminal.backend().buffer();
        let last_line = utils::screen_line(&buffer, last_line_y);

        assert!(
            last_line.contains("match #1/8"),
            "\"match #1/8\" not found on last line: {}", 
            last_line
        );

    });
}
