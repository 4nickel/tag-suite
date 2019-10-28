
.PHONY: db
db:
	sed -i 's/Integer/BigInt/g' src/db/schema.rs

.PHONY: bleed
bleed:
	cargo update
	rustup update
