# termal

## (a fork of the original sib-swiss/termal that I am modifying for my own use cases)

Tired of using vim as an MSA viewer, and also of fiddling with giant programs like Jalview, I knew someone would have made a TUI for this, and I found `termal` to be the best. I've made some modifications for highlighting motifs, generating and viewing MAFFT alignment trees, exporting to SVG, and reordering and filtering individual sequences.

See `wishlist.md` for proposed enhancements and future features.

## Terminal colors and themes

Termal relies on standard ANSI colors. If your terminal theme remaps ANSI black to a non-black color, dark backgrounds may appear tinted. For best results, use a theme where ANSI black is pure black and the default background is also black. If colors look off, try a different theme or adjust your terminal palette.
