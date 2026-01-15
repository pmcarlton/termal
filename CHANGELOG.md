# Changelog

## [Unreleased]

### Added

* Search mode with a line editor for sequence regex searches (`/`) and EMBOSS fuzzy searches (`\\`)
* Search registry with multiple saved searches, enable/disable toggles, and a Search List panel (`:s`)
* EMBOSS fuzzy search integration (fuzzpro/fuzznuc) with GFF parsing and `tools.config`
* Optional leading mismatch count for EMBOSS patterns (e.g., `2 PATTERN` -> `-pmis 2`)
* Configurable search colors via `colors.config` and mono-white as the default color mode
* Header filtering: reject current header match to `rejected<file>` (`!`) and write filtered view to `filtered<file>` (`W`)
* README caveat about terminal color schemes affecting ANSI colors
* Clustal alignment input format (`-f clustal`)
* Wishlist document for future enhancements
* SVG export of the current view (`:es`)
* MAFFT realignment with guide tree output and tree panel (`:ra`)

### Changed

* Status line now shows saved searches and the current search (including type R/E)
* Saved searches in the status line are color-coded to match highlights (background)
* Current header match is highlighted with a red background; other header matches stay white
* Current search highlight is dimmer; gap characters use half-intensity
* SVG export renders highlighted sequence matches in bold text

### Fixed

* Search navigation now keeps current search active (n/p) and shows match M of N
* Saved EMBOSS searches now highlight correctly
* Rejecting all sequences now exits cleanly with a message
* SVG export overwrite confirmation now restores the prompt flow
* MAFFT tree leaf names now map to headers with spaces/dots normalized and numeric prefixes stripped
* Tree panel uses box-drawing characters with branches reaching leaf names

## [1.3.0]

### Added

* Vim-style count prefixes for motion commands and pane resizing
* Absolute and relative jump commands for horizontal and vertical navigation
* Regex-based search in sequence headers, with forward/backward navigation
* User-defined sequence ordering (`-o`)
* User-defined colormap (`-c`)

### Changed

* Modeline is now anchored to the bottom-left corner and displays:
  * Pending command arguments (counts, search patterns)
  * Current search match index (when applicable)

### Fixed

* No longer possible to crash by widening the left pane (`>`) all the way to the
  right.
* No longer possible to obscure the sequence metric and numbers by narrowing the
  left pane (`<`).

## [1.2.0] 

### Added

* Capacity to read Stockholm format
* Capacity to read files with sequences of different lengths

### Fixed

* Out-of-bounds error when zoombox is a single character.

## [1.1.0] 2025-05-20

### Fixed

* Color maps and color schemes

## [1.0.0] 2024-05-04

Initial release
