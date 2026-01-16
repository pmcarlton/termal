// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Thomas Junier

use serde::{Deserialize, Serialize};

use crate::app::{LabelSearchSource, SearchKind};

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionFile {
    pub version: u32,
    pub source_filename: String,
    pub headers: Vec<String>,
    pub sequences: Vec<String>,
    pub tree_lines: Option<Vec<String>>,
    pub tree_newick: Option<String>,
    pub saved_searches: Vec<SessionSearchEntry>,
    pub current_search: Option<SessionCurrentSearch>,
    pub label_search: Option<SessionLabelSearch>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionSearchEntry {
    pub id: usize,
    pub name: String,
    pub query: String,
    pub kind: SessionSearchKind,
    pub enabled: bool,
    pub color: (u8, u8, u8),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionCurrentSearch {
    pub kind: SessionSearchKind,
    pub pattern: String,
    pub current_match: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionLabelSearch {
    pub pattern: String,
    pub current: Option<usize>,
    pub matches: Option<Vec<usize>>,
    pub source: Option<SessionLabelSource>,
    pub tree_range: Option<(usize, usize)>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum SessionSearchKind {
    Regex,
    Emboss,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum SessionLabelSource {
    Regex,
    Tree,
}

impl From<SearchKind> for SessionSearchKind {
    fn from(kind: SearchKind) -> Self {
        match kind {
            SearchKind::Regex => SessionSearchKind::Regex,
            SearchKind::Emboss => SessionSearchKind::Emboss,
        }
    }
}

impl From<SessionSearchKind> for SearchKind {
    fn from(kind: SessionSearchKind) -> Self {
        match kind {
            SessionSearchKind::Regex => SearchKind::Regex,
            SessionSearchKind::Emboss => SearchKind::Emboss,
        }
    }
}

impl From<LabelSearchSource> for SessionLabelSource {
    fn from(source: LabelSearchSource) -> Self {
        match source {
            LabelSearchSource::Regex => SessionLabelSource::Regex,
            LabelSearchSource::Tree => SessionLabelSource::Tree,
        }
    }
}

impl From<SessionLabelSource> for LabelSearchSource {
    fn from(source: SessionLabelSource) -> Self {
        match source {
            SessionLabelSource::Regex => LabelSearchSource::Regex,
            SessionLabelSource::Tree => LabelSearchSource::Tree,
        }
    }
}
