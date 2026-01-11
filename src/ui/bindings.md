# Main Key Bindings

Run `termal -b` to see this message if it doesn't fit on screen.

Arguments (counts, search patterns) and match index are shown in the modeline.

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
[count]n,p   : next / previous match
!            : reject current header match (remove from view, append to rejected<file>)
Esc          : cancel search

## Searching (sequences)

/regexp<Ret> : search sequences
\\pattern<Ret> : search sequences (EMBOSS fuzzpro/fuzznuc)
Esc          : cancel search
:s<Ret>      : open Search List panel (a=add, d=delete, space=toggle, 1-9=select)
P            : save current search and clear its highlights
W            : write currently shown alignment to filtered<file>

## Adjusting the Panes

[count]<,> : widen/narrow left pane by count columns
a          : hide/show left pane        
c          : hide/show bottom pane    
f          : toggle fullscreen alignment pane 

## Video

s,S: next/previous color scheme
m,M: next/previous color map
i: toggle inverse/direct video

Monochrome direct video is the default.

## Metrics and Orderings

o,O: next/previous ordering
t,T: next/previous metric
