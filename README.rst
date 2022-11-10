##########
DataUtils
##########

Author: HanishKVC
Version: 20221110IST1418


Overview
###########

Provide a bunch of data related helper entities and logic.

This is extracted from fuzzerk, in which some of these logics started out.
Inturn when using filter-repo to extract out the specifics wrt this module,
I kept vm.rs/datam in initially, to allow some of the history behind variant,
wrt fuzzer to remain there.


Details
#########

Variant allows one to store integer, string or binary buffer.

Convertion between Binary buffer and Hex string.

Convert string into different int types through isize.

