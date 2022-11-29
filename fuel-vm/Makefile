.PHONY: all interpreter fuzz

all:

interpreter: ## Compile the interpreter
	@RUSTFLAGS="-C target-cpu=native" \
			  cargo build --release \
			  --bin interpreter \
			  --features log,cli
	@mkdir -p build
	@cp target/release/interpreter build/

fuzz:
	@cargo +nightly fuzz run grammar_aware --jobs 4 -- -max_len=8096 -timeout=60

help: ## Display this help screen
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'
