#
# This makefile is used as a sort of portable
# macro language and just dispatches to cargo
# for building. You are free to use cargo directly
# instead.
#

PREFIX=${HOME}/.local
BIN=${PREFIX}/bin

all: db

.PHONY: skel
skel:
	mkdir -p test

.PHONY: db
db: skel
	diesel migration run
	$(MAKE) schema

.PHONY: db-redo
db-redo: skel
	diesel migration redo
	$(MAKE) schema

.PHONY: schema
schema:
	sed -i 's/Integer/BigInt/g' src/db/schema.rs

.PHONY: bleed
bleed:
	cargo update
	rustup update

.PHONY: debug
debug:
	cargo build --debug

.PHONY: release
release:
	cargo build --release

.PHONY: test
test:
	cargo test --verbose

.PHONY: bench
bench:
	cargo bench

.PHONY: link
link:
	ln -s target/debug/tag tag
	ln -s target/debug/tdb tdb
	ln -s target/release/tag ${BIN}/tag
	ln -s target/release/tdb ${BIN}/tdb
