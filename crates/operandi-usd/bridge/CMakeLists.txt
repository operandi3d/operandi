# Builds liboperandi_usd: Operandi's small C bridge to OpenUSD, as one
# shared library with a plain C interface (operandi_usd.h).
#
#   cmake -S crates/operandi-usd/bridge -B <build> -DLIBRARY_PACK=<Blender library pack folder>
#
# scripts/build-usd.sh wraps this. OpenUSD comes prebuilt from Blender's
# library pack (the same download Cycles uses): usd/, tbb/ and python/.
# OpenUSD is TOST-1.0; blender/ is GPL-2.0-or-later (adapted from Blender).

cmake_minimum_required(VERSION 3.16)
project(operandi_usd CXX)

if(NOT LIBRARY_PACK)
  message(FATAL_ERROR "Set LIBRARY_PACK to a Blender library pack folder (e.g. lib/macos_arm64)")
endif()

set(USD_ROOT ${LIBRARY_PACK}/usd)
find_library(USD_LIBRARY NAMES usd_ms PATHS ${USD_ROOT}/lib NO_DEFAULT_PATH REQUIRED)
file(GLOB PYTHON_INCLUDE_DIR LIST_DIRECTORIES true ${LIBRARY_PACK}/python/include/python3*)

add_library(operandi_usd SHARED
  operandi_usd.cpp
  operandi_usd_read.cpp
  operandi_usd.h
  blender/blender_usd.hh
  blender/usd_utils.cc
  blender/usd_writer_camera.cc
  blender/usd_writer_light.cc
  blender/usd_writer_material.cc
  blender/usd_writer_mesh.cc
  blender/usd_writer_transform.cc
)
target_include_directories(operandi_usd PRIVATE
  ${USD_ROOT}/include
  ${LIBRARY_PACK}/tbb/include
  # OpenUSD's headers include Python's when its Python support is on, as it
  # is in Blender's build. The bridge never runs Python.
  ${PYTHON_INCLUDE_DIR}
)
# Default symbol visibility, unlike the Cycles bridge: OpenUSD compares
# value types by their C++ type info, and a TfToken made inside a
# hidden-visibility library has type info of its own that the USD library
# doesn't recognise ("Attempted to pack unsupported type 'TfToken'").
set_target_properties(operandi_usd PROPERTIES
  CXX_STANDARD 17
  CXX_STANDARD_REQUIRED ON
)
target_link_libraries(operandi_usd PRIVATE ${USD_LIBRARY})

# Blender's OpenUSD is built with Python support and leaves Python's symbols
# for the program to provide (Blender links Python in). Operandi doesn't
# have Python, and those symbols must exist when the library loads, so the
# bridge links the ones OpenUSD names from the pack's static Python. They
# are never called: nothing here starts an interpreter.
if(NOT WIN32)
  file(GLOB PYTHON_STATIC ${LIBRARY_PACK}/python/lib/libpython3*.a)
  if(PYTHON_STATIC)
    execute_process(COMMAND nm -u ${USD_LIBRARY} OUTPUT_VARIABLE _undefined)
    string(REGEX MATCHALL "[^\n]+" _undefined "${_undefined}")
    set(_keep "")
    foreach(sym ${_undefined})
      string(STRIP "${sym}" sym)
      # Linux nm prints "U name"; macOS -u prints just the name.
      string(REGEX REPLACE "^U +" "" sym "${sym}")
      if(sym MATCHES "^_?_?Py")
        if(APPLE)
          list(APPEND _keep "-Wl,-u,${sym}")
        else()
          list(APPEND _keep "-Wl,--undefined=${sym}")
        endif()
      endif()
    endforeach()
    list(LENGTH _keep _count)
    message(STATUS "Linking ${_count} Python symbols OpenUSD needs")
    target_link_libraries(operandi_usd PRIVATE ${_keep} ${PYTHON_STATIC})
    if(APPLE)
      target_link_libraries(operandi_usd PRIVATE "-framework CoreFoundation" z)
    else()
      target_link_libraries(operandi_usd PRIVATE dl util pthread m)
    endif()
  endif()
endif()

if(APPLE)
  set_target_properties(operandi_usd PROPERTIES INSTALL_RPATH "@loader_path" BUILD_WITH_INSTALL_RPATH ON)
elseif(UNIX)
  set_target_properties(operandi_usd PROPERTIES INSTALL_RPATH "$ORIGIN" BUILD_WITH_INSTALL_RPATH ON)
endif()

install(TARGETS operandi_usd LIBRARY DESTINATION lib RUNTIME DESTINATION bin ARCHIVE DESTINATION lib)
