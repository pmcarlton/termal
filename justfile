format:
    rustfmt src/**/*.rs

tags:
    ctags -R --exclude='data/*' --exclude='target/*'

test:
	cargo test 2> /dev/null

app-test:
	make -C app-tests/ test
