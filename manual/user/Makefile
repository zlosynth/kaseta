.PHONY: pdf
pdf: clean
	inkscape -D schema.svg -o schema.pdf --export-latex
	inkscape -D schema-inverted.svg -o schema-inverted.pdf --export-latex
	pdflatex manual.tex

.PHONY: clean
clean:
	rm -rf *.pdf *.pdf_tex *.log *.aux .auctex-auto
