.PHONY: def #the DEFAULT linecount func. multi-threaded.
def:
	@cargo r 

.PHONY: display #displays filetree. single-threaded.
display:
	@cargo r -- -d 

flame:
	@cargo flamegraph --root -b lc
	open -a Safari flamegraph.svg
# .PHONY: display-async #displays filetree. multi-threaded (unreliable print order)
# display-async:
# 	@cargo r -- --test-async
# 
