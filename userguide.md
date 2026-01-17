# Termal User Guide

Termal is a terminal UI for exploring multiple sequence alignments. This guide
summarizes each function and how to use it.

## Launch and Files

Termal opens an alignment file (FASTA, Clustal, or Stockholm) and renders it in
a scrollable, zoomable viewport. A `.trml` session can be loaded instead of a
sequence file to restore view-specific state, searches, and notes.

## Viewports and Navigation

The alignment pane shows the current window into the alignment. Horizontal and
vertical scrolling move the viewport, while zoom modes swap between per-residue
and downsampled views. A zoombox shows the viewport position when zoomed out.

## Labels, Cursor, and Selection

The cursor highlights a single sequence for visual inspection and is toggled
with `.`. Selections are explicit, per-view sets of sequences that actions
operate on; header search and tree navigation replace the current selection.
The selection can be cleared with `X`, and the cursor can be cleared with
`:cc`.

## Searching Sequence Headers

Header search (`"pattern`) selects all matching sequence IDs and moves the
cursor to the first match. The `n/p` keys move the cursor among the selected
set.

## Searching Sequences

Sequence search (`/pattern`) highlights matching residues and keeps a current
match index. EMBOSS fuzzy search (`\pattern`) uses `fuzzpro`/`fuzznuc` (as
configured) and highlights matches from the tool output.

## Search Registry

The Search List (`:s`) stores multiple searches with enable/disable toggles,
color labels, and optional selection by number. Stored searches remain active
until disabled or removed.

## Filtering and Rejection

Rejection removes sequences from views and writes them to a rejected output
file. In the original and filtered views, rejects also update the global
rejected set; custom views only remove sequences locally. The current selection
is the rejection target.

## Views

Views are named subsets of sequences with their own ordering, tree, searches,
selection, cursor, and notes. You can create (`:vc`), switch (`:vs`), delete
(`:vd`), and move selected sequences to another view (`:mv`).

## Ordering and Metrics

Ordering changes the vertical sequence order (original, metric-based, match
grouping, or tree order). Metrics toggle between percent identity and sequence
length for ordering and side-panel bars.

## Tree Panel and Navigation

Realignment (`:ra`) runs MAFFT to generate a guide tree and aligns sequences to
tree order. The tree panel can be toggled, and tree navigation mode (`:tn`)
lets you move among internal nodes to select subtrees that match the sequences
you will reject or move.

## Notes

Global notes (`@`) store free-form session documentation. Per-view notes (`|`)
track notes that are specific to the current view.

## Export

SVG export (`:es`) writes an SVG capture of the visible viewport, using the
current color/selection state. Output filenames can be edited before writing.

## Session Save/Load

Sessions (`:ss`, `:sl`) save and restore alignment state, views, searches, and
notes. Session files are JSON and use the `.trml` extension.

## Color and Tools Configuration

`.termalconfig` configures search colors and tool locations. Termal searches
for it in `$HOME` and then the current directory.
