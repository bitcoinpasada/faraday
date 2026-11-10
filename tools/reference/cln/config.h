/* The configuration ccan's crypto files ask of config.h, for a Linux
   machine with GCC or Clang; Core Lightning's configure writes the real
   one. Faraday's. */
#ifndef FARADAY_CLN_CONFIG_H
#define FARADAY_CLN_CONFIG_H
#define HAVE_BYTESWAP_H 1
#define HAVE_BSWAP_64 1
#define HAVE_LITTLE_ENDIAN 1
#define HAVE_BIG_ENDIAN 0
#define HAVE_ATTRIBUTE_UNUSED 1
#define HAVE_ATTRIBUTE_NONNULL 1
#define HAVE_ATTRIBUTE_NORETURN 1
#define HAVE_ATTRIBUTE_CONST 1
#define HAVE_ATTRIBUTE_PURE 1
#define HAVE_ATTRIBUTE_USED 1
#define HAVE_ATTRIBUTE_COLD 1
#define HAVE_BUILTIN_CONSTANT_P 1
#define HAVE_UNALIGNED_ACCESS 1
#define HAVE_WARN_UNUSED_RESULT 1
#endif
