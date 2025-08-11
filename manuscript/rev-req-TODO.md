21-Jul-2025

Dear Dr. Junier,

Manuscript ID BIOADV-2025-277 entitled "Termal: a fast and interactive
terminal-based viewer for multiple sequence alignments" which you submitted to
Bioinformatics Advances, has been reviewed.  

The comments of the Associate Editor, Alex Bateman and reviewers who reviewed
your manuscript are included at the foot of this letter.

The reviewers have recommended publication, but also suggest some minor
revisions to your manuscript.  Therefore, I invite you to respond to the
reviewers comments and revise your manuscript.

To revise your manuscript, log into https://mc.manuscriptcentral.com/bioadv and
enter your Author Centre, where you will find your manuscript title listed under
"Manuscripts with Decisions."  Under "Actions," click on "Create a Revision."
Your manuscript number has been appended to denote a revision.

You may also click the below link to start the revision process (or continue the
process if you have already started your revision) for your manuscript. If you
use the below link you will not be required to login to ScholarOne Manuscripts.

*** PLEASE NOTE: This is a two-step process. After clicking on the link, you
will be directed to a webpage to confirm. ***

https://mc.manuscriptcentral.com/bioadv?URL_MASK=165dea4169e34ef5815b79929c9d2b84

You will be unable to make your revisions on the originally submitted version of
the manuscript.  Instead, revise your manuscript using a word processing program
and save it on your computer.  Please also highlight the changes to your
manuscript within the document by using the track changes mode in MS Word or by
using bold or coloured text.

When preparing your revised manuscript for resubmission, please ensure that the
manuscript conforms to our Instructions to Authors
(https://academic.oup.com/bioinformaticsadvances/pages/instructions-to-authors).

Once the revised manuscript is prepared, you can upload it and submit it through
your Author Centre.

When submitting your revised manuscript, you will be able to respond to the
comments made by the reviewers in the space provided.  You can use this space to
document any changes you make to the original manuscript.  In order to expedite
the processing of the revised manuscript, please be as specific as possible in
your response to the reviewers.

IMPORTANT:  Your original files are available to you when you upload your
revised manuscript.  Please delete any redundant files before completing the
submission.

Because we are trying to facilitate timely publication of manuscripts submitted
to Bioinformatics Advances, your revised manuscript should be uploaded within 30
days of receiving this decision.  If it is not possible for you to submit your
revision in a reasonable amount of time, we may have to consider your paper as a
new submission.

Once again, thank you for submitting your manuscript to Bioinformatics Advances
and I look forward to receiving your revision.

Sincerely,
Dr. Michael DeGiorgio
Editor in Chief, Bioinformatics Advances
mdegiorg@fau.edu


Associate Editor: Bateman, Alex
Comments to the Author:
Dear Dr. Junier,

Thanks for your submission.  The reviewers can see the use of the tools and have
a few suggestions for improvements.  Please look at them all and address what
you can reasonably. There is no need to add editing functionality for final
acceptance. On this basis I am happy to recommend the paper is accepted subject
to minor revisions that will not require rereview.

Yours sincerely
Alex Bateman

Reviewer Comments to Authors:
Reviewer: 1

Comments to the Author

The authors present a new tool for visualisation of multiple sequence alignments
from the terminal. As the authors note, although other tools have already been
developed for this specific purpose, their tool does offer a convenient and fast
implementation sufficient for the intended use case - inspection of alignments
produced by tools running on platforms lacking graphical video support such as
HPC clusters.

I was able to manually build and also install the tool via the termal-msa cargo
package and provided public github repository (please note there is also a
preprint at https://www.preprints.org/manuscript/202504.1866/v1 which provides
incorrect links and cargo package names which should be revised to avoid
confusion). Once an alignment file has been loaded the display is responsive,
and built in help (?) enables people to find their way around quickly. Relative
sizes of row and column histograms may be a litle difficult to resolve for some,
but overall the tool does what is required efficiently and effectively. The zoom
mode in particular is useful for quickly navigating a large alignment - and no
doubt additional navigation keystrokes will be provided to even further improve
user experience.

Mandatory revisions

M1. Please check all links on the public github repository. e.g in the README -
many of these seem to point to pages on the gitlab.swiss repository which is not
publicly accessible. Links to that repository also appear in cargo.toml.

Suggested points of revision
1. Whilst the tool seems to work well in 'kitty' (as noted in the manuscript),
   the authors should note there may be issues with termal when used in
   proprietary systems - such as the standard OSX terminal (Version 2.14
   (455.1)). Whilst the alignment was displayed, the clustal colour scheme was
   not shown correctly (mostly black and white) and sequence rows were not
   correctly updated when scrolling horizontally or paging.


Suggested Improvements
1. Provide a CLI option to disable (or otherwise thread off) the initial
   calculation of conservation and consensus (also perhaps trap Ctrl-C to cancel
   the calculation).
2. termal-msa fails if the sequences are not properly aligned (ie strings are of
   unequal length). This is a shame, since an MSA browser like this is also
   useful for looking at unaligned sequences. Furthermore, some tools do not
   output alignments that are properly padded with terminal gaps, again
   preventing their use with termal-msa.

3. Practically everyone will want a '/' search command !

Reviewer: 2

Comments to the Author Great manuscript and interesting software. However, I
couldn't test it as the links on the github page to download the tool are
broken. In addition to the current functionalities, it would be great to be able
to edit the alignment, e.g. trim at the N or C-terminal, remove some sequences
or redundant sequences.

Reviewer: 3

Comments to the Author I have read the manuscript, and run the Termal tool. I
only have minor comments as the visualisation tool runs as advertised, is
reasonably intuative and very fast, even on large alignments.

One issue I faced is the lack of documentation regarding input formats, this is
left until the last lines of the README. I worked it out from the example, but
it would be nice to support more alignment formats. Stockholm would be
particularly useful for my purposes. At the very least, more informative than:
"thread 'main' panicked at src/alignment.rs" would be appreciated.

The download links in the markdown README don't seem to work, I think "v1.0.0"
should be "v1.1.0" for each.

I.e. link should be
https://github.com/sib-swiss/termal/releases/download/v1.1.0/termal-v1.1.0-linux-x86_64.tar.gz
not:
https://github.com/sib-swiss/termal/releases/download/v1.1.0/termal-v1.0.0-linux-x86_64.tar.gz
