use super::{SearchColorConfig, ToolsConfig};
use crate::{
    alignment::Alignment,
    app::{order, App, SearchKind, SeqMatch, SeqOrdering},
    tree::{parse_newick, tree_lines_and_order},
};
use serde_json::json;
use std::path::PathBuf;

#[test]
fn test_order_00() {
    assert_eq!(vec![2, 1, 0], order(&[20.0, 15.0, 10.0]));
}

#[test]
fn test_order_05() {
    assert_eq!(
        vec![3, 2, 0, 1, 4],
        order(&[12.23, 34.89, 7.0, -23.2, 100.0]),
    );
}

#[test]
fn test_order_10() {
    // Reverse order
    let orig = vec![3.0, 2.0, 5.0, 1.0, 4.0];
    let direct_order = order(&orig);
    assert_eq!(vec![3, 1, 0, 4, 2], direct_order);
    let reverse_order = order(&direct_order);
    assert_eq!(vec![2, 1, 4, 0, 3], reverse_order);
}

#[test]
fn test_ordering_00() {
    let hdrs = vec![
        String::from("R1"),
        String::from("R2"),
        String::from("R3"),
        String::from("R4"),
    ];
    let seqs = vec![
        String::from("catgcatatg"), // 0 diffs WRT consensus
        String::from("cCtgcatatg"), // 1 diffs WRT consensus
        String::from("catAcTtatg"), // 2 diffs WRT consensus
        String::from("caGgAataAg"), // 3 diffs WRT consensus
    ];
    let aln = Alignment::from_vecs(hdrs, seqs);
    let mut app = App::new("TEST", aln, None);
    assert_eq!(app.ordering, vec![0, 1, 2, 3]);
    app.next_ordering_criterion();
    // Ordering is now by increasing metric, which is (by default) %id WRT consensus. Given the above
    // sequences, this effectively reverses the order.
    assert_eq!(app.ordering, vec![3, 2, 1, 0]);
    app.next_ordering_criterion();
    // Now by decreasing metric, which in this case is (by construction) the same as the
    // original.
    assert_eq!(app.ordering, vec![0, 1, 2, 3]);
}

#[test]
fn test_ordering_05() {
    let hdrs = vec![
        String::from("R1"),
        String::from("R2"),
        String::from("R3"),
        String::from("R4"),
        String::from("R5"),
    ];
    let seqs = vec![
        String::from("catgcatatg"), // 0 diffs WRT consensus
        String::from("caGgAaCaAg"), // 4 diffs WRT consensus
        String::from("catAcTtatg"), // 2 diffs WRT consensus
        String::from("cCtgcatatg"), // 1 diffs WRT consensus
        String::from("caGgAataAg"), // 3 diffs WRT consensus
    ];
    let aln = Alignment::from_vecs(hdrs, seqs);
    let mut app = App::new("TEST", aln, None);
    assert_eq!(app.ordering, vec![0, 1, 2, 3, 4]);
    app.next_ordering_criterion();
    assert_eq!(app.ordering, vec![1, 4, 2, 3, 0]);
    assert_eq!(app.reverse_ordering, vec![4, 0, 2, 3, 1]);
    app.next_ordering_criterion();
    assert_eq!(app.ordering, vec![0, 3, 2, 4, 1]);
    assert_eq!(app.reverse_ordering, vec![0, 4, 2, 1, 3]);
}

#[test]
fn test_msafara_config_from_value() {
    let value = json!({
        "palette": ["#010203"],
        "current_search": [4, 5, 6],
        "emboss_bin_dir": "/opt/emboss",
        "mafft_bin_dir": "/opt/mafft"
    });
    let colors = SearchColorConfig::from_value(&value);
    assert_eq!(colors.palette[0], (1, 2, 3));
    assert_eq!(colors.current_search, (4, 5, 6));

    let tools = ToolsConfig::from_value(&value);
    assert_eq!(tools.emboss_bin_dir, Some(PathBuf::from("/opt/emboss")));
    assert_eq!(tools.mafft_bin_dir, Some(PathBuf::from("/opt/mafft")));
}

#[test]
fn test_update_records_from_alignment() {
    let hdrs = vec![String::from("A"), String::from("B")];
    let seqs = vec![String::from("AA"), String::from("CC")];
    let aln = Alignment::from_vecs(hdrs, seqs);
    let mut app = App::new("TEST", aln, None);

    let updated = Alignment::from_vecs(
        vec![String::from("A"), String::from("B")],
        vec![String::from("TT"), String::from("GG")],
    );
    app.update_records_from_alignment(&updated, &[1]).unwrap();

    assert_eq!(app.records[0].sequence, "AA");
    assert_eq!(app.records[1].sequence, "GG");
}

#[test]
fn test_ordering_status_label() {
    let hdrs = vec![String::from("R1"), String::from("R2")];
    let seqs = vec![String::from("AA"), String::from("AA")];
    let aln = Alignment::from_vecs(hdrs, seqs);
    let mut app = App::new("TEST", aln, None);
    assert_eq!(app.ordering_status_label(), "o:original");
    app.next_ordering_criterion();
    assert_eq!(app.ordering_status_label(), "o:%id↑");
    app.next_ordering_criterion();
    assert_eq!(app.ordering_status_label(), "o:%id↓");
    app.next_ordering_criterion();
    assert_eq!(app.ordering_status_label(), "o:match");
    app.set_user_ordering(vec![String::from("R1"), String::from("R2")])
        .unwrap();
    assert_eq!(app.ordering_status_label(), "o:tree");
}

#[test]
fn test_create_view_from_selection() {
    let hdrs = vec![String::from("R1"), String::from("R2"), String::from("R3")];
    let seqs = vec![String::from("AA"), String::from("BB"), String::from("CC")];
    let aln = Alignment::from_vecs(hdrs, seqs);
    let mut app = App::new("TEST", aln, None);
    app.select_label_by_rank(1).unwrap();
    app.create_view_from_selection("picked").unwrap();

    let view = app.views.get("picked").expect("view");
    assert_eq!(view.sequence_ids, vec![1]);
    assert!(view.tree.is_none());
    assert_eq!(
        view.user_ordering.as_ref().unwrap(),
        &vec!["R2".to_string()]
    );
}

#[test]
fn test_tree_invalidated_on_view_change() {
    let hdrs = vec![String::from("R1"), String::from("R2")];
    let seqs = vec![String::from("AA"), String::from("BB")];
    let aln = Alignment::from_vecs(hdrs, seqs);
    let mut app = App::new("TEST", aln, None);
    app.create_view_from_current("custom").unwrap();
    app.switch_view("custom").unwrap();

    let tree = parse_newick("(R1,R2);").unwrap();
    let (lines, _order) = tree_lines_and_order(&tree).unwrap();
    app.tree = Some(tree.clone());
    app.tree_newick = Some(String::from("(R1,R2);"));
    app.tree_lines = lines.clone();
    app.tree_panel_width = app
        .tree_lines
        .iter()
        .map(|line| line.len() as u16)
        .max()
        .unwrap_or(0);
    if let Some(view) = app.views.get_mut("custom") {
        view.tree = Some(tree);
        view.tree_newick = Some(String::from("(R1,R2);"));
        view.tree_lines = lines;
        view.tree_panel_width = app.tree_panel_width;
    }

    app.remove_sequences(&[0]);
    assert!(app.tree.is_none());
    assert!(app.tree_lines.is_empty());
    let view = app.views.get("custom").expect("view");
    assert!(view.tree.is_none());
    assert!(view.tree_lines.is_empty());
}

#[test]
fn test_set_tree_for_current_view() {
    let hdrs = vec![String::from("R1"), String::from("R2")];
    let seqs = vec![String::from("AA"), String::from("BB")];
    let aln = Alignment::from_vecs(hdrs, seqs);
    let mut app = App::new("TEST", aln, None);
    let tree = parse_newick("(R1,R2);").unwrap();
    let (lines, _order) = tree_lines_and_order(&tree).unwrap();
    let width = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0)
        .min(u16::MAX as usize) as u16;
    app.set_tree_for_current_view(tree, String::from("(R1,R2);"), lines, width);
    assert!(app.tree().is_some());
    let view = app.views.get("original").expect("view");
    assert!(view.tree.is_some());
    assert!(view.tree_lines.len() == 2);
}

#[test]
fn test_set_tree_ordering_from_tree() {
    let hdrs = vec![String::from("R1"), String::from("R2")];
    let seqs = vec![String::from("AA"), String::from("BB")];
    let aln = Alignment::from_vecs(hdrs, seqs);
    let mut app = App::new("TEST", aln, None);
    let tree = parse_newick("(R2,R1);").unwrap();
    app.tree = Some(tree);
    app.set_tree_ordering_from_tree().unwrap();
    assert_eq!(app.get_seq_ordering(), SeqOrdering::User);
    assert_eq!(app.ordering, vec![1, 0]);
}

#[test]
fn test_view_alignment_override_applied() {
    let hdrs = vec![String::from("R1"), String::from("R2")];
    let seqs = vec![String::from("AA"), String::from("BB")];
    let aln = Alignment::from_vecs(hdrs, seqs);
    let mut app = App::new("TEST", aln, None);
    app.select_label_by_rank(1).unwrap();
    app.create_view_from_selection("picked").unwrap();
    if let Some(view) = app.views.get_mut("picked") {
        view.alignment_override = Some(vec![String::from("XX")]);
    }
    app.switch_view("picked").unwrap();
    assert_eq!(app.alignment.sequences, vec![String::from("XX")]);
}

#[test]
fn test_select_label_by_rank() {
    let hdrs = vec![String::from("R1"), String::from("R2"), String::from("R3")];
    let seqs = vec![String::from("AA"), String::from("BB"), String::from("CC")];
    let aln = Alignment::from_vecs(hdrs, seqs);
    let mut app = App::new("TEST", aln, None);
    app.select_label_by_rank(1).unwrap();
    assert_eq!(app.cursor_rank(), Some(1));
    assert!(app.is_label_selected(1));
}

#[test]
fn test_invert_selection() {
    let hdrs = vec![String::from("R1"), String::from("R2"), String::from("R3")];
    let seqs = vec![String::from("AA"), String::from("BB"), String::from("CC")];
    let aln = Alignment::from_vecs(hdrs, seqs);
    let mut app = App::new("TEST", aln, None);
    app.select_label_by_rank(0).unwrap();
    app.invert_selection();
    assert_eq!(app.selection_ranks(), vec![1, 2]);
}

#[test]
fn test_select_sequences_with_current_match() {
    let hdrs = vec![String::from("R1"), String::from("R2"), String::from("R3")];
    let seqs = vec![String::from("AA"), String::from("BA"), String::from("CC")];
    let aln = Alignment::from_vecs(hdrs, seqs);
    let mut app = App::new("TEST", aln, None);
    app.regex_search_sequences("A");
    let count = app.select_sequences_with_current_match().unwrap();
    assert_eq!(count, 2);
    assert_eq!(app.selection_ranks(), vec![0, 1]);
}

#[test]
fn test_rank_to_screenline_00() {
    let hdrs = vec![
        String::from("R1"),
        String::from("R2"),
        String::from("R3"),
        String::from("R4"),
        String::from("R5"),
    ];
    let seqs = vec![
        String::from("catgcatatg"), // 0 diffs WRT consensus
        String::from("caGgAaCaAg"), // 4 diffs WRT consensus
        String::from("catAcTtatg"), // 2 diffs WRT consensus
        String::from("cCtgcatatg"), // 1 diffs WRT consensus
        String::from("caGgAataAg"), // 3 diffs WRT consensus
    ];
    let aln = Alignment::from_vecs(hdrs, seqs);
    let mut app = App::new("TEST", aln, None);
    assert_eq!(app.ordering, vec![0, 1, 2, 3, 4]);
    // Until ordering changes, rank == screenline
    assert_eq!(0, app.rank_to_screenline(0));
    assert_eq!(1, app.rank_to_screenline(1));
    assert_eq!(2, app.rank_to_screenline(2));
    assert_eq!(3, app.rank_to_screenline(3));
    assert_eq!(4, app.rank_to_screenline(4));
    app.next_ordering_criterion();
    assert_eq!(app.ordering, vec![1, 4, 2, 3, 0]);
    assert_eq!(app.reverse_ordering, vec![4, 0, 2, 3, 1]);
    // Now the ordering is by metric, so rank != screenline
    assert_eq!(app.rank_to_screenline(0), 4);
    assert_eq!(app.rank_to_screenline(1), 0);
    assert_eq!(app.rank_to_screenline(2), 2);
    assert_eq!(app.rank_to_screenline(3), 3);
    assert_eq!(app.rank_to_screenline(4), 1);
}

#[test]
fn test_regex_lbl_search_10() {
    let hdrs = vec![
        String::from("Accipiter"),
        String::from("Aquila"),
        String::from("Milvus"),
        String::from("Buteo"),
        String::from("Pernis"),
    ];
    let seqs = vec![
        String::from("catgcatatg"), // 0 diffs WRT consensus
        String::from("caGgAaCaAg"), // 4 diffs WRT consensus
        String::from("catAcTtatg"), // 2 diffs WRT consensus
        String::from("cCtgcatatg"), // 1 diffs WRT consensus
        String::from("caGgAataAg"), // 3 diffs WRT consensus
    ];
    let aln = Alignment::from_vecs(hdrs, seqs);
    let mut app = App::new("TEST", aln, None);
    app.regex_search_labels("^A");
    match app.search_state {
        Some(state) => {
            assert_eq!(state.pattern, "^A");
            assert_eq!(state.match_linenums, vec![0, 1]);
            assert_eq!(state.current, 0);
        }
        None => panic!(),
    }
}

#[test]
fn test_regex_lbl_search_case_insensitive() {
    let hdrs = vec![String::from("Accipiter"), String::from("Aquila")];
    let seqs = vec![String::from("catg"), String::from("catg")];
    let aln = Alignment::from_vecs(hdrs, seqs);
    let mut app = App::new("TEST", aln, None);
    app.regex_search_labels("^a");
    assert!(app.is_label_selected(0));
    assert!(app.is_label_selected(1));
}

#[test]
fn test_regex_seq_search_spans() {
    let hdrs = vec![String::from("R1")];
    let seqs = vec![String::from("A-C--GT")];
    let aln = Alignment::from_vecs(hdrs, seqs);
    let mut app = App::new("TEST", aln, None);
    app.regex_search_sequences("CG");
    let spans = app.seq_search_spans().unwrap();
    assert_eq!(spans[0], vec![(2, 6)]);
    let counts = app.seq_search_counts().unwrap();
    assert_eq!(counts, (1, 1));
    let cur = app.current_seq_match().unwrap();
    assert_eq!(
        cur,
        SeqMatch {
            seq_index: 0,
            start: 2,
            end: 6
        }
    );
}

#[test]
fn test_search_ordering_groups_matches() {
    let hdrs = vec![
        String::from("R1"),
        String::from("R2"),
        String::from("R3"),
        String::from("R4"),
    ];
    let seqs = vec![
        String::from("AA--"),
        String::from("BBBB"),
        String::from("A-A-"),
        String::from("CCCC"),
    ];
    let aln = Alignment::from_vecs(hdrs, seqs);
    let mut app = App::new("TEST", aln, None);
    app.regex_search_sequences("aa");
    app.next_ordering_criterion();
    app.next_ordering_criterion();
    app.next_ordering_criterion();
    assert_eq!(app.ordering, vec![0, 2, 1, 3]);
}

#[test]
fn test_remove_sequences_preserves_search_state() {
    let hdrs = vec![String::from("R1"), String::from("R2"), String::from("R3")];
    let seqs = vec![String::from("AA"), String::from("BB"), String::from("AA")];
    let aln = Alignment::from_vecs(hdrs, seqs);
    let mut app = App::new("TEST", aln, None);
    app.regex_search_sequences("AA");
    app.next_ordering_criterion();
    app.next_ordering_criterion();
    app.next_ordering_criterion();
    app.remove_sequences(&[1]);
    assert_eq!(app.get_seq_ordering(), SeqOrdering::SearchMatch);
    assert_eq!(app.current_seq_search_pattern(), Some("AA"));
    assert_eq!(app.seq_search_spans().unwrap().len(), 2);
}

#[test]
fn test_remove_sequences_prunes_user_ordering() {
    let hdrs = vec![
        String::from("R1"),
        String::from("R2"),
        String::from("R3"),
        String::from("R4"),
    ];
    let seqs = vec![
        String::from("AA"),
        String::from("BB"),
        String::from("CC"),
        String::from("DD"),
    ];
    let aln = Alignment::from_vecs(hdrs, seqs);
    let mut app = App::new("TEST", aln, None);
    app.set_user_ordering(vec![
        String::from("R3"),
        String::from("R1"),
        String::from("R4"),
        String::from("R2"),
    ])
    .unwrap();
    app.remove_sequences(&[1]);
    assert_eq!(app.alignment.headers.len(), 3);
    assert_eq!(app.user_ordering.as_ref().unwrap().len(), 3);
    assert!(!app
        .user_ordering
        .as_ref()
        .unwrap()
        .contains(&String::from("R2")));
}

#[test]
fn test_remove_sequences_preserves_ordering_lengths() {
    let hdrs = vec![
        String::from("R1"),
        String::from("R2"),
        String::from("R3"),
        String::from("R4"),
    ];
    let seqs = vec![
        String::from("AA"),
        String::from("BB"),
        String::from("AA"),
        String::from("CC"),
    ];
    let aln = Alignment::from_vecs(hdrs, seqs);
    let mut app = App::new("TEST", aln, None);

    app.next_ordering_criterion();
    app.remove_sequences(&[1]);
    assert_eq!(app.ordering.len(), app.alignment.num_seq());
    assert_eq!(app.reverse_ordering.len(), app.alignment.num_seq());

    app.regex_search_sequences("AA");
    app.next_ordering_criterion();
    app.next_ordering_criterion();
    app.next_ordering_criterion();
    app.remove_sequences(&[0]);
    assert_eq!(app.ordering.len(), app.alignment.num_seq());
    assert_eq!(app.reverse_ordering.len(), app.alignment.num_seq());
}

#[test]
fn test_session_save_and_load() {
    let hdrs = vec![String::from("R1"), String::from("R2"), String::from("R3")];
    let seqs = vec![String::from("AA"), String::from("BB"), String::from("AA")];
    let aln = Alignment::from_vecs(hdrs, seqs);
    let mut app = App::new("TEST", aln, None);
    let tree = parse_newick("(R1,(R2,R3));").unwrap();
    let (lines, _order) = tree_lines_and_order(&tree).unwrap();
    app.tree_lines = lines;
    app.tree_panel_width = app
        .tree_lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0)
        .min(u16::MAX as usize) as u16;
    app.tree_newick = Some(String::from("(R1,(R2,R3));"));
    app.tree = Some(tree);
    app.set_notes(String::from("Session notes"));
    app.set_view_notes(String::from("View notes"));
    app.add_saved_search_with_kind(String::from("motif"), String::from("AA"), SearchKind::Regex)
        .unwrap();
    app.regex_search_sequences("AA");
    app.set_label_matches_from_tree(vec![0, 2], (0, 2));

    let mut path = std::env::temp_dir();
    path.push("msafara-test-session.msfr");
    let _ = std::fs::remove_file(&path);
    app.save_session(&path).unwrap();

    let loaded = App::from_session_file(&path).unwrap();
    assert_eq!(loaded.alignment.headers.len(), 3);
    assert_eq!(loaded.tree_lines.len(), 3);
    assert!(loaded.tree.is_some());
    assert_eq!(loaded.saved_searches().len(), 1);
    assert_eq!(loaded.current_seq_search_pattern(), Some("AA"));
    assert_eq!(loaded.selection_ranks(), vec![0, 2]);
    assert_eq!(loaded.notes(), "Session notes");
    assert_eq!(loaded.view_notes(), "View notes");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_tree_ordering_maps_header_tokens() {
    let hdrs = vec![String::from("seq 1"), String::from("seq2")];
    let seqs = vec![String::from("AA"), String::from("AA")];
    let aln = Alignment::from_vecs(hdrs, seqs);
    let mut app = App::new("TEST", aln, None);
    app.set_user_ordering(vec![String::from("seq"), String::from("seq2")])
        .unwrap();
    assert_eq!(
        app.user_ordering.unwrap(),
        vec![String::from("seq 1"), String::from("seq2")]
    );
}

#[test]
fn test_tree_ordering_maps_underscored_headers() {
    let hdrs = vec![String::from("1 CELEG-F08G5 1a"), String::from("seq2")];
    let seqs = vec![String::from("AA"), String::from("AA")];
    let aln = Alignment::from_vecs(hdrs, seqs);
    let mut app = App::new("TEST", aln, None);
    app.set_user_ordering(vec![String::from("1_CELEG-F08G5_1a"), String::from("seq2")])
        .unwrap();
    assert_eq!(
        app.user_ordering.unwrap(),
        vec![String::from("1 CELEG-F08G5 1a"), String::from("seq2")]
    );
}

#[test]
fn test_tree_ordering_strips_numeric_prefix_and_dots() {
    let hdrs = vec![String::from("CELEG-F08G5.1a"), String::from("seq2")];
    let seqs = vec![String::from("AA"), String::from("AA")];
    let aln = Alignment::from_vecs(hdrs, seqs);
    let mut app = App::new("TEST", aln, None);
    app.set_user_ordering(vec![String::from("1_CELEG-F08G5_1a"), String::from("seq2")])
        .unwrap();
    assert_eq!(
        app.user_ordering.unwrap(),
        vec![String::from("CELEG-F08G5.1a"), String::from("seq2")]
    );
}

#[test]
fn test_search_registry_add_toggle_delete() {
    let hdrs = vec![String::from("R1")];
    let seqs = vec![String::from("ACGT")];
    let aln = Alignment::from_vecs(hdrs, seqs);
    let mut app = App::new("TEST", aln, None);
    app.regex_search_sequences("CG");
    let query = app.current_seq_search_pattern().unwrap().to_string();
    app.add_saved_search(query.clone(), query).unwrap();
    assert_eq!(app.saved_searches().len(), 1);
    assert!(app.saved_searches()[0].enabled);
    assert!(app.toggle_saved_search(0));
    assert!(!app.saved_searches()[0].enabled);
    assert!(app.delete_saved_search(0));
    assert!(app.saved_searches().is_empty());
}

#[test]
fn test_parse_emboss_query() {
    assert_eq!(super::parse_emboss_query("2 ABC"), (Some(2), "ABC"));
    assert_eq!(super::parse_emboss_query("10   A*B"), (Some(10), "A*B"));
    assert_eq!(super::parse_emboss_query("ABC"), (None, "ABC"));
    assert_eq!(super::parse_emboss_query("2"), (None, "2"));
    assert_eq!(super::parse_emboss_query("2  "), (None, "2"));
}

#[test]
fn test_parse_gff_matches_header_token() {
    let headers = vec![String::from("seq 1"), String::from("seq2")];
    let sequences = vec![String::from("ABCD"), String::from("EFGH")];
    let gff = "seq\tsrc\tfeat\t2\t4\t.\t.\t.\tID=seq.1\n";
    let state = super::parse_gff_to_state(&headers, &sequences, gff, "TEST").unwrap();
    assert_eq!(state.spans_by_seq[0], vec![(1, 4)]);
    assert!(state.spans_by_seq[1].is_empty());
}
