.PHONY: pdf
pdf: clean pdf-images
	pdflatex manual.tex
	pdflatex manual.tex

.PHONY: pdf-images
pdf-images: $(PATCHES)
	inkscape -D schema.svg -o schema.pdf --export-latex

.PHONY: clean
clean:
	rm -rf *.pdf *.log *.aux .auctex-auto
