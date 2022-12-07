############
DataUtilsK
############

Author: HanishKVC
Version: 20221110IST1418
License: LGPL-3.0+


Overview
###########

Provide a bunch of data related helper entities and logic.

History
==========

Initial set extracted from fuzzerk, in which some of these logics started out.
Inturn when using filter-repo to extract out the specifics wrt this module,
I kept vm.rs/datam in initially, to allow some of the history behind variant,
wrt fuzzer to remain there.

Some of the data processing helpers moved from gameplaypgnd.

Details
#########

Variant allows one to store a integer, string or a binary/byte buffer
in the same variable/data entity.

Convertion between Binary/Byte buffer and Hex string.

Convert string into different int types through isize.

Do sliding windowed averaging and cross correlation on vector of data.

