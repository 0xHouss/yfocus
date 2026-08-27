PREFIX ?= /usr/local
CARGO ?= cargo

.PHONY: build install uninstall test plugin-test validate fmt clippy bundle verify-bundle clean

build:
	$(CARGO) build --release

install: build
	install -Dm755 target/release/yfocus $(DESTDIR)$(PREFIX)/bin/yfocus

uninstall:
	rm -f $(DESTDIR)$(PREFIX)/bin/yfocus

bundle:
	scripts/build-bundle.sh

verify-bundle:
	scripts/verify-bundle.sh

test:
	$(CARGO) test --all-targets

plugin-test:
	omarchy plugin validate .
	qmllint -I "$(OMARCHY_PATH)/shell" BarWidget.qml FocusOverlay.qml || echo "qmllint not available"

validate: plugin-test

fmt:
	$(CARGO) fmt

clippy:
	$(CARGO) clippy --all-targets -- -D warnings

clean:
	$(CARGO) clean
