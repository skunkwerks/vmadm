

dist:
	cargo build --release
	$(MAKE) -C rel/pkgng
