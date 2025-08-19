.PHONY: clean zip

all: cover_letter.pdf termal.pdf termal-preprint.pdf cover_letter_revision.pdf

cover_letter_revision.pdf: cover_letter_revision.tex
	xelatex $<

cover_letter.pdf: cover_letter.tex
	xelatex $<

termal.pdf: termal.tex termal.bib 
	xelatex $<
	bibtex termal
	xelatex $<
	xelatex $<

termal-preprint.pdf: termal-preprint.tex termal.bib 
	xelatex $<
	bibtex termal-preprint
	xelatex $<
	xelatex $<

SUBMISSION_DIR = latex-src

latex-src.zip: termal-preprint.tex body.tex abstract.tex termal.bib \
	README-preprint.md figure-1.pdf
	# Clean and recreate submission dir
	rm -rf $(SUBMISSION_DIR)
	mkdir $(SUBMISSION_DIR)
	# Symlink required files (adjust as needed)
	ln -s ../termal-preprint.tex $(SUBMISSION_DIR)/termal-preprint.tex
	ln -s ../body.tex $(SUBMISSION_DIR)/body.tex
	ln -s ../abstract.tex $(SUBMISSION_DIR)/abstract.tex
	ln -s ../termal.bib $(SUBMISSION_DIR)/termal.bib
	ln -s ../figure-1.pdf $(SUBMISSION_DIR)/figure-1.pdf
	ln -s ../README-preprint.md $(SUBMISSION_DIR)/README.md
	# Create zip archive, following symlinks
	zip -r $@ $(SUBMISSION_DIR)

clean:
	$(RM) *.aux *.log *.bbl *.blg *.out *.toc *.lof *.lot latex-src.zip
