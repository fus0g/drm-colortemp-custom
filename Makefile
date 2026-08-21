# Minimal Makefile for drm-custom-colorfix

.DEFAULT_GOAL := build

NAME := drm-custom-colorfix
VERSION := 2.3.0
RELEASE := 1
RPMBUILD_DIR := $(shell pwd)/build-rpm

.PHONY: build test clean install uninstall rpm

build:
	@echo "Building $(NAME) (release)..."
	cargo build --release
	@echo "✓ Build complete: target/release/$(NAME)"

test:
	@echo "Running tests..."
	cargo test --release

install: build
	@echo "Installing $(NAME)..."
	install -D -m 755 target/release/$(NAME) /usr/local/bin/$(NAME)
	install -D -m 644 scripts/$(NAME).service /etc/systemd/system/$(NAME).service
	install -D -m 644 $(NAME).conf /etc/default/$(NAME).conf
	systemctl daemon-reload
	@echo "✓ Installation complete"

uninstall:
	@echo "Uninstalling $(NAME)..."
	rm -f /usr/local/bin/$(NAME)
	rm -f /etc/systemd/system/$(NAME).service
	rm -f /etc/default/$(NAME).conf
	systemctl daemon-reload
	@echo "✓ Uninstall complete"

rpm:
	@echo "Packaging RPM for $(NAME)-$(VERSION)..."
	rm -rf $(RPMBUILD_DIR)
	mkdir -p $(RPMBUILD_DIR)/{BUILD,RPMS,SOURCES,SPECS,SRPMS}
	# Create source archive from working directory
	tar --exclude='./build-rpm' --exclude='./target' --exclude='./.git' -czf $(RPMBUILD_DIR)/SOURCES/$(NAME)-$(VERSION).tar.gz --transform 's,^\./,$(NAME)-$(VERSION)/,' .
	cp packaging/rpm/$(NAME).spec $(RPMBUILD_DIR)/SPECS/
	rpmbuild --define "_topdir $(RPMBUILD_DIR)" -ba $(RPMBUILD_DIR)/SPECS/$(NAME).spec
	@echo ""
	@echo "✓ RPM Package successfully built!"
	@ls -la $(RPMBUILD_DIR)/RPMS/*/*.rpm

clean:
	cargo clean
	rm -rf $(RPMBUILD_DIR)
	@echo "✓ Clean complete"
