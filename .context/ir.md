# IR

IR is the normalized bridge between raw C++ declarations and generated wrapper code.

It contains:
- source headers
- opaque handle types for classes
- generated ABI functions
- enums
- parser support metadata

Example flow:

C++ -> parser model -> IR -> wrapper.h / wrapper.cpp
