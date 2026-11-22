# Builds liboperandi_cycles: Cycles plus Operandi's small C bridge, as one
# shared library with a plain C interface (operandi_cycles.h).
#
#   cmake -S crates/operandi-cycles/cycles -B <build> -DCYCLES_SOURCE_DIR=<cycles checkout>
#
# scripts/build-cycles.sh wraps this with the pinned Cycles version and the
# options Operandi uses. Cycles is Apache-2.0.

cmake_minimum_required(VERSION 3.16)
project(operandi_cycles C CXX)

if(NOT CYCLES_SOURCE_DIR)
  message(FATAL_ERROR "Set CYCLES_SOURCE_DIR to a Cycles source checkout")
endif()

# Build Cycles' own libraries as part of this project.
add_subdirectory(${CYCLES_SOURCE_DIR} cycles EXCLUDE_FROM_ALL)

# Cycles sets its compile flags per directory, so copy them from its source
# tree for the bridge to see the same configuration.
get_directory_property(_cycles_defs DIRECTORY ${CYCLES_SOURCE_DIR}/src COMPILE_DEFINITIONS)
get_directory_property(_cycles_opts DIRECTORY ${CYCLES_SOURCE_DIR}/src COMPILE_OPTIONS)
get_directory_property(_cycles_incs DIRECTORY ${CYCLES_SOURCE_DIR}/src INCLUDE_DIRECTORIES)
get_directory_property(_cycles_cxx_flags DIRECTORY ${CYCLES_SOURCE_DIR}/src DEFINITION CMAKE_CXX_FLAGS)
get_directory_property(_cycles_cxx_std DIRECTORY ${CYCLES_SOURCE_DIR}/src DEFINITION CMAKE_CXX_STANDARD)
# Cycles needs C++20. Take its setting from the src directory, else its top
# directory, and never go below 20.
if(NOT _cycles_cxx_std)
  get_directory_property(_cycles_cxx_std DIRECTORY ${CYCLES_SOURCE_DIR} DEFINITION CMAKE_CXX_STANDARD)
endif()
if(NOT _cycles_cxx_std OR _cycles_cxx_std LESS 20)
  set(_cycles_cxx_std 20)
endif()
separate_arguments(_cycles_cxx_flags)
list(FILTER _cycles_cxx_flags EXCLUDE REGEX "^[-/]std[=:]")
# On Windows Cycles leaves /std:c++17 in the global CMAKE_CXX_FLAGS (its own
# libraries override it per target); drop it here so CXX_STANDARD applies.
string(REGEX REPLACE "/std:c\\+\\+[0-9a-z]+" "" CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS}")

add_library(operandi_cycles SHARED operandi_cycles.cpp operandi_cycles.h)
target_compile_definitions(operandi_cycles PRIVATE ${_cycles_defs})
target_compile_options(operandi_cycles PRIVATE ${_cycles_cxx_flags} ${_cycles_opts})
target_include_directories(operandi_cycles PRIVATE ${CYCLES_SOURCE_DIR}/src ${_cycles_incs})
set_target_properties(operandi_cycles PROPERTIES
  CXX_STANDARD ${_cycles_cxx_std}
  CXX_STANDARD_REQUIRED ON
  CXX_VISIBILITY_PRESET hidden
  VISIBILITY_INLINES_HIDDEN ON
)

set(_cycles_libs
  cycles_session
  cycles_integrator
  cycles_scene
  cycles_device
  cycles_kernel
  cycles_bvh
  cycles_subd
  cycles_graph
  cycles_util
  extern_sky
)
# Their own third-party libraries (OpenImageIO, TBB, Embree...) come along
# through these static libraries' link interfaces.
target_link_libraries(operandi_cycles PRIVATE ${_cycles_libs})
# Cycles' Metal device is Objective-C++ and calls these system frameworks;
# its static libraries don't carry them in their link interfaces.
if(APPLE)
  target_link_libraries(operandi_cycles PRIVATE
    "-framework Foundation" "-framework Metal" "-framework IOKit"
    "-framework CoreFoundation" "-framework QuartzCore")
endif()

# Linux: find the libraries build-cycles.sh copies next to the bridge.
if(UNIX AND NOT APPLE)
  set_target_properties(operandi_cycles PROPERTIES INSTALL_RPATH "$ORIGIN")
endif()

install(TARGETS operandi_cycles LIBRARY DESTINATION lib RUNTIME DESTINATION bin ARCHIVE DESTINATION lib)
