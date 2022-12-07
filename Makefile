
cbuild:
	cargo build
	cargo build --release

cclean:
	cargo clean

ctest:
	cargo test -- --show-output

