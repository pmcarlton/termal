// SPDX-License-Identifier: MIT 
// Copyright (c) 2025 Thomas Junier 

mod common;

use crate::common::utils;

use termal_msa::ui::key_handling;

#[test]
fn capG_moves_to_bottom() {
    utils::with_rig("tests/data/test-motion.msa", 80, 50, |ui, terminal| {
        assert_eq!(0, ui.top_line());
        let key_G = utils::keypress('G');
        key_handling::handle_key_press(ui, key_G);
        assert_eq!(ui.max_top_line(), ui.top_line());
        // Idempotence at bottom
        key_handling::handle_key_press(ui, key_G);
        assert_eq!(ui.max_top_line(), ui.top_line());
    });
}

#[test]
fn g_moves_to_top() {
    utils::with_rig("tests/data/test-motion.msa", 80, 50, |ui, terminal| {
        let key_G = utils::keypress('G');
        key_handling::handle_key_press(ui, key_G);
        assert_eq!(ui.max_top_line(), ui.top_line());
        let key_g = utils::keypress('g');
        key_handling::handle_key_press(ui, key_g);
        assert_eq!(0, ui.top_line());
        // Idempotence at top
        key_handling::handle_key_press(ui, key_g);
        assert_eq!(0, ui.top_line());
    });
}
