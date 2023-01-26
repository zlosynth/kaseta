PATCHES = \
	patch-clean-slate.pdf \
	patch-ping-pong-echo.pdf \
	patch-saturation.pdf

.PHONY: pdf
pdf: clean pdf-images
	pdflatex manual.tex

.PHONY: pdf-images
pdf-images: $(PATCHES)
	inkscape -D schema.svg -o schema.pdf --export-latex

patch-%.pdf: patch-%.svg
	inkscape -D $< -o $@ --export-latex

.PHONY: clean
clean:
	rm -rf *.pdf *.pdf_tex *.log *.aux .auctex-auto
