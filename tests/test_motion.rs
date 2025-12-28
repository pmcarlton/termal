mod utils;

use termal_msa::ui::key_handling;

#[test]
fn capG_moves_to_bottom() {
    utils::with_rig("tests/data/test-motion.msa", 80, 50, |ui, terminal| {
        assert_eq!(0, ui.top_line());
        let key_G = utils::keypress('G');
        key_handling::handle_key_press(ui, key_G);
        assert_eq!(ui.max_top_line(), ui.top_line());
    });
}
