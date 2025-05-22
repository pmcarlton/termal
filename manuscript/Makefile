.PHONY: clean

all: cover_letter.pdf termal.pdf

cover_letter.pdf: cover_letter.tex
	xelatex $<

termal.pdf: termal.tex termal.bib 
	xelatex $<
	bibtex termal
	xelatex $<
	xelatex $<

clean:
	$(RM) *.aux *.log *.bbl *.blg *.out *.toc *.lof *.lot 
