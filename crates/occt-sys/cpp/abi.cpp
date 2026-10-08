#include "greyhound_abi.h"
#include <cstdio>

namespace {
thread_local char error_text[1024]{};
}

void greyhound::clear_error() noexcept {
  error_text[0] = '\0';
}

void greyhound::set_error(const char* message) noexcept {
  std::snprintf(error_text, sizeof(error_text), "%s", message ? message : "native operation failed");
}

extern "C" uint32_t greyhound_abi_version() noexcept {
  return 3;
}

extern "C" const char* greyhound_last_error() noexcept {
  return error_text;
}