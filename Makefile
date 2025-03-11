.PHONY: async #the DEFAULT linecount func. multi-threaded.
default:
	cargo r -- -b

.PHONY: verbose #displays filetree. single-threaded.
verbose:
	cargo r -- -v -b

.PHONY: verbose-async #displays filetree. multi-threaded (unreliable print order)
verbose-async:
	cargo r -- --test-async

