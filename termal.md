% Termal(1) Version 0.1 | TUI Multiple Sequence Alignment Viewer
% Thomas Junier

NAME
====

Termal - Multiple sequence alignment viewer with a text interface

SYNOPSIS
========

`termal [options] <MSA file>`

where `<MSA file>` is an alignment in multiple FastA, Clustal, or Stockholm format.

OPTIONS (SHORT)
===============

There are many options, but most are for debugging --- see the full OPTIONS section at
the end of this man page. You're most likely to use the ones listed below, but
most have equivalent key bindings (see KEY BINDINGS).

`-h, --help`

: Show the help message and exit successfully

`-i, --info`
:    Info mode (no TUI) - prints out statistics about the alignment.

`-f, --format <format>`
:    Sequence file format [fasta|clustal|stockholm] (or just f|c|s)

`-C, --no-color`
:    Disable color

`--poll-wait-time <POLL_WAIT_TIME>`
:    Poll wait time [ms] [default: 100] Used for tweaking reactivity.

`-V, --version`
:    Print version


INTERFACE
=========

Termal has a purely textual interface, and is entirely keyboard-driven.

Display
-------

Termal uses the entire screen and divides it into three main areas, as follows (see
Figure 1 below):

* Top left: sequence numbers and headers, as well as a sequence metric (A).
* Top right: alignment (B - this is the main area)
* Bottom right: position and consensus (C).

```
┌───┌──────────┌──┌ data/aln5.pep - 18/226s (0.08) x 40/105┐
│  1│JPNFFBMG_0│█▊│------------MSTT------------------------█
│  2│FMNIGCAI_0│█▊│------------MSTT-----------------------T║
│  3│JHNJIINN_0│█▊│------------MENT-----------------------T║
│  4│BPCAMGGF_0│█▊│------------MSTT-----------------------A║
│  5│EINLENDL_0│█▊│------------META----------------------DN║
│  6│LEACOLDL_0│█▊│------------MSDT-----------------------N║
│  7│JIFGGMGC_0│█▊│------------MATT-----------------------D║
│  8│PGMANCIO_0│█▋│------------MTTSQ----------------------N║
│                 │------------                -----------N║
│       A         │------------        B       -----------V║
│                 │------------                ------------║
│ 12│NDKPGHOA_0│█▌│------------MVDDSL----------------------║
│ 13│FGDGKIFP_0│█▌│------------MNLKCKMKAFLGFLKEGFFVVD------║
│ 14│MODHFIIH_0│█▌│------------MTDET----------------------T║
│ 15│FIAOOHFG_0│█▋│------------MSTDQ-----------------------║
│ 16│LCKICBJP_0│█▋│------------MTTRS-----------------------║
│ 17│DCDHNMCP_0│█▎│----------------------------------------║
│ 18│KCHCLCAP_0│█▋│------------MANES-----------------------║
└───└──────────└──└🬹═══════════════════════════════════════┘
│                 │|    :    |    :    |    :    |    :    │
│Position         │0        10                  30        4│
│Consensus        │------------Mstt-     C     ------------│
│Conservation     │            █▁▁▁                       ▁│
└─────────────────└ Press '?' for help ────────────────────┘
```
**Figure 1**: Termal's display areas.


The alignment area is always visible; the other two can be hidden to make room
for it (see KEY BINDINGS).

Zooming
-------

By default, Termal shows as much of the alignment as fits on the screen. Smaller
alignments can fit entirely on screen, but it's quite common for alignments to
be too large, at least in one dimension, sometimes both. To see more of the
alignment, there are two options:

* Scrolling: this simply shifts the displayed portion ("view port") of the
  alignment left, right, up, or down. One can move by a single line (sequence)
  or column (position), by screenfuls, or directly to the top, bottom, leftmost,
  or rightmost positions (see KEY BINDINGS).

* Zooming Out: this shows the first and last sequences, as well as evenly-spaced
  sequences in between so as to show as many sequences as possible. The same
  sampling is applied to columns. A box shows the location of the view port,
  that is, what part of the alignment would fill the alignment area when zooming
  back in. The zoom box can be moved using the same commands as for scrolling
  (see above).


KEY BINDINGS
============

Scrolling
------

* h,j,k,l: move view port / zoom box left, down, up, right
* H,J,K,L: like h,j,k,l, but large motions
* ^,G,g,$: full left, bottom, top, full right

Zooming
-------

* z,Z    : cycle through zoom modes
* r      : highlight zoom box residues in consensus
* v      : show view guides

Pane Size
---------

* <,>    : widen/narrow label pane
* a      : hide/show label pane

Other
-----

* Q,q    : quit
* ?      : help
* @      : notes editor (Esc to close; Ctrl-A/Ctrl-E line start/end; Ctrl-B/Ctrl-F word left/right)

Searching
---------

* "regexp<Ret> : search sequence headers
* n,p          : next / previous header match (current match highlighted in red)
* !            : reject current header match (remove from view, append to rejected<file>)
* /regexp<Ret> : search sequences
* \\pattern<Ret> : search sequences (EMBOSS fuzzpro/fuzznuc; optional leading "N " sets -pmis)
* [,]          : previous / next sequence match (current match underlined)
* P            : save current sequence search and clear its highlights

Extended commands (:)
--------------------

* :s<Ret>      : open Search List panel (a=add, c=current, d=delete, space=toggle, 1-9=select)
* :es<Ret>     : export current view to SVG (prompts for path)
* :ra<Ret>     : realign sequences with mafft and show tree panel (requires .termalconfig)
* :tn<Ret>     : enter tree navigation mode (auto-realigns if needed)
* :tt<Ret>     : toggle tree panel visibility
* :rc<Ret>     : reject current match (y/n to confirm)
* :ru<Ret>     : reject unmatched sequences (y/n to confirm)
* :rm<Ret>     : reject matched sequences (y/n to confirm)
* :rk<Ret>     : reject marked sequences (from label search or tree selection)
* :ur<Ret>     : undo last rejection (restores file and sequences)
* :sn<Ret>     : select header by displayed number (e.g., :sn 31)
* :rn<Ret>     : reject by displayed number(s) (e.g., :rn 1,4,6-8)
* :ss<Ret>     : save session to .trml (prompted, with overwrite confirmation)
* :sl<Ret>     : load session from .trml (choose from list)

Tree navigation (:tn)
---------------------

* Right/Left or l/h : descend into child / move to parent (change range)
* Up/Down or k/j    : move within current depth
* Esc               : exit tree navigation (marks selected leaves)

Filtering
---------

* W            : write currently shown alignment to filtered<file>

OPTIONS
=======

`-h, --help`

: Show the help message and exit successfully

You can also pass a `.trml` session file as the alignment argument to restore a saved session.


`-i, --info`
:    Info mode (no TUI)

`-w, --width <WIDTH>`
:    Fixed terminal width (mostly used for testing/debugging)

`-t, --height <HEIGHT>`
:    Fixed terminal height ("tall" -- -h is already used)

`-L, --hide-labels-pane`
:    Start with labels pane hidden

`-B, --hide-bottom-pane`
:    Start with bottom pane hidden

`-D, --debug`
:    (Currently no effect)

`-C, --no-color`
:    Disable color

`--no-scrollbars`
:    Disable scrollbars (mostly for testing)

`--poll-wait-time <POLL_WAIT_TIME>`
:    Poll wait time [ms] [default: 100]

`--panic`
:    Panic (for testing)

`--no-zoom-box`
:    Do not show zoom box (zooming itself is not disabled)

`--no-zb-guides`
:    Do not show zoom box guides (only useful if zoom box not shown)

`-h, --help`
:    Print help

`-V, --version`
:    Print version

BUGS AND LIMITATIONS
====================

* Termal cannot yet read Phylip or other formats beyond fasta/clustal/stockholm.

* A fast terminal is recommended (e.g., Alacritty, Ghostty, or WezTerm).
