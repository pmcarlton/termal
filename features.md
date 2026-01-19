# Msafara Features

Msafara is a terminal UI for multiple sequence alignments with workflows tuned for
rapid motif search, filtering, and tree-guided selection. The list below reflects
the current feature set.

## Viewing and Navigation

- Zoomed-in and zoomed-out alignment views with horizontal and vertical scrolling.
- Optional labels, metrics, and bottom panes with adjustable layout.
- Ordering modes: original, metric-based, match-grouped, and tree order.
- Metrics: percent identity to consensus and ungapped length.

## Searching and Highlighting

- Header search (`"pattern`) with regex and cursor navigation.
- Sequence regex search (`/pattern`) with match navigation and highlights.
- EMBOSS fuzzy search (`\pattern`) using `fuzzpro`/`fuzznuc` with GFF parsing.
- Search registry with multiple saved searches, enable/disable, and colored labels.

## Selection and Views

- Selection tools: select by number/range (`:sn`), invert (`I`), select matches (`:sm`),
  select all (`A`), clear (`X`), and cursor-based toggle (`x`).
- Views: original/filtered/rejected plus custom views (`:vc`, `:vx`, `:vs`, `:vd`, `:mv`).
- Rejection workflow writes rejected sequences to versioned output files.

## Trees and Realignment

- MAFFT realignment with guide tree output (`:ra`) and tree panel display.
- Tree navigation (`:tn`) selects subtrees and supports half-screen scrolling.
- Auto-alignment on unaligned FASTA input (MAFFT `--maxiterate 1000 --localpair`).

## Sessions, Notes, and Export

- Session save/load to `.msfr` with per-view state, searches, trees, and notes.
- Global notes (`@`) and per-view notes (`|`).
- SVG export of the current view (`:es`).

## Configuration and Tooling

- `.msafara.config` for search colors and tool locations (EMBOSS, MAFFT).
- Clustal and Stockholm input support, plus FASTA.
