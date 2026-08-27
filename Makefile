SHELL := /bin/bash
PREFIX ?= /usr/local
BUN ?= bun

.PHONY: build bundle verify-bundle test plugin-test validate fmt clean

build:
	./build.sh

install: build
	install -Dm755 bin/yfocus $(DESTDIR)$(PREFIX)/bin/yfocus
	# also install arch ELFs if they exist (from `make bundle`)
	install -Dm755 bin/yfocus-x86_64 $(DESTDIR)$(PREFIX)/bin/yfocus-x86_64 2>/dev/null || true
	install -Dm755 bin/yfocus-aarch64 $(DESTDIR)$(PREFIX)/bin/yfocus-aarch64 2>/dev/null || true

uninstall:
	rm -f $(DESTDIR)$(PREFIX)/bin/yfocus
	rm -f $(DESTDIR)$(PREFIX)/bin/yfocus-x86_64
	rm -f $(DESTDIR)$(PREFIX)/bin/yfocus-aarch64

bundle:
	scripts/build-bundle.sh

verify-bundle:
	scripts/verify-bundle.sh

test:
	$(BUN) test

plugin-test:
	omarchy plugin validate .
	qmllint -I "$(OMARCHY_PATH)/shell" BarWidget.qml FocusOverlay.qml || echo "qmllint not available"

validate: plugin-test

clean:
	rm -rf target
	rm -f bin/yfocus bin/yfocus-*.sha256 bin/yfocus.srcid
