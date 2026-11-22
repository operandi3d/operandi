/* SPDX-FileCopyrightText: 2024 Blender Authors
 * SPDX-FileCopyrightText: 2026 Operandi contributors
 *
 * SPDX-License-Identifier: GPL-2.0-or-later
 *
 * Adapted from Blender source/blender/io/usd/intern/usd_utils.cc at commit
 * e1bec63572c7092feb1af6719f8b4f76acce15fc: make_safe_identifier without
 * the Unicode option (Operandi writes ASCII names), and get_unique_path. */
#include "blender_usd.hh"

#include <pxr/usd/usd/prim.h>

namespace operandi::blender_usd {

static bool is_safe_char(unsigned char c, bool is_first)
{
  if (c == '_') {
    return true;
  }
  const bool is_letter = (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z');
  const bool is_digit = c >= '0' && c <= '9';
  return is_first ? is_letter : (is_letter || is_digit);
}

std::string make_safe_name(const std::string &name)
{
  if (name.empty()) {
    return "_";
  }
  std::string safe;
  safe.reserve(name.size() + 1);
  bool first = true;
  /* Insert a leading '_' to account for invalid starting characters. */
  if (name[0] >= '0' && name[0] <= '9') {
    safe.push_back('_');
    first = false;
  }
  for (size_t i = 0; i < name.size(); i++) {
    const unsigned char c = static_cast<unsigned char>(name[i]);
    if (c >= 0x80) {
      /* One '_' per UTF-8 code point: skip its continuation bytes. */
      safe.push_back('_');
      while (i + 1 < name.size() && (static_cast<unsigned char>(name[i + 1]) & 0xC0) == 0x80) {
        i++;
      }
    }
    else {
      safe.push_back(is_safe_char(c, first) ? char(c) : '_');
    }
    first = false;
  }
  return safe;
}

pxr::SdfPath get_unique_path(const pxr::UsdStageRefPtr &stage, const pxr::SdfPath &path)
{
  std::string unique_path = path.GetString();
  int suffix = 2;
  while (stage->GetPrimAtPath(pxr::SdfPath(unique_path)).IsValid()) {
    unique_path = path.GetString() + std::to_string(suffix++);
  }
  return pxr::SdfPath(unique_path);
}

}  // namespace operandi::blender_usd
