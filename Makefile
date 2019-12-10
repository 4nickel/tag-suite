#
# This makefile is used as a sort of portable
# macro language and just dispatches to cargo
# for building. You are free to use cargo directly
# instead.
#

PREFIX=${HOME}/.local
BIN=${PREFIX}/bin
TEST_FILES="$(shell pwd)/test/files"

all: db

.PHONY: db
db:
	diesel migration run
	$(MAKE) schema

.PHONY: db-redo
db-redo:
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

.PHONY: test-files
test-files:
	mkdir -p ${TEST_FILES}/1500
	touch ${TEST_FILES}/{a,b,c}
	for n in $(seq 1 1 1500); do touch ${TEST_FILES}/1500/$n; done

.PHONY: test
test: test-files
	cargo test --verbose

.PHONY: bench
bench:
	cargo bench

.PHONY: clean
clean:
	rm -rf ${TEST_FILES}/*
	cargo clean

.PHONY: link
link:
	ln -s target/debug/tag tag
	ln -s target/debug/tdb tdb
	ln -s target/release/tag ${BIN}/tag
	ln -s target/release/tdb ${BIN}/tdb
