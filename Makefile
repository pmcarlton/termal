.PHONY: test clean install fmt release

VERSION=v1.0.0
RUST_SOURCES = $(shell find src -name '*.rs')
LINUX_BINARY = ./target/release/msafara
LINUX_STATIC_BINARY = target/x86_64-unknown-linux-musl/release/msafara
COMPRESSED_LINUX_STATIC_BINARY = msafara-$(VERSION)-linux-x86_64.tar.gz
COMPRESSED_LINUX_STATIC_BINARY_SHA256 = $(COMPRESSED_LINUX_STATIC_BINARY).sha256
WINDOWS_BINARY = ./target/x86_64-pc-windows-gnu/release/msafara.exe
INSTALL_DIR = /usr/local/bin
MAN_DIR = /usr/share/man
BINARIES = $(LINUX_BINARY) $(LINUX_STATIC_BINARY) $(WINDOWS_BINARY) 
COMPRESSED_BINARIES = $(COMPRESSED_LINUX_STATIC_BINARY) 

all: $(BINARIES) $(COMPRESSED_BINARIES) msafara.1.gz

release: $(COMPRESSED_BINARIES) $(COMPRESSED_LINUX_STATIC_BINARY_SHA256)

$(COMPRESSED_LINUX_STATIC_BINARY): $(LINUX_STATIC_BINARY)
	tar -czvf $@ -C $(dir $(LINUX_STATIC_BINARY)) $(notdir $(LINUX_STATIC_BINARY))

$(COMPRESSED_LINUX_STATIC_BINARY_SHA256): $(COMPRESSED_LINUX_STATIC_BINARY)
	sha256sum $< > $@

$(LINUX_BINARY): $(RUST_SOURCES)
	cargo build --release

$(LINUX_STATIC_BINARY): $(RUST_SOURCES)
	cargo build --release --target x86_64-unknown-linux-musl
	strip $@

$(WINDOWS_BINARY): $(RUST_SOURCES)
	cargo build --release --target x86_64-pc-windows-gnu

msafara.1.gz: msafara.1
	gzip -kf $<

msafara.1: msafara.md
	pandoc --standalone --to=man $< > $@

tags: $(RUST_SOURCES)
	ctags -R --exclude='data/*' --exclude='target/*'

fmt:
	rustfmt src/**/*.rs

roadmap.pdf: roadmap.md meta.yaml
	pandoc --standalone --metadata-file meta.yaml --to=latex \
				--filter pandoc-crossref --citeproc --number-sections \
				--output $@ $<

install: 
	install -m 755 $(LINUX_BINARY) $(INSTALL_DIR)
	install -m 644 msafara.1.gz $(MAN_DIR)/man1

test:
	cargo test 2> /dev/null
	make -C app-tests/ test

clean:
	$(RM) msafara.1

mrproper: clean
	cargo clean
	$(RM) $(BINARIES) $(COMPRESSED_LINUX_STATIC_BINARY_SHA256)
