Dear Editors,

Thank you for the constructive and insightfule comments. Please find my
responses blow.

> The authors present a new tool for visualisation of multiple sequence alignments
> from the terminal. As the authors note, although other tools have already been
> developed for this specific purpose, their tool does offer a convenient and fast
> implementation sufficient for the intended use case - inspection of alignments
> produced by tools running on platforms lacking graphical video support such as
> HPC clusters.
> 
> I was able to manually build and also install the tool via the termal-msa cargo
> package and provided public github repository (please note there is also a
> preprint at https://www.preprints.org/manuscript/202504.1866/v1 which provides
> incorrect links and cargo package names which should be revised to avoid
> confusion).

Thanks for finding this error. There is now a new version of the preprint
(https://www.preprints.org/manuscript/202504.1866/v3) which matches the
manuscript; in particular it has the correct crate name.

> Once an alignment file has been loaded the display is responsive,
> and built in help (?) enables people to find their way around quickly. Relative
> sizes of row and column histograms may be a litle difficult to resolve for some,
> but overall the tool does what is required efficiently and effectively. The zoom
> mode in particular is useful for quickly navigating a large alignment - and no
> doubt additional navigation keystrokes will be provided to even further improve
> user experience.

This is indeed in the TODO list - jumping to a given position or sequence (by
number or name), among other features. 

> Mandatory revisions
> 
> M1. Please check all links on the public github repository. e.g in the README -
> many of these seem to point to pages on the gitlab.swiss repository which is not
> publicly accessible. Links to that repository also appear in cargo.toml.
> 

All links to gitlab.swiss have been replaced by links to github.com/sib-swiss.

> Suggested points of revision

> 1. Whilst the tool seems to work well in 'kitty' (as noted in the manuscript),
>    the authors should note there may be issues with termal when used in
>    proprietary systems - such as the standard OSX terminal (Version 2.14
>    (455.1)). Whilst the alignment was displayed, the clustal colour scheme was
>    not shown correctly (mostly black and white) and sequence rows were not
>    correctly updated when scrolling horizontally or paging.

The recommendation for 24-bit ("true") colour is noted in the manuscript
in section 'Performance and Limitations', but the README has been updated to
make it more apparent: this is now mentioned in the first section (Summary),
bullet point list item #4; the reader is now also referred to the LIMITATIONS
section, which contains suggestions for OSX terminal emulators that support
24-bit colour. 

> Suggested Improvements
> 1. Provide a CLI option to disable (or otherwise thread off) the initial
>    calculation of conservation and consensus (also perhaps trap Ctrl-C to cancel
>    the calculation).

I have not noticed any performance hit due to the computation of the consensus,
even on large alignments. But it could certainly be done - I will add this to the
TODO list.

> 2. termal-msa fails if the sequences are not properly aligned (ie strings are of
>    unequal length). This is a shame, since an MSA browser like this is also
>    useful for looking at unaligned sequences. Furthermore, some tools do not
>    output alignments that are properly padded with terminal gaps, again
>    preventing their use with termal-msa.

Thanks for pointing out this use case. Termal can now read sequence files with
unequal sequence lengths. Any sequence shorter than the longest one is padded
with trailing space characters.
> 
> 3. Practically everyone will want a '/' search command !

Certainly - this is in fact already on my TODO list, but slated for the next
release as it will entail significant extra code to read a search pattern, move
between occurrences, and highlight them.

> Reviewer: 2
> 
> Comments to the Author Great manuscript and interesting software. However, I
> couldn't test it as the links on the github page to download the tool are
> broken.

Thanks for pointing this out, and sorry for not finding the problem before
submitting the manuscript. The version numbers were off. This has been fixed.

> In addition to the current functionalities, it would be great to be able
> to edit the alignment, e.g. trim at the N or C-terminal, remove some sequences
> or redundant sequences.
> 

Agreed. This is also on my TODO list for the more distant future.

> Reviewer: 3
> 
> Comments to the Author I have read the manuscript, and run the Termal tool. I
> only have minor comments as the visualisation tool runs as advertised, is
> reasonably intuative and very fast, even on large alignments.
> 
> One issue I faced is the lack of documentation regarding input formats, this is
> left until the last lines of the README.

The accepted sequence/alignment file formats (FastA and now also Stockholm) are
now mentioned in the first section of the README (Summary, bullet point list
item #3). 

> I worked it out from the example, but
> it would be nice to support more alignment formats. Stockholm would be
> particularly useful for my purposes. At the very least, more informative than:
> "thread 'main' panicked at src/alignment.rs" would be appreciated.

Termal can now read Stockholm format. The README and manuscript have been
changed accordingly. Other formats (in particular PHYLIP and CLUSTAL) are in the
TODO list.

> The download links in the markdown README don't seem to work, I think "v1.0.0"
> should be "v1.1.0" for each.

This has now been fixed (see Reviewer #2).
