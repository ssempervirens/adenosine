
.PHONY: help
help: ## Print info about all commands
	@echo "Commands:"
	@echo
	@grep -E '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "    \033[01;32m%-20s\033[0m %s\n", $$1, $$2}'

.PHONY: test
test: build ## Run all tests (requires Cargo.lock up to date)
	cargo test --locked

.PHONY: lint
lint: ## Run syntax/style checks
	# for now, allow function calls in '.or()'
	cargo clippy -- --no-deps -A clippy::or_fun_call

.PHONY: fmt
fmt: ## Run syntax re-formatting
	cargo fmt

.PHONY: build
build: ## Build
	cargo build

.PHONY: build-release
build-release: ## Build for release (requires Cargo.lock up to date)
	cargo build --release --locked

.PHONY: completions
completions: build  ## generate shell completions
	./target/debug/adenosine --shell-completions bash status > extra/adenosine.bash_completions
	./target/debug/adenosine --shell-completions bash status > extra/adenosine.zsh_completions
	./target/debug/adenosine-pds --shell-completions bash generate-secret > extra/adenosine-pds.bash_completions
	./target/debug/adenosine-pds --shell-completions bash generate-secret > extra/adenosine-pds.zsh_completions

extra/adenosine.1: extra/adenosine.1.scdoc
	scdoc < extra/adenosine.1.scdoc > extra/adenosine.1

extra/adenosine-pds.1: extra/adenosine-pds.1.scdoc
	scdoc < extra/adenosine-pds.1.scdoc > extra/adenosine-pds.1

extra/adenosine.1.md: extra/adenosine.1
	pandoc extra/adenosine.1 -o extra/adenosine.1.md

extra/adenosine-pds.1.md: extra/adenosine-pds.1
	pandoc extra/adenosine-pds.1 -o extra/adenosine-pds.1.md

.PHONY: manpage
manpage: extra/adenosine.1.md extra/adenosine-pds.1.md ## Rebuild manpages using scdoc and pandoc

.PHONY: deb
deb: ## Build debian packages (.deb files)
	cargo deb -p adenosine-cli
	cargo deb -p adenosine-pds
