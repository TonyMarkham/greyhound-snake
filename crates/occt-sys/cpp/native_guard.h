#pragma once
#include "greyhound_abi.h"
#include <Standard_Failure.hxx>
#include <exception>

namespace greyhound {
template <typename T, typename F>
T guard(T failure, F&& operation) noexcept {
  clear_error();
  try {
    return operation();
  } catch (const Standard_Failure& error) {
    set_error(error.what());
  } catch (const std::exception& error) {
    set_error(error.what());
  } catch (...) {
    set_error("unknown native exception");
  }
  return failure;
}
}