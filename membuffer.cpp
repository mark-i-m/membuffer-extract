/*
 * Copyright 2002-2019 Intel Corporation.
 *
 * This software is provided to you as Sample Source Code as defined in the
 * accompanying End User License Agreement for the Intel(R) Software Development
 * Products ("Agreement") section 1.L.
 *
 * This software and the related documents are provided as is, with no express
 * or implied warranties, other than those that are expressly stated in the
 * License.
 */

/*
 * Sample buffering tool
 *
 * This tool collects an address trace of instructions that access memory
 * by filling a buffer.  When the buffer overflows,the callback writes all
 * of the collected records to a file.
 *
 * This tool does a similar task as memtrace.cpp, but it uses the buffering api.
 */

#include "pin.H"
#include <cmath>
#include <cstddef>
#include <cstdlib>
#include <fstream>
#include <iostream>
#include <unistd.h>
#include <zlib.h>

#include <unordered_map>

using std::cerr;
using std::endl;
using std::hex;
using std::ofstream;
using std::string;

/*
 * Knobs for tool
 */

/*
 * Name of the output file
 */
KNOB<string> KnobOutputFile(KNOB_MODE_WRITEONCE, "pintool", "o",
                            "membuffer.out", "output file");

/*
 * Emit the address trace to the output file
 */
KNOB<BOOL> KnobEmitTrace(KNOB_MODE_WRITEONCE, "pintool", "emit", "0",
                         "emit a trace in the output file");

/* Struct for holding memory references.
 */
struct MEMREF {
  ADDRINT page;
};

BUFFER_ID bufId;

TLS_KEY mlog_key;

#define NUM_BUF_PAGES 1024

#define CHUNK 16384

/*
 * MLOG - thread specific data that is not handled by the buffering API.
 */
class MLOG {
public:
  MLOG(THREADID tid);
  ~MLOG();

  VOID DumpBufferToFile(struct MEMREF *reference, UINT64 numElements,
                        THREADID tid);
  VOID Deflate(unsigned char *buf, size_t size);

private:
  ofstream _ofile;
  z_stream _zstrm;
  unsigned char _chunk_out[CHUNK];

  std::unordered_map<UINT64, UINT64> hist;

  VOID dump() const;
};

MLOG::MLOG(THREADID tid) {
  if (KnobEmitTrace) {
    const string filename =
        KnobOutputFile.Value() + "." + decstr(getpid()) + "." + decstr(tid);

    _ofile.open(filename.c_str(),
                std::fstream::out | std::fstream::binary | std::fstream::app);

    if (!_ofile) {
      cerr << "Error: could not open output file." << endl;
      exit(1);
    }

    _zstrm.zalloc = Z_NULL;
    _zstrm.zfree = Z_NULL;
    _zstrm.opaque = Z_NULL;

    if (deflateInit(&_zstrm, Z_DEFAULT_COMPRESSION) != Z_OK) {
      cerr << "Error: could not init zstream." << endl;
      exit(1);
    }
  }
}

VOID MLOG::dump() const {
  std::cout << "DONE" << std::endl;

  for (auto it = hist.begin(); it != hist.end(); ++it) {
    std::cout << std::hex << it->first << " " << std::dec << it->second
              << std::endl;
  }
}

MLOG::~MLOG() {
  if (KnobEmitTrace) {
    int ret;
    do {
      _zstrm.avail_out = CHUNK;
      _zstrm.next_out = _chunk_out;
      ret = deflate(&_zstrm, Z_FINISH);
      assert(ret != Z_STREAM_ERROR);

      _ofile.write((char *)_chunk_out, CHUNK - _zstrm.avail_out);
    } while (ret != Z_STREAM_END);
    (void)deflateEnd(&_zstrm);
    _ofile.close();
  }
}

UINT64 prefixLenCompute(UINT64 common) {
  common |= common >> 1;
  common |= common >> 2;
  common |= common >> 4;
  common |= common >> 8;
  common |= common >> 16;
  common |= common >> 32;

  common >>= 1;

  common += 1;

  return ((UINT64)log2(common)) >> 3;
}

UINT64 getMSBytes(int n, UINT64 val) {
  switch (n) {
  case 0:
    return 0;
  case 1:
    return val & 0xff00000000000000;
  case 2:
    return val & 0xffff000000000000;
  case 3:
    return val & 0xffffff0000000000;
  case 4:
    return val & 0xffffffff00000000;
  case 5:
    return val & 0xffffffffff000000;
  case 6:
    return val & 0xffffffffffff0000;
  case 7:
    return val & 0xffffffffffffff00;
  case 8:
    return val & 0xffffffffffffffff;
  default:
    assert(false);
  }
}

VOID MLOG::Deflate(unsigned char *buf, size_t size) {
  int ret;
  _zstrm.avail_in = size;
  _zstrm.next_in = buf;
  do {
    _zstrm.avail_out = CHUNK;
    _zstrm.next_out = _chunk_out;
    ret = deflate(&_zstrm, Z_NO_FLUSH);
    assert(ret != Z_STREAM_ERROR);

    _ofile.write((char *)_chunk_out, CHUNK - _zstrm.avail_out);
  } while (_zstrm.avail_out == 0);
  assert(_zstrm.avail_in == 0);
}

VOID MLOG::DumpBufferToFile(struct MEMREF *reference, UINT64 numElements,
                            THREADID tid) {
  // We avoid storing common bytes by storing them once per dumped buffer.

  // Compute common prefix
  UINT64 common_and = 0xFFFFFFFFFFFFFFFF;
  UINT64 common_or = 0;
  for (UINT64 i = 0; i < numElements; ++i) {
    const UINT64 page = reference[i].page >> 12;
    common_and &= page;
    common_or |= page;
  }

  UINT64 common = common_and & ~(common_and ^ common_or);

  // Compute prefix length -- that is, we find the first zero bit in `common`
  UINT64 prefixLen = prefixLenCompute(common);
  UINT64 remaining = sizeof(UINT64) - prefixLen;

  common = getMSBytes(prefixLen, common);

  const size_t size = 3 * sizeof(UINT64) + remaining * numElements;
  unsigned char *buff = (unsigned char *)malloc(size);
  if (!buff) {
    std::cerr << "unable to alloc buffer" << std::endl;
    exit(1);
  }

  unsigned char *next = buff;
  *(UINT64 *)next = common;
  next += sizeof(UINT64);
  *(UINT64 *)next = prefixLen;
  next += sizeof(UINT64);
  *(UINT64 *)next = numElements;
  next += sizeof(UINT64);

  for (UINT64 i = 0; i < numElements; i++, reference++) {
    const UINT64 page = (reference->page >> 12) & ~common;
    assert((page | common) == (reference->page >> 12));
    memcpy(next, (unsigned char *)&page, remaining);
    next += remaining;

    hist[reference->page >> 21]++;
  }

  Deflate(buff, size);

  _zstrm.avail_in = 0;
  _zstrm.next_in = Z_NULL;

  free(buff);
}

/*!
 *  Print out help message.
 */
INT32 Usage() {
  cerr << "This tool demonstrates the basic use of the buffering API." << endl
       << endl;

  return -1;
}

/*
 * Insert code to write data to a thread-specific buffer for instructions
 * that access memory.
 */
VOID Trace(TRACE trace, VOID *v) {
  // Insert a call to record the effective address.
  for (BBL bbl = TRACE_BblHead(trace); BBL_Valid(bbl); bbl = BBL_Next(bbl)) {
    for (INS ins = BBL_InsHead(bbl); INS_Valid(ins); ins = INS_Next(ins)) {
      if (INS_IsMemoryRead(ins) && INS_IsStandardMemop(ins)) {
        INS_InsertFillBuffer(ins, IPOINT_BEFORE, bufId, IARG_MEMORYREAD_EA,
                             offsetof(struct MEMREF, page), IARG_END);
      }

      if (INS_HasMemoryRead2(ins) && INS_IsStandardMemop(ins)) {
        INS_InsertFillBuffer(ins, IPOINT_BEFORE, bufId, IARG_MEMORYREAD2_EA,
                             offsetof(struct MEMREF, page), IARG_END);
      }

      if (INS_IsMemoryWrite(ins) && INS_IsStandardMemop(ins)) {
        INS_InsertFillBuffer(ins, IPOINT_BEFORE, bufId, IARG_MEMORYWRITE_EA,
                             offsetof(struct MEMREF, page), IARG_END);
      }
    }
  }
}

/**************************************************************************
 *
 *  Callback Routines
 *
 **************************************************************************/

/*!
 * Called when a buffer fills up, or the thread exits, so we can process it or
 * pass it off as we see fit.
 * @param[in] id		buffer handle
 * @param[in] tid		id of owning thread
 * @param[in] ctxt		application context
 * @param[in] buf		actual pointer to buffer
 * @param[in] numElements	number of records
 * @param[in] v			callback value
 * @return  A pointer to the buffer to resume filling.
 */
VOID *BufferFull(BUFFER_ID id, THREADID tid, const CONTEXT *ctxt, VOID *buf,
                 UINT64 numElements, VOID *v) {
  if (!KnobEmitTrace)
    return buf;

  struct MEMREF *reference = (struct MEMREF *)buf;

  MLOG *mlog = static_cast<MLOG *>(PIN_GetThreadData(mlog_key, tid));

  mlog->DumpBufferToFile(reference, numElements, tid);

  return buf;
}

VOID ThreadStart(THREADID tid, CONTEXT *ctxt, INT32 flags, VOID *v) {
  // There is a new MLOG for every thread.  Opens the output file.
  MLOG *mlog = new MLOG(tid);

  // A thread will need to look up its MLOG, so save pointer in TLS
  PIN_SetThreadData(mlog_key, mlog, tid);
}

VOID ThreadFini(THREADID tid, const CONTEXT *ctxt, INT32 code, VOID *v) {
  MLOG *mlog = static_cast<MLOG *>(PIN_GetThreadData(mlog_key, tid));

  delete mlog;

  PIN_SetThreadData(mlog_key, 0, tid);
}

/*!
 * The main procedure of the tool.
 * This function is called when the application image is loaded but not yet
 * started.
 * @param[in]   argc            total number of elements in the argv array
 * @param[in]   argv            array of command line arguments,
 *                              including pin -t <toolname> -- ...
 */
int main(int argc, char *argv[]) {
  // Initialize PIN library. Print help message if -h(elp) is specified
  // in the command line or the command line is invalid
  if (PIN_Init(argc, argv)) {
    return Usage();
  }

  // Initialize the memory reference buffer
  bufId = PIN_DefineTraceBuffer(sizeof(struct MEMREF), NUM_BUF_PAGES,
                                BufferFull, 0);

  if (bufId == BUFFER_ID_INVALID) {
    cerr << "Error: could not allocate initial buffer" << endl;
    return 1;
  }

  // Initialize thread-specific data not handled by buffering api.
  mlog_key = PIN_CreateThreadDataKey(0);

  // add an instrumentation function
  TRACE_AddInstrumentFunction(Trace, 0);

  // add callbacks
  PIN_AddThreadStartFunction(ThreadStart, 0);
  PIN_AddThreadFiniFunction(ThreadFini, 0);

  // Start the program, never returns
  PIN_StartProgram();

  return 0;
}
