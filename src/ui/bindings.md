# Main Key Bindings

Run `termal -b` to see this message if it doesn't fit on screen.

Arguments (counts, search patterns), match index, and ordering mode are shown in the modeline.

Formats: use `-f` with `fasta`, `clustal`, or `stockholm`.

## Scrolling

[count]arrows: scroll by count columns/sequences;
    h,j,k,l are aliases for left, down, up, and right
[count]shift-arrows: scroll by count screenfuls
^,G,g,$: full left, bottom, top, full right

## Jumping (positions)

[count]| : jump to absolute column
[count]- : jump to absolute sequence (by current order)
[count]% : jump to vertical position (0–100%)
[count]# : jump to horizontal position (0–100%)

## Zooming

z,Z: next/previous zoom mode

## Searching (headers)

"regexp<Ret> : search sequence headers
[count]n,p   : next / previous header match (moves cursor)
[count][,]   : previous / next sequence match
!            : reject current header match (adds to rejected view, appends to rejected<file>)
Esc          : cancel search

## Searching (sequences)

/regexp<Ret> : search sequences
\\pattern<Ret> : search sequences (EMBOSS fuzzpro/fuzznuc; optional leading "N " sets -pmis)
Esc          : cancel search
P            : save current search and clear its highlights

## Extended commands (:)

:s<Ret>      : open Search List panel (a=add, c=current, d=delete, space=toggle, 1-9=select)
:es<Ret>     : export current view to SVG (prompts for path)
:ra<Ret>     : realign sequences with mafft and show tree panel (requires .termalconfig)
:tn<Ret>     : enter tree navigation mode (auto-realigns if needed)
:tt<Ret>     : toggle tree panel visibility
:rc<Ret>     : reject current match (y/n to confirm)
:ru<Ret>     : reject unmatched sequences (y/n to confirm)
:rm<Ret>     : reject matched sequences (y/n to confirm)
:rs<Ret>     : reject selected sequences
:sn<Ret>     : select header by displayed number (e.g., :sn 31)
:rn<Ret>     : reject by displayed number(s) (e.g., :rn 1,4,6-8)
:ss<Ret>     : save session to .trml (prompted, with overwrite confirmation)
:sl<Ret>     : load session from .trml (choose from list)
:vc<Ret>     : create a new view from the current view (prompts for name)
:vx<Ret>     : create a new view from selected sequences (prompts with view list)
:vs<Ret>     : switch to another view (choose from list)
:vd<Ret>     : delete a view (choose from list)
:mv<Ret>     : move selected sequences to another view (or :mv 1,4,6-8)

## Tree navigation

Right/Left or l/h : descend into child / move to parent (change range)
Up/Down or k/j    : move within current depth
Shift-Up/Down     : scroll by screen without changing selection
Esc               : exit tree navigation (selects leaves)

## Filtering

W            : write current view to its output file (orig/filt/rej/view tag)

## Adjusting the Panes

[count]<,> : widen/narrow left pane by count columns
a          : hide/show left pane        
c          : hide/show bottom pane    
f          : toggle fullscreen alignment pane 

## Video

s,S: next/previous color scheme
m,M: next/previous color map
i: toggle inverse/direct video

## Notes

@: open global notes editor (Esc to close; Ctrl-A/Ctrl-E line start/end; Ctrl-B/Ctrl-F word left/right)
|: open view notes editor (per-view)

## Selection

x: select cursor line (clears previous selection)
A: select all in view
X: clear selection
.: toggle cursor highlight
:cc<Ret> : clear cursor highlight

Monochrome direct video is the default.

## Metrics and Orderings

o,O: next/previous ordering
t,T: next/previous metric

Ordering modes are shown as o:original, o:match, o:tree, or o:length/%id.
