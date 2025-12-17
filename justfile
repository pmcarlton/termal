format:
    rustfmt src/**/*.rs

tags:
    ctags -R --exclude='data/*' --exclude='target/*'
