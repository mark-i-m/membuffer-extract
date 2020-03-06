# membuffer-extract

Extracts traces generated as follows:
- Let `T` be a raw trace -- a list of `u64` which represent the page numbers
  (`address >> 12`) of all memory accesses, in order of access.
- Compress `T` in chunks of arbitrary size to produce a new data stream `T'`:
    - For each chunk, compute a common prefix `p` of length `l` bytes, such
      that each address in the chunk has `p` as the most significant `l` bytes.
    - The compressed form of the chunk is then composed of (in order):
        - `p: u64`
        - `l: u64`
        - `chunk_size: u64`: the number of addresses in this chunk
        - The least significant `(8 - l)` bytes of each address, packed.
- Run `T'` through the `zlib` compressor.


## Generating traces

I've been generating traces using [Intel PIN][pin] with a customized version of
one of the packaged PinTools (which follow the PIN licensing agreement).

- Download the PIN source from Intel.
- `membuffer.cpp` should be placed in `source/tools/MemTrace` 
- `source/tools/MemTrace/makefile.rules` should be modified to add the following:

    ```make
    $(OBJDIR)membuffer$(OBJ_SUFFIX): membuffer.cpp makefile.rules
        $(CXX) $(TOOL_CXXFLAGS) $(COMP_OBJ)$@ $<

    $(OBJDIR)membuffer$(PINTOOL_SUFFIX): $(OBJDIR)membuffer$(OBJ_SUFFIX)
            $(LINKER) $(TOOL_LDFLAGS) $(LINK_EXE)$@ $^ $(TOOL_LPATHS) $(TOOL_LIBS) zlib/libz.a
    ```

    Make sure to update `zlib/libz.a` to be the path to the actual `libz.a`. In
    my case, I just compiled it from source.

- Run `make` to compile the `membuffer` PinTool.
- Run `../../../pin -t obj-intel64/membuffer.so -emit -o /tmp/membuffer.out -- ./my-workload`
    to collect a trace of the program `./my-workload`. The trace is generated
    at `/tmp/membuffer.out.PID.TID` (of course, feel free to adjust the output
    path in the command).

[pin]: https://software.intel.com/en-us/articles/pin-a-dynamic-binary-instrumentation-tool
