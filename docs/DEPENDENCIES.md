# Dependency versions and notices

Resolved by Cargo.lock on 2026-10-21.

Operandi itself is free software under the GNU General Public License,
version 3 or (at your option) any later version (`GPL-3.0-or-later`; text in
`LICENSE`). Every dependency below is under a licence compatible with that.

The implementation uses public Rust crates from crates.io. eframe/egui 0.33.3 was
selected as a cohesive API generation with wgpu 27, explicit GPU backend features,
and AccessKit. This is a deliberate pinned generation, not a claim it is the newest.
Metal is exercised on Apple M1 Max. Vulkan/DX12 portability is a design provision;
Windows/Linux and other vendors remain untested. glam supplies portable f64 math;
serde and serde_json encode explicit bounded versioned UTF-8 data.

M1.6 enables `float_roundtrip` on the existing serde_json 1.0.154 dependency to
preserve generated f64 coordinates exactly through native JSON save/reopen.
No crate version, new dependency, schema or license change accompanies this.

The main window's saved place (2026-11-07, `window_state.rs`) names
windows-sys 0.52 directly on Windows (same version and features winit already
uses) and turns on objc2-app-kit's `NSScreen` feature on macOS, to list the
screens. No new crate.

The environment picture in the 3D view (HDRI, 2026-11-12) names half 2.7.1
(MIT OR Apache-2.0) directly for its Rgba16Float texture; it was already in
the tree through exr. No new crate.

The dependency inventory below includes platform-conditional dependencies; inclusion
does not mean that every package is linked into the macOS binary. Cargo.lock pins
checksums and sources. Upstream license texts remain in the Cargo registry source
packages. A notice collection is included for the current macOS build dependencies
and fonts under docs/licenses. Re-audit for any distribution or feature change.

Keypad keys on Linux and Windows (2026-11-07): egui-winit 0.33.3 (MIT OR
Apache-2.0, Emil Ernerfeldt and the egui contributors) is vendored in
`vendor/egui-winit` with a small patch (`operandi.patch`, explained in
`OPERANDI-PATCH.md`) that reports whether a digit came from the keypad or the
main row, which upstream discards. The workspace uses it through
`[patch.crates-io]`; the version and licence are unchanged, and its MIT text
is kept beside it.

The workspace layout crate (operandi-layout, 2026-11-03) adds egui_tiles
0.14.1 for splits, tabs and drag-and-drop docking, with its dependencies
itertools, either and enumn, all MIT OR Apache-2.0. egui_tiles 0.14.1 is the
newest release that works with the pinned egui 0.33.

Backdrop items (2026-11-04) make operandi-app depend directly on image
0.25.10 (MIT OR Apache-2.0), already in the tree through arboard, with its
`png` and `jpeg` features to read reference pictures. The jpeg feature uses
zune-jpeg and zune-core (MIT OR Apache-2.0 OR Zlib), also already listed.

The Image Viewer (2026-11-07, branch image-viewer) turns on image
0.25.10's `exr`, `tiff`, `hdr`, `bmp` and `tga` features, so the Render
window can keep renders as float OpenEXR files and save and load TIFF,
Radiance HDR, Targa and BMP. That adds exr 1.74.2 and lebe 0.5.3
(BSD-3-Clause), bit_field 0.10.3 and num-complex 0.4.6 (MIT OR
Apache-2.0), zune-inflate 0.2.54 (MIT OR Apache-2.0 OR Zlib), and pulp
0.22.3, pulp-wasm-simd-flag 0.1.1, raw-cpuid 11.6.0 and reborrow 0.5.5
(MIT); tiff 0.11.3 was already in the tree.

Cycles built-in Render Outputs (2026-11-20, branch cycles-render-outputs)
make operandi-app depend directly on exr 1.74.2 (BSD-3-Clause), already in
the tree through image, pinned `=1.74.2` with default features off (no
rayon), to write OpenEXR multilayer files with Cryptomatte metadata.
operandi-cycles now uses serde_json (workspace version) to read Cycles'
Cryptomatte manifests. No new crate.

The Text tool (2026-11-05) makes operandi-core depend directly on
ttf-parser 0.25.1 (letter outlines) and epaint_default_fonts 0.33.3 (its two
shipped fonts, Ubuntu Light and Hack, already in the app through egui), both
already in the tree, and adds earcut 0.4.12 (MIT OR Apache-2.0) to fill
letters that have holes. Installed system fonts are read at run time, never
bundled.

The icon studio (2026-11-20, `icon_studio_cli.rs`) makes operandi-app name
ab_glyph 0.2.32 (Apache-2.0) directly, already in the tree through egui, to
letter its contact sheet with egui's own Ubuntu Light. No new crate.

The material converter (2026-11-11, branch blender-materials-starter) is a
new workspace crate, `operandi-preset-convert`, using only crates already
in the tree (image 0.25.10, base64 0.22.1, serde_json). Its Blender-side
exporter, `tools/blender-export/export.py`, imports Blender's `bpy`, so it
is GPL-2.0-or-later, lives in its own folder with SPDX headers and a
LICENSE.md, and is never linked into or shipped with the app; Operandi
only reads the JSON it writes. Blender itself is an external tool run by
`scripts/build-starter-pack.sh`. The starter pack's pictures are CC0 (Poly
Haven, ambientCG) and stay outside the repo.

3D mice (2026-11-09, branch spacemouse-1) add hidapi 2.6.7 (MIT; it
builds the bundled C HIDAPI library, which its authors offer under a
BSD-style licence among others) on macOS and Windows only, with its
`macos-shared-device` feature so the device is opened shared. On Windows it
pulls windows-sys 0.61.2 (MIT OR Apache-2.0). The build-time cc and
pkg-config (MIT OR Apache-2.0) were already in the tree. No 3Dconnexion SDK
or framework file is used or bundled. On macOS, libloading 0.8.9 (ISC,
already in the tree through wgpu) loads the 3DxWare client framework at
run time when the user has installed it.

One transitive dependency, block 0.1.6 (through Metal bindings), emits a future Rust
compatibility warning; the current build passes. Milestone 1 had no authored unsafe Rust or FFI. Milestone 1.1 adds one bounded macOS termination adapter, described below.

| Crate | Version | Declared license | Upstream |
|---|---|---|---|
| ab_glyph | 0.2.32 | Apache-2.0 | [source](https://github.com/alexheretic/ab-glyph) |
| ab_glyph_rasterizer | 0.1.10 | Apache-2.0 | [source](https://github.com/alexheretic/ab-glyph) |
| accesskit | 0.21.1 | MIT OR Apache-2.0 | [source](https://github.com/AccessKit/accesskit) |
| accesskit_atspi_common | 0.14.2 | MIT OR Apache-2.0 | [source](https://github.com/AccessKit/accesskit) |
| accesskit_consumer | 0.31.0 | MIT OR Apache-2.0 | [source](https://github.com/AccessKit/accesskit) |
| accesskit_macos | 0.22.2 | MIT OR Apache-2.0 | [source](https://github.com/AccessKit/accesskit) |
| accesskit_unix | 0.17.2 | MIT OR Apache-2.0 | [source](https://github.com/AccessKit/accesskit) |
| accesskit_windows | 0.29.2 | MIT OR Apache-2.0 | [source](https://github.com/AccessKit/accesskit) |
| accesskit_winit | 0.29.2 | Apache-2.0 | [source](https://github.com/AccessKit/accesskit) |
| adler2 | 2.0.1 | 0BSD OR MIT OR Apache-2.0 | [source](https://github.com/oyvindln/adler2) |
| ahash | 0.8.12 | MIT OR Apache-2.0 | [source](https://github.com/tkaitchuck/ahash) |
| android-activity | 0.6.1 | MIT OR Apache-2.0 | [source](https://github.com/rust-mobile/android-activity) |
| android-properties | 0.2.2 | MIT | [source](https://github.com/miklelappo/android-properties) |
| android_system_properties | 0.1.6 | MIT OR Apache-2.0 | [source](https://github.com/nical/android_system_properties) |
| arboard | 3.7.0 | MIT OR Apache-2.0 | [source](https://github.com/1Password/arboard) |
| arrayvec | 0.7.8 | MIT OR Apache-2.0 | [source](https://github.com/bluss/arrayvec) |
| as-raw-xcb-connection | 1.0.1 | MIT OR Apache-2.0 | [source](https://github.com/psychon/as-raw-xcb-connection) |
| ash | 0.38.0+1.3.281 | MIT OR Apache-2.0 | [source](https://github.com/ash-rs/ash) |
| async-broadcast | 0.7.2 | MIT OR Apache-2.0 | [source](https://github.com/smol-rs/async-broadcast) |
| async-channel | 2.5.0 | Apache-2.0 OR MIT | [source](https://github.com/smol-rs/async-channel) |
| async-executor | 1.14.0 | Apache-2.0 OR MIT | [source](https://github.com/smol-rs/async-executor) |
| async-io | 2.6.0 | Apache-2.0 OR MIT | [source](https://github.com/smol-rs/async-io) |
| async-lock | 3.4.2 | Apache-2.0 OR MIT | [source](https://github.com/smol-rs/async-lock) |
| async-process | 2.5.0 | Apache-2.0 OR MIT | [source](https://github.com/smol-rs/async-process) |
| async-recursion | 1.2.0 | MIT OR Apache-2.0 | [source](https://github.com/dcchut/async-recursion) |
| async-signal | 0.2.14 | Apache-2.0 OR MIT | [source](https://github.com/smol-rs/async-signal) |
| async-task | 4.7.1 | Apache-2.0 OR MIT | [source](https://github.com/smol-rs/async-task) |
| async-trait | 0.1.92 | MIT OR Apache-2.0 | [source](https://github.com/dtolnay/async-trait) |
| atomic-waker | 1.1.2 | Apache-2.0 OR MIT | [source](https://github.com/smol-rs/atomic-waker) |
| atspi | 0.25.0 | Apache-2.0 OR MIT | [source](https://github.com/odilia-app/atspi) |
| atspi-common | 0.9.0 | Apache-2.0 OR MIT | [source](https://github.com/odilia-app/atspi) |
| atspi-connection | 0.9.0 | Apache-2.0 OR MIT | [source](https://github.com/odilia-app/atspi/) |
| atspi-proxies | 0.9.0 | Apache-2.0 OR MIT | [source](https://github.com/odilia-app/atspi) |
| autocfg | 1.5.1 | Apache-2.0 OR MIT | [source](https://github.com/cuviper/autocfg) |
| bit-set | 0.8.0 | Apache-2.0 OR MIT | [source](https://github.com/contain-rs/bit-set) |
| bit-vec | 0.8.0 | Apache-2.0 OR MIT | [source](https://github.com/contain-rs/bit-vec) |
| bit_field | 0.10.3 | Apache-2.0 OR MIT | [source](https://github.com/phil-opp/rust-bit-field) |
| bitflags | 1.3.2 | MIT/Apache-2.0 | [source](https://github.com/bitflags/bitflags) |
| bitflags | 2.13.2 | MIT OR Apache-2.0 | [source](https://github.com/bitflags/bitflags) |
| block | 0.1.6 | MIT | [source](http://github.com/SSheldon/rust-block) |
| block2 | 0.5.1 | MIT | [source](https://github.com/madsmtm/objc2) |
| blocking | 1.7.0 | Apache-2.0 OR MIT | [source](https://github.com/smol-rs/blocking) |
| bumpalo | 3.20.3 | MIT OR Apache-2.0 | [source](https://github.com/fitzgen/bumpalo) |
| bytemuck | 1.25.2 | Zlib OR Apache-2.0 OR MIT | [source](https://github.com/Lokathor/bytemuck) |
| bytemuck_derive | 1.12.1 | Zlib OR Apache-2.0 OR MIT | [source](https://github.com/Lokathor/bytemuck) |
| byteorder-lite | 0.1.0 | Unlicense OR MIT | [source](https://github.com/image-rs/byteorder-lite) |
| bytes | 1.12.1 | MIT | [source](https://github.com/tokio-rs/bytes) |
| calloop | 0.13.0 | MIT | [source](https://github.com/Smithay/calloop) |
| calloop | 0.14.5 | MIT | [source](https://github.com/Smithay/calloop) |
| calloop-wayland-source | 0.3.0 | MIT | [source](https://github.com/smithay/calloop-wayland-source) |
| calloop-wayland-source | 0.4.1 | MIT | [source](https://github.com/smithay/calloop-wayland-source) |
| cc | 1.9.0 | MIT OR Apache-2.0 | [source](https://github.com/rust-lang/cc-rs) |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 | [source](https://github.com/rust-lang/cfg-if) |
| cfg_aliases | 0.2.2 | MIT | [source](https://github.com/katharostech/cfg_aliases) |
| cgl | 0.3.2 | MIT / Apache-2.0 | [source](https://github.com/servo/cgl-rs) |
| clipboard-win | 5.4.1 | BSL-1.0 | [source](https://github.com/DoumanAsh/clipboard-win) |
| codespan-reporting | 0.12.0 | Apache-2.0 | [source](https://github.com/brendanzab/codespan) |
| combine | 4.6.9 | MIT | [source](https://github.com/Marwes/combine) |
| concurrent-queue | 2.5.0 | Apache-2.0 OR MIT | [source](https://github.com/smol-rs/concurrent-queue) |
| core-foundation | 0.10.1 | MIT OR Apache-2.0 | [source](https://github.com/servo/core-foundation-rs) |
| core-foundation | 0.9.4 | MIT OR Apache-2.0 | [source](https://github.com/servo/core-foundation-rs) |
| core-foundation-sys | 0.8.7 | MIT OR Apache-2.0 | [source](https://github.com/servo/core-foundation-rs) |
| core-graphics | 0.23.2 | MIT OR Apache-2.0 | [source](https://github.com/servo/core-foundation-rs) |
| core-graphics-types | 0.1.3 | MIT OR Apache-2.0 | [source](https://github.com/servo/core-foundation-rs) |
| core-graphics-types | 0.2.0 | MIT OR Apache-2.0 | [source](https://github.com/servo/core-foundation-rs) |
| crc32fast | 1.5.3 | MIT OR Apache-2.0 | [source](https://github.com/srijs/rust-crc32fast) |
| crossbeam-utils | 0.8.23 | MIT OR Apache-2.0 | [source](https://github.com/crossbeam-rs/crossbeam) |
| crunchy | 0.2.4 | MIT | [source](https://github.com/eira-fransham/crunchy) |
| cursor-icon | 1.2.0 | MIT OR Apache-2.0 OR Zlib | [source](https://github.com/rust-windowing/cursor-icon) |
| dispatch | 0.2.0 | MIT | [source](http://github.com/SSheldon/rust-dispatch) |
| dispatch2 | 0.3.1 | Zlib OR Apache-2.0 OR MIT | [source](https://github.com/madsmtm/objc2) |
| displaydoc | 0.2.7 | MIT OR Apache-2.0 | [source](https://github.com/yaahc/displaydoc) |
| dlib | 0.5.3 | MIT | [source](https://github.com/elinorbgr/dlib) |
| document-features | 0.2.12 | MIT OR Apache-2.0 | [source](https://github.com/slint-ui/document-features) |
| downcast-rs | 1.2.1 | MIT/Apache-2.0 | [source](https://github.com/marcianx/downcast-rs) |
| dpi | 0.1.2 | Apache-2.0 AND MIT | [source](https://github.com/rust-windowing/winit) |
| ecolor | 0.33.3 | MIT OR Apache-2.0 | [source](https://github.com/emilk/egui) |
| eframe | 0.33.3 | MIT OR Apache-2.0 | [source](https://github.com/emilk/egui/tree/main/crates/eframe) |
| egui | 0.33.3 | MIT OR Apache-2.0 | [source](https://github.com/emilk/egui) |
| egui-wgpu | 0.33.3 | MIT OR Apache-2.0 | [source](https://github.com/emilk/egui/tree/main/crates/egui-wgpu) |
| egui-winit | 0.33.3 | MIT OR Apache-2.0 | [source](https://github.com/emilk/egui/tree/main/crates/egui-winit) |
| egui_glow | 0.33.3 | MIT OR Apache-2.0 | [source](https://github.com/emilk/egui/tree/main/crates/egui_glow) |
| egui_tiles | 0.14.1 | MIT OR Apache-2.0 | [source](https://github.com/rerun-io/egui_tiles) |
| either | 1.19.0 | MIT OR Apache-2.0 | [source](https://github.com/rayon-rs/either) |
| emath | 0.33.3 | MIT OR Apache-2.0 | [source](https://github.com/emilk/egui/tree/main/crates/emath) |
| endi | 1.1.1 | MIT | [source](https://github.com/zeenix/endi) |
| enumflags2 | 0.7.12 | MIT OR Apache-2.0 | [source](https://github.com/meithecatte/enumflags2) |
| enumflags2_derive | 0.7.12 | MIT OR Apache-2.0 | [source](https://github.com/meithecatte/enumflags2) |
| enumn | 0.1.14 | MIT OR Apache-2.0 | [source](https://github.com/dtolnay/enumn) |
| earcut | 0.4.12 | MIT OR Apache-2.0 | [source](https://github.com/georust/earcut) |
| epaint | 0.33.3 | MIT OR Apache-2.0 | [source](https://github.com/emilk/egui/tree/main/crates/epaint) |
| epaint_default_fonts | 0.33.3 | (MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0 | [source](https://github.com/emilk/egui/tree/main/crates/epaint_default_fonts) |
| equivalent | 1.0.3 | Apache-2.0 OR MIT | [source](https://github.com/indexmap-rs/equivalent) |
| errno | 0.3.14 | MIT OR Apache-2.0 | [source](https://github.com/lambda-fairy/rust-errno) |
| error-code | 3.4.0 | BSL-1.0 | [source](https://github.com/DoumanAsh/error-code) |
| event-listener | 5.4.2 | Apache-2.0 OR MIT | [source](https://github.com/smol-rs/event-listener) |
| event-listener-strategy | 0.5.4 | Apache-2.0 OR MIT | [source](https://github.com/smol-rs/event-listener-strategy) |
| exr | 1.74.2 | BSD-3-Clause | [source](https://github.com/johannesvollmer/exrs) |
| fastrand | 2.5.0 | Apache-2.0 OR MIT | [source](https://github.com/smol-rs/fastrand) |
| fax | 0.2.7 | MIT | [source](https://github.com/pdf-rs/fax) |
| fdeflate | 0.3.7 | MIT OR Apache-2.0 | [source](https://github.com/image-rs/fdeflate) |
| find-msvc-tools | 0.1.15 | MIT OR Apache-2.0 | [source](https://github.com/rust-lang/cc-rs) |
| flate2 | 1.1.10 | MIT OR Apache-2.0 | [source](https://github.com/rust-lang/flate2-rs) |
| foldhash | 0.1.5 | Zlib | [source](https://github.com/orlp/foldhash) |
| foldhash | 0.2.0 | Zlib | [source](https://github.com/orlp/foldhash) |
| foreign-types | 0.5.0 | MIT/Apache-2.0 | [source](https://github.com/sfackler/foreign-types) |
| foreign-types-macros | 0.2.4 | MIT/Apache-2.0 | [source](https://github.com/sfackler/foreign-types) |
| foreign-types-shared | 0.3.1 | MIT/Apache-2.0 | [source](https://github.com/sfackler/foreign-types) |
| form_urlencoded | 1.2.2 | MIT OR Apache-2.0 | [source](https://github.com/servo/rust-url) |
| futures-core | 0.3.34 | MIT OR Apache-2.0 | [source](https://github.com/rust-lang/futures-rs) |
| futures-io | 0.3.34 | MIT OR Apache-2.0 | [source](https://github.com/rust-lang/futures-rs) |
| futures-lite | 2.6.1 | Apache-2.0 OR MIT | [source](https://github.com/smol-rs/futures-lite) |
| futures-macro | 0.3.34 | MIT OR Apache-2.0 | [source](https://github.com/rust-lang/futures-rs) |
| futures-task | 0.3.34 | MIT OR Apache-2.0 | [source](https://github.com/rust-lang/futures-rs) |
| futures-util | 0.3.34 | MIT OR Apache-2.0 | [source](https://github.com/rust-lang/futures-rs) |
| gethostname | 1.1.0 | Apache-2.0 | [source](https://codeberg.org/swsnr/gethostname.rs.git) |
| getrandom | 0.3.4 | MIT OR Apache-2.0 | [source](https://github.com/rust-random/getrandom) |
| getrandom | 0.4.3 | MIT OR Apache-2.0 | [source](https://github.com/rust-random/getrandom) |
| gl_generator | 0.14.0 | Apache-2.0 | [source](https://github.com/brendanzab/gl-rs/) |
| glam | 0.30.10 | MIT OR Apache-2.0 | [source](https://github.com/bitshifter/glam-rs) |
| glow | 0.16.0 | MIT OR Apache-2.0 OR Zlib | [source](https://github.com/grovesNL/glow) |
| glutin | 0.32.3 | Apache-2.0 | [source](https://github.com/rust-windowing/glutin) |
| glutin-winit | 0.5.0 | MIT | [source](https://github.com/rust-windowing/glutin) |
| glutin_egl_sys | 0.7.1 | Apache-2.0 | [source](https://github.com/rust-windowing/glutin) |
| glutin_glx_sys | 0.6.1 | Apache-2.0 | [source](https://github.com/rust-windowing/glutin) |
| glutin_wgl_sys | 0.6.1 | Apache-2.0 | [source](https://github.com/rust-windowing/glutin) |
| gpu-alloc | 0.6.2 | MIT OR Apache-2.0 | [source](https://github.com/zakarumych/gpu-alloc) |
| gpu-alloc-types | 0.3.1 | MIT OR Apache-2.0 | [source](https://github.com/zakarumych/gpu-alloc) |
| gpu-allocator | 0.27.0 | MIT OR Apache-2.0 | [source](https://github.com/Traverse-Research/gpu-allocator) |
| gpu-descriptor | 0.3.2 | MIT OR Apache-2.0 | [source](https://github.com/zakarumych/gpu-descriptor) |
| gpu-descriptor-types | 0.2.0 | MIT OR Apache-2.0 | [source](https://github.com/zakarumych/gpu-descriptor) |
| half | 2.7.1 | MIT OR Apache-2.0 | [source](https://github.com/VoidStarKat/half-rs) |
| hashbrown | 0.15.5 | MIT OR Apache-2.0 | [source](https://github.com/rust-lang/hashbrown) |
| hashbrown | 0.16.1 | MIT OR Apache-2.0 | [source](https://github.com/rust-lang/hashbrown) |
| hashbrown | 0.17.1 | MIT OR Apache-2.0 | [source](https://github.com/rust-lang/hashbrown) |
| hermit-abi | 0.5.3 | MIT OR Apache-2.0 | [source](https://github.com/hermit-os/hermit-rs) |
| hex | 0.4.3 | MIT OR Apache-2.0 | [source](https://github.com/KokaKiwi/rust-hex) |
| hexf-parse | 0.2.1 | CC0-1.0 | [source](https://github.com/lifthrasiir/hexf) |
| hidapi | 2.6.7 | MIT | [source](https://github.com/ruabmbua/hidapi-rs) |
| icu_collections | 2.3.0 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| icu_locale_core | 2.3.0 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| icu_normalizer | 2.3.0 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| icu_normalizer_data | 2.3.0 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| icu_properties | 2.3.0 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| icu_properties_data | 2.3.0 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| icu_provider | 2.3.1 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| idna | 1.1.0 | MIT OR Apache-2.0 | [source](https://github.com/servo/rust-url/) |
| idna_adapter | 1.2.2 | Apache-2.0 OR MIT | [source](https://github.com/hsivonen/idna_adapter) |
| image | 0.25.10 | MIT OR Apache-2.0 | [source](https://github.com/image-rs/image) |
| indexmap | 2.14.2 | Apache-2.0 OR MIT | [source](https://github.com/indexmap-rs/indexmap) |
| itertools | 0.14.0 | MIT OR Apache-2.0 | [source](https://github.com/rust-itertools/itertools) |
| itoa | 1.0.18 | MIT OR Apache-2.0 | [source](https://github.com/dtolnay/itoa) |
| jni | 0.22.4 | MIT OR Apache-2.0 | [source](https://github.com/jni-rs/jni-rs) |
| jni-macros | 0.22.4 | MIT OR Apache-2.0 | [source](https://github.com/jni-rs/jni-rs) |
| jni-sys | 0.3.1 | MIT OR Apache-2.0 | [source](https://github.com/jni-rs/jni-sys) |
| jni-sys | 0.4.1 | MIT OR Apache-2.0 | [source](https://github.com/jni-rs/jni-sys) |
| jni-sys-macros | 0.4.1 | MIT OR Apache-2.0 | [source](https://github.com/jni-rs/jni-sys) |
| jobserver | 0.1.35 | MIT OR Apache-2.0 | [source](https://github.com/rust-lang/jobserver-rs) |
| js-sys | 0.3.106 | MIT OR Apache-2.0 | [source](https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/js-sys) |
| khronos-egl | 6.0.0 | MIT/Apache-2.0 | [source](https://github.com/timothee-haudebourg/khronos-egl) |
| khronos_api | 3.1.0 | Apache-2.0 | [source](https://github.com/brendanzab/gl-rs/) |
| lebe | 0.5.3 | BSD-3-Clause | [source](https://github.com/johannesvollmer/lebe) |
| libc | 0.2.191 | MIT OR Apache-2.0 | [source](https://github.com/rust-lang/libc) |
| libloading | 0.8.9 | ISC | [source](https://github.com/nagisa/rust_libloading/) |
| libm | 0.2.16 | MIT | [source](https://github.com/rust-lang/compiler-builtins) |
| libredox | 0.1.25 | MIT | [source](https://gitlab.redox-os.org/redox-os/libredox.git) |
| linux-raw-sys | 0.12.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | [source](https://github.com/sunfishcode/linux-raw-sys) |
| linux-raw-sys | 0.4.15 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | [source](https://github.com/sunfishcode/linux-raw-sys) |
| litemap | 0.8.3 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| litrs | 1.0.1 | MIT OR Apache-2.0 | [source](https://github.com/LukasKalbertodt/litrs) |
| lock_api | 0.4.14 | MIT OR Apache-2.0 | [source](https://github.com/Amanieu/parking_lot) |
| log | 0.4.34 | MIT OR Apache-2.0 | [source](https://github.com/rust-lang/log) |
| malloc_buf | 0.0.6 | MIT | [source](https://github.com/SSheldon/malloc_buf) |
| memchr | 2.8.3 | Unlicense OR MIT | [source](https://github.com/BurntSushi/memchr) |
| memmap2 | 0.9.12 | MIT OR Apache-2.0 | [source](https://github.com/RazrFalcon/memmap2-rs) |
| memoffset | 0.9.1 | MIT | [source](https://github.com/Gilnaa/memoffset) |
| metal | 0.32.0 | MIT OR Apache-2.0 | [source](https://github.com/gfx-rs/metal-rs) |
| miniz_oxide | 0.8.9 | MIT OR Zlib OR Apache-2.0 | [source](https://github.com/Frommi/miniz_oxide/tree/master/miniz_oxide) |
| miniz_oxide | 0.9.1 | MIT OR Zlib OR Apache-2.0 | [source](https://github.com/Frommi/miniz_oxide/tree/master/miniz_oxide) |
| moxcms | 0.8.1 | BSD-3-Clause OR Apache-2.0 | [source](https://github.com/awxkee/moxcms.git) |
| naga | 27.0.3 | MIT OR Apache-2.0 | [source](https://github.com/gfx-rs/wgpu) |
| ndk | 0.9.0 | MIT OR Apache-2.0 | [source](https://github.com/rust-mobile/ndk) |
| ndk-context | 0.1.1 | MIT OR Apache-2.0 | [source](https://github.com/rust-windowing/android-ndk-rs) |
| ndk-sys | 0.6.0+11769913 | MIT OR Apache-2.0 | [source](https://github.com/rust-mobile/ndk) |
| nohash-hasher | 0.2.0 | Apache-2.0 OR MIT | [source](https://github.com/paritytech/nohash-hasher) |
| num-complex | 0.4.6 | MIT OR Apache-2.0 | [source](https://github.com/rust-num/num-complex) |
| num-traits | 0.2.19 | MIT OR Apache-2.0 | [source](https://github.com/rust-num/num-traits) |
| num_enum | 0.7.6 | BSD-3-Clause OR MIT OR Apache-2.0 | [source](https://github.com/illicitonion/num_enum) |
| num_enum_derive | 0.7.6 | BSD-3-Clause OR MIT OR Apache-2.0 | [source](https://github.com/illicitonion/num_enum) |
| objc | 0.2.7 | MIT | [source](http://github.com/SSheldon/rust-objc) |
| objc-sys | 0.3.5 | MIT | [source](https://github.com/madsmtm/objc2) |
| objc2 | 0.5.3 | MIT | [source](https://github.com/madsmtm/objc2) |
| objc2 | 0.6.5 | MIT | [source](https://github.com/madsmtm/objc2) |
| objc2-app-kit | 0.2.2 | MIT | [source](https://github.com/madsmtm/objc2) |
| objc2-app-kit | 0.3.2 | Zlib OR Apache-2.0 OR MIT | [source](https://github.com/madsmtm/objc2) |
| objc2-cloud-kit | 0.2.2 | MIT | [source](https://github.com/madsmtm/objc2) |
| objc2-contacts | 0.2.2 | MIT | [source](https://github.com/madsmtm/objc2) |
| objc2-core-data | 0.2.2 | MIT | [source](https://github.com/madsmtm/objc2) |
| objc2-core-foundation | 0.3.2 | Zlib OR Apache-2.0 OR MIT | [source](https://github.com/madsmtm/objc2) |
| objc2-core-graphics | 0.3.2 | Zlib OR Apache-2.0 OR MIT | [source](https://github.com/madsmtm/objc2) |
| objc2-core-image | 0.2.2 | MIT | [source](https://github.com/madsmtm/objc2) |
| objc2-core-location | 0.2.2 | MIT | [source](https://github.com/madsmtm/objc2) |
| objc2-encode | 4.1.0 | MIT | [source](https://github.com/madsmtm/objc2) |
| objc2-foundation | 0.2.2 | MIT | [source](https://github.com/madsmtm/objc2) |
| objc2-foundation | 0.3.2 | MIT | [source](https://github.com/madsmtm/objc2) |
| objc2-io-surface | 0.3.2 | Zlib OR Apache-2.0 OR MIT | [source](https://github.com/madsmtm/objc2) |
| objc2-link-presentation | 0.2.2 | MIT | [source](https://github.com/madsmtm/objc2) |
| objc2-metal | 0.2.2 | MIT | [source](https://github.com/madsmtm/objc2) |
| objc2-quartz-core | 0.2.2 | MIT | [source](https://github.com/madsmtm/objc2) |
| objc2-symbols | 0.2.2 | MIT | [source](https://github.com/madsmtm/objc2) |
| objc2-ui-kit | 0.2.2 | MIT | [source](https://github.com/madsmtm/objc2) |
| objc2-uniform-type-identifiers | 0.2.2 | MIT | [source](https://github.com/madsmtm/objc2) |
| objc2-user-notifications | 0.2.2 | MIT | [source](https://github.com/madsmtm/objc2) |
| once_cell | 1.21.4 | MIT OR Apache-2.0 | [source](https://github.com/matklad/once_cell) |
| orbclient | 0.3.55 | MIT | [source](https://gitlab.redox-os.org/redox-os/orbclient) |
| ordered-float | 5.5.0 | MIT | [source](https://github.com/reem/rust-ordered-float) |
| ordered-stream | 0.2.0 | MIT OR Apache-2.0 | [source](https://github.com/danieldg/ordered-stream) |
| owned_ttf_parser | 0.25.1 | Apache-2.0 | [source](https://github.com/alexheretic/owned-ttf-parser) |
| parking | 2.2.1 | Apache-2.0 OR MIT | [source](https://github.com/smol-rs/parking) |
| parking_lot | 0.12.5 | MIT OR Apache-2.0 | [source](https://github.com/Amanieu/parking_lot) |
| parking_lot_core | 0.9.12 | MIT OR Apache-2.0 | [source](https://github.com/Amanieu/parking_lot) |
| paste | 1.0.15 | MIT OR Apache-2.0 | [source](https://github.com/dtolnay/paste) |
| percent-encoding | 2.3.2 | MIT OR Apache-2.0 | [source](https://github.com/servo/rust-url/) |
| pin-project | 1.1.13 | Apache-2.0 OR MIT | [source](https://github.com/taiki-e/pin-project) |
| pin-project-internal | 1.1.13 | Apache-2.0 OR MIT | [source](https://github.com/taiki-e/pin-project) |
| pin-project-lite | 0.2.17 | Apache-2.0 OR MIT | [source](https://github.com/taiki-e/pin-project-lite) |
| piper | 0.2.5 | MIT OR Apache-2.0 | [source](https://github.com/smol-rs/piper) |
| pkg-config | 0.3.34 | MIT OR Apache-2.0 | [source](https://github.com/rust-lang/pkg-config-rs) |
| plain | 0.2.3 | MIT/Apache-2.0 | [source](https://github.com/randomites/plain) |
| png | 0.18.1 | MIT OR Apache-2.0 | [source](https://github.com/image-rs/image-png) |
| polling | 3.11.0 | Apache-2.0 OR MIT | [source](https://github.com/smol-rs/polling) |
| pollster | 0.4.0 | Apache-2.0/MIT | [source](https://github.com/zesterer/pollster) |
| portable-atomic | 1.15.0 | Apache-2.0 OR MIT | [source](https://github.com/taiki-e/portable-atomic) |
| portable-atomic-util | 0.2.8 | Apache-2.0 OR MIT | [source](https://github.com/taiki-e/portable-atomic-util) |
| potential_utf | 0.1.6 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| presser | 0.3.1 | MIT OR Apache-2.0 | [source](https://github.com/EmbarkStudios/presser) |
| proc-macro-crate | 3.5.0 | MIT OR Apache-2.0 | [source](https://github.com/bkchr/proc-macro-crate) |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 | [source](https://github.com/dtolnay/proc-macro2) |
| profiling | 1.0.18 | MIT OR Apache-2.0 | [source](https://github.com/aclysma/profiling) |
| pulp | 0.22.3 | MIT | [source](https://github.com/sarah-quinones/pulp) |
| pulp-wasm-simd-flag | 0.1.1 | MIT | [source](https://github.com/sarah-quinones/pulp) |
| pxfm | 0.1.30 | BSD-3-Clause OR Apache-2.0 | [source](https://github.com/awxkee/pxfm) |
| quick-error | 2.0.1 | MIT/Apache-2.0 | [source](http://github.com/tailhook/quick-error) |
| quick-xml | 0.41.0 | MIT | [source](https://github.com/tafia/quick-xml) |
| quote | 1.0.47 | MIT OR Apache-2.0 | [source](https://github.com/dtolnay/quote) |
| r-efi | 5.3.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | [source](https://github.com/r-efi/r-efi) |
| r-efi | 6.0.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | [source](https://github.com/r-efi/r-efi) |
| range-alloc | 0.1.5 | MIT OR Apache-2.0 | [source](https://github.com/gfx-rs/range-alloc) |
| raw-cpuid | 11.6.0 | MIT | [source](https://github.com/gz/rust-cpuid) |
| raw-window-handle | 0.6.2 | MIT OR Apache-2.0 OR Zlib | [source](https://github.com/rust-windowing/raw-window-handle) |
| reborrow | 0.5.5 | MIT | [source](https://github.com/sarah-ek/reborrow) |
| redox_syscall | 0.4.1 | MIT | [source](https://gitlab.redox-os.org/redox-os/syscall) |
| redox_syscall | 0.5.18 | MIT | [source](https://gitlab.redox-os.org/redox-os/syscall) |
| redox_syscall | 0.9.5 | MIT | [source](https://gitlab.redox-os.org/redox-os/kernel) |
| renderdoc-sys | 1.1.0 | MIT OR Apache-2.0 | [source](https://github.com/ebkalderon/renderdoc-rs) |
| rustc-hash | 1.1.0 | Apache-2.0/MIT | [source](https://github.com/rust-lang-nursery/rustc-hash) |
| rustc-hash | 2.1.3 | Apache-2.0 OR MIT | [source](https://github.com/rust-lang/rustc-hash) |
| rustc_version | 0.4.1 | MIT OR Apache-2.0 | [source](https://github.com/djc/rustc-version-rs) |
| rustix | 0.38.44 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | [source](https://github.com/bytecodealliance/rustix) |
| rustix | 1.1.5 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | [source](https://github.com/bytecodealliance/rustix) |
| rustversion | 1.0.23 | MIT OR Apache-2.0 | [source](https://github.com/dtolnay/rustversion) |
| same-file | 1.0.6 | Unlicense/MIT | [source](https://github.com/BurntSushi/same-file) |
| scoped-tls | 1.0.1 | MIT/Apache-2.0 | [source](https://github.com/alexcrichton/scoped-tls) |
| scopeguard | 1.2.0 | MIT OR Apache-2.0 | [source](https://github.com/bluss/scopeguard) |
| semver | 1.0.28 | MIT OR Apache-2.0 | [source](https://github.com/dtolnay/semver) |
| serde | 1.0.229 | MIT OR Apache-2.0 | [source](https://github.com/serde-rs/serde) |
| serde_core | 1.0.229 | MIT OR Apache-2.0 | [source](https://github.com/serde-rs/serde) |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 | [source](https://github.com/serde-rs/serde) |
| serde_json | 1.0.154 | MIT OR Apache-2.0 | [source](https://github.com/serde-rs/json) |
| serde_repr | 0.1.21 | MIT OR Apache-2.0 | [source](https://github.com/dtolnay/serde-repr) |
| shlex | 2.0.1 | MIT OR Apache-2.0 | [source](https://github.com/comex/rust-shlex) |
| signal-hook-registry | 1.4.8 | MIT OR Apache-2.0 | [source](https://github.com/vorner/signal-hook) |
| simd-adler32 | 0.3.10 | MIT | [source](https://github.com/mcountryman/simd-adler32) |
| simd_cesu8 | 1.2.0 | Apache-2.0 OR MIT | [source](https://github.com/seancroach/simd_cesu8) |
| simdutf8 | 0.1.5 | MIT OR Apache-2.0 | [source](https://github.com/rusticstuff/simdutf8) |
| slab | 0.4.12 | MIT | [source](https://github.com/tokio-rs/slab) |
| slotmap | 1.1.1 | Zlib | [source](https://github.com/orlp/slotmap) |
| smallvec | 1.16.3 | MIT OR Apache-2.0 | [source](https://github.com/servo/rust-smallvec) |
| smithay-client-toolkit | 0.19.2 | MIT | [source](https://github.com/smithay/client-toolkit) |
| smithay-client-toolkit | 0.20.0 | MIT | [source](https://github.com/smithay/client-toolkit) |
| smithay-clipboard | 0.7.3 | MIT | [source](https://github.com/smithay/smithay-clipboard) |
| smol_str | 0.2.2 | MIT OR Apache-2.0 | [source](https://github.com/rust-analyzer/smol_str) |
| spirv | 0.3.0+sdk-1.3.268.0 | Apache-2.0 | [source](https://github.com/gfx-rs/rspirv) |
| stable_deref_trait | 1.2.1 | MIT OR Apache-2.0 | [source](https://github.com/storyyeller/stable_deref_trait) |
| static_assertions | 1.1.0 | MIT OR Apache-2.0 | [source](https://github.com/nvzqz/static-assertions-rs) |
| syn | 2.0.119 | MIT OR Apache-2.0 | [source](https://github.com/dtolnay/syn) |
| syn | 3.0.9 | MIT OR Apache-2.0 | [source](https://github.com/dtolnay/syn) |
| synstructure | 0.14.0 | MIT | [source](https://github.com/mystor/synstructure) |
| tempfile | 3.27.0 | MIT OR Apache-2.0 | [source](https://github.com/Stebalien/tempfile) |
| termcolor | 1.4.1 | Unlicense OR MIT | [source](https://github.com/BurntSushi/termcolor) |
| thiserror | 1.0.69 | MIT OR Apache-2.0 | [source](https://github.com/dtolnay/thiserror) |
| thiserror | 2.0.21 | MIT OR Apache-2.0 | [source](https://github.com/dtolnay/thiserror) |
| thiserror-impl | 1.0.69 | MIT OR Apache-2.0 | [source](https://github.com/dtolnay/thiserror) |
| thiserror-impl | 2.0.21 | MIT OR Apache-2.0 | [source](https://github.com/dtolnay/thiserror) |
| tiff | 0.11.3 | MIT | [source](https://github.com/image-rs/image-tiff) |
| tinystr | 0.8.4 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| tokio | 1.54.0 | MIT | [source](https://github.com/tokio-rs/tokio) |
| toml_datetime | 1.1.2+spec-1.1.0 | MIT OR Apache-2.0 | [source](https://github.com/toml-rs/toml) |
| toml_edit | 0.25.17+spec-1.1.0 | MIT OR Apache-2.0 | [source](https://github.com/toml-rs/toml) |
| toml_parser | 1.1.5+spec-1.1.0 | MIT OR Apache-2.0 | [source](https://github.com/toml-rs/toml) |
| tracing | 0.1.44 | MIT | [source](https://github.com/tokio-rs/tracing) |
| tracing-attributes | 0.1.31 | MIT | [source](https://github.com/tokio-rs/tracing) |
| tracing-core | 0.1.36 | MIT | [source](https://github.com/tokio-rs/tracing) |
| ttf-parser | 0.25.1 | MIT OR Apache-2.0 | [source](https://github.com/harfbuzz/ttf-parser) |
| type-map | 0.5.1 | MIT/Apache-2.0 | [source](https://github.com/kardeiz/type-map) |
| uds_windows | 1.2.1 | MIT | [source](https://github.com/haraldh/rust_uds_windows) |
| unicode-ident | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 | [source](https://github.com/dtolnay/unicode-ident) |
| unicode-segmentation | 1.13.3 | MIT OR Apache-2.0 | [source](https://github.com/unicode-rs/unicode-segmentation) |
| unicode-width | 0.2.2 | MIT OR Apache-2.0 | [source](https://github.com/unicode-rs/unicode-width) |
| url | 2.5.8 | MIT OR Apache-2.0 | [source](https://github.com/servo/rust-url) |
| utf8_iter | 1.0.4 | Apache-2.0 OR MIT | [source](https://github.com/hsivonen/utf8_iter) |
| uuid | 1.28.0 | Apache-2.0 OR MIT | [source](https://github.com/uuid-rs/uuid) |
| version_check | 0.9.5 | MIT/Apache-2.0 | [source](https://github.com/SergioBenitez/version_check) |
| walkdir | 2.5.0 | Unlicense/MIT | [source](https://github.com/BurntSushi/walkdir) |
| wasip2 | 1.0.4+wasi-0.2.12 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | [source](https://github.com/bytecodealliance/wasi-rs) |
| wasm-bindgen | 0.2.129 | MIT OR Apache-2.0 | [source](https://github.com/wasm-bindgen/wasm-bindgen) |
| wasm-bindgen-futures | 0.4.79 | MIT OR Apache-2.0 | [source](https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/futures) |
| wasm-bindgen-macro | 0.2.129 | MIT OR Apache-2.0 | [source](https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/macro) |
| wasm-bindgen-macro-support | 0.2.129 | MIT OR Apache-2.0 | [source](https://github.com/wasm-bindgen/wasm-bindgen/tree/main/crates/macro-support) |
| wasm-bindgen-shared | 0.2.129 | MIT OR Apache-2.0 | [source](https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/shared) |
| wayland-backend | 0.3.17 | MIT | [source](https://github.com/smithay/wayland-rs) |
| wayland-client | 0.31.15 | MIT | [source](https://github.com/smithay/wayland-rs) |
| wayland-csd-frame | 0.3.0 | MIT | [source](https://github.com/rust-windowing/wayland-csd-frame) |
| wayland-cursor | 0.31.14 | MIT | [source](https://github.com/smithay/wayland-rs) |
| wayland-protocols | 0.32.13 | MIT | [source](https://github.com/smithay/wayland-rs) |
| wayland-protocols-experimental | 20250721.0.1 | MIT | [source](https://github.com/smithay/wayland-rs) |
| wayland-protocols-misc | 0.3.12 | MIT | [source](https://github.com/smithay/wayland-rs) |
| wayland-protocols-plasma | 0.3.12 | MIT | [source](https://github.com/smithay/wayland-rs) |
| wayland-protocols-wlr | 0.3.12 | MIT | [source](https://github.com/smithay/wayland-rs) |
| wayland-scanner | 0.31.11 | MIT | [source](https://github.com/smithay/wayland-rs) |
| wayland-sys | 0.31.11 | MIT | [source](https://github.com/smithay/wayland-rs) |
| web-sys | 0.3.106 | MIT OR Apache-2.0 | [source](https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/web-sys) |
| web-time | 1.1.0 | MIT OR Apache-2.0 | [source](https://github.com/daxpedda/web-time) |
| webbrowser | 1.2.4 | MIT OR Apache-2.0 | [source](https://github.com/amodm/webbrowser-rs) |
| weezl | 0.1.12 | MIT OR Apache-2.0 | [source](https://github.com/image-rs/weezl) |
| wgpu | 27.0.1 | MIT OR Apache-2.0 | [source](https://github.com/gfx-rs/wgpu) |
| wgpu-core | 27.0.3 | MIT OR Apache-2.0 | [source](https://github.com/gfx-rs/wgpu) |
| wgpu-core-deps-apple | 27.0.0 | MIT OR Apache-2.0 | [source](https://github.com/gfx-rs/wgpu) |
| wgpu-core-deps-emscripten | 27.0.0 | MIT OR Apache-2.0 | [source](https://github.com/gfx-rs/wgpu) |
| wgpu-core-deps-windows-linux-android | 27.0.0 | MIT OR Apache-2.0 | [source](https://github.com/gfx-rs/wgpu) |
| wgpu-hal | 27.0.4 | MIT OR Apache-2.0 | [source](https://github.com/gfx-rs/wgpu) |
| wgpu-types | 27.0.1 | MIT OR Apache-2.0 | [source](https://github.com/gfx-rs/wgpu) |
| winapi-util | 0.1.11 | Unlicense OR MIT | [source](https://github.com/BurntSushi/winapi-util) |
| windows | 0.58.0 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows | 0.61.3 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-collections | 0.2.0 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-core | 0.58.0 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-core | 0.61.2 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-future | 0.2.1 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-implement | 0.58.0 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-implement | 0.60.2 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-interface | 0.58.0 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-interface | 0.59.3 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-link | 0.1.3 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-link | 0.2.1 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-numerics | 0.2.0 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-result | 0.2.0 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-result | 0.3.4 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-strings | 0.1.0 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-strings | 0.4.2 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-sys | 0.52.0 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-sys | 0.59.0 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-targets | 0.52.6 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows-threading | 0.1.0 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows_aarch64_gnullvm | 0.52.6 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows_aarch64_msvc | 0.52.6 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows_i686_gnu | 0.52.6 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows_i686_gnullvm | 0.52.6 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows_i686_msvc | 0.52.6 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows_x86_64_gnu | 0.52.6 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows_x86_64_gnullvm | 0.52.6 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| windows_x86_64_msvc | 0.52.6 | MIT OR Apache-2.0 | [source](https://github.com/microsoft/windows-rs) |
| winit | 0.30.13 | Apache-2.0 | [source](https://github.com/rust-windowing/winit) |
| winnow | 1.0.4 | MIT | [source](https://github.com/winnow-rs/winnow) |
| wit-bindgen | 0.57.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | [source](https://github.com/bytecodealliance/wit-bindgen) |
| writeable | 0.6.4 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| x11-dl | 2.21.0 | MIT | [source](https://github.com/AltF02/x11-rs.git) |
| x11rb | 0.13.2 | MIT OR Apache-2.0 | [source](https://github.com/psychon/x11rb) |
| x11rb-protocol | 0.13.2 | MIT OR Apache-2.0 | [source](https://github.com/psychon/x11rb) |
| xcursor | 0.3.11 | MIT | [source](https://github.com/esposm03/xcursor-rs) |
| xkbcommon-dl | 0.4.2 | MIT | [source](https://github.com/rust-windowing/xkbcommon-dl) |
| xkeysym | 0.2.1 | MIT OR Apache-2.0 OR Zlib | [source](https://github.com/notgull/xkeysym) |
| xml-rs | 0.8.29 | MIT | [source](https://github.com/kornelski/xml-rs) |
| yoke | 0.8.3 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| yoke-derive | 0.8.4 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| zbus | 5.19.0 | MIT | [source](https://github.com/z-galaxy/zbus/) |
| zbus-lockstep | 0.5.2 | MIT | [source](https://github.com/luukvanderduim/zbus-lockstep) |
| zbus-lockstep-macros | 0.5.2 | MIT | [source](https://github.com/luukvanderduim/zbus-lockstep) |
| zbus_macros | 5.19.0 | MIT | [source](https://github.com/z-galaxy/zbus/) |
| zbus_names | 4.3.5 | MIT | [source](https://github.com/z-galaxy/zbus/) |
| zbus_xml | 5.2.1 | MIT | [source](https://github.com/z-galaxy/zbus/) |
| zcheapstr | 1.1.0 | MIT | [source](https://github.com/z-galaxy/zcheapstr/) |
| zerocopy | 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT | [source](https://github.com/google/zerocopy) |
| zerocopy-derive | 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT | [source](https://github.com/google/zerocopy) |
| zerofrom | 0.1.8 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| zerofrom-derive | 0.1.8 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| zerotrie | 0.2.5 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| zerovec | 0.11.8 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| zerovec-derive | 0.11.6 | Unicode-3.0 | [source](https://github.com/unicode-org/icu4x) |
| zlib-rs | 0.6.8 | Zlib | [source](https://github.com/trifectatechfoundation/zlib-rs) |
| zmij | 1.0.23 | MIT | [source](https://github.com/dtolnay/zmij) |
| zune-core | 0.5.3 | MIT OR Apache-2.0 OR Zlib | [source](https://github.com/etemesi254/zune-image) |
| zune-inflate | 0.2.54 | MIT OR Apache-2.0 OR Zlib | [source](https://github.com/etemesi254/zune-image) |
| zune-jpeg | 0.5.15 | MIT OR Apache-2.0 OR Zlib | [source](https://github.com/etemesi254/zune-image/tree/dev/crates/zune-jpeg) |
| zvariant | 5.15.1 | MIT | [source](https://github.com/z-galaxy/zbus/) |
| zvariant_derive | 5.15.1 | MIT | [source](https://github.com/z-galaxy/zbus/) |
| zvariant_utils | 4.2.0 | MIT | [source](https://github.com/z-galaxy/zbus/) |

## Milestone 1.1 native integration

The native Quit bypass in the existing framework was a concrete blocker for
M11-001/025. macOS-only direct declarations reuse already resolved MIT packages
objc2 0.5.3, objc2-app-kit 0.2.2 and objc2-foundation 0.2.2; no new framework or
resolved crate version was introduced. Public package source and its AppKit
types were consulted for this integration, not any modeling application.

native.rs replaces NSApplication's terminate: implementation once on the main
thread. Its matching Objective-C ABI callback only posts an atomic quit intent
and requests repaint; normal eframe root-window Close occurs after the shared
document decision. Safety/lifetime/signature invariants are at the unsafe block.
Command-Q was exercised through native UI. Other platforms compile the no-op
platform adapter and retain their window-close route; they remain unvalidated.

The optional qa feature uses the existing egui/wgpu stack without additional
dependencies. It cannot access a reference application. It exports original
application renders and exact framebuffer metadata to ignored development output.


Milestone 1.1 font selection: the macOS adapter reads the installed system sans
font at runtime (`/System/Library/Fonts/SFNS.ttf`) when available, with egui's
bundled licensed fonts as fallback. No Apple font file is copied into this
repository or the bundle. Numeric property text uses the bundled monospace face.
The original editable vector icons are authored in `theme.rs`; no extracted or
traced proprietary assets are included.

## Dictionary SVG rendering — 2026-10-21

The shared icon renderer uses resvg 0.48.1 with all default features disabled.
Only bundled admitted SVG paths/gradients are rendered; no fonts, external
images, runtime network loading or system-font scanning is needed. This keeps
editable SVG artwork intact and avoids a bespoke, incomplete SVG interpreter.
Raster sizes follow drawable scale and textures are cached per egui context.
The new dependency closure below is from locked cargo metadata; license texts
are retained under `docs/licenses/<crate>-<version>/`. No project license is chosen.

| Crate | Version | Declared license | Source |
|---|---|---|---|
| arrayref | 0.3.9 | BSD-2-Clause | [upstream](https://github.com/droundy/arrayref) |
| data-url | 0.3.2 | MIT OR Apache-2.0 | [upstream](https://github.com/servo/rust-url) |
| euclid | 0.22.14 | MIT OR Apache-2.0 | [upstream](https://github.com/servo/euclid) |
| float-cmp | 0.9.0 | MIT | [upstream](https://github.com/mikedilger/float-cmp) |
| imagesize | 0.15.0 | MIT | [upstream](https://github.com/Roughsketch/imagesize) |
| kurbo | 0.13.1 | Apache-2.0 OR MIT | [upstream](https://github.com/linebender/kurbo) |
| pico-args | 0.5.0 | MIT | [upstream](https://github.com/RazrFalcon/pico-args) |
| polycool | 0.4.0 | MIT OR Apache-2.0 | [upstream](https://github.com/linebender/kurbo) |
| resvg | 0.48.1 | Apache-2.0 OR MIT | [upstream](https://github.com/linebender/resvg) |
| rgb | 0.8.53 | MIT | [upstream](https://github.com/kornelski/rust-rgb) |
| roxmltree | 0.21.1 | MIT OR Apache-2.0 | [upstream](https://github.com/RazrFalcon/roxmltree) |
| simplecss | 0.2.2 | Apache-2.0 OR MIT | [upstream](https://github.com/linebender/simplecss) |
| siphasher | 1.0.4 | MIT OR Apache-2.0 | [upstream](https://github.com/jedisct1/rust-siphash) |
| strict-num | 0.1.1 | MIT | [upstream](https://github.com/RazrFalcon/strict-num) |
| svgtypes | 0.16.1 | Apache-2.0 OR MIT | [upstream](https://github.com/linebender/svgtypes) |
| tiny-skia | 0.12.0 | BSD-3-Clause | [upstream](https://github.com/linebender/tiny-skia) |
| tiny-skia-path | 0.12.0 | BSD-3-Clause | [upstream](https://github.com/linebender/tiny-skia/tree/master/path) |
| usvg | 0.48.1 | Apache-2.0 OR MIT | [upstream](https://github.com/linebender/resvg) |

## M1.5 native numeric-key adapter — 2026-10-21

No crate version or license was changed. The existing objc2-app-kit 0.2.2
explicit feature list now includes NSEvent. Public AppKit virtual key codes were
read from the installed macOS SDK HIToolbox Events.h. No reference-product code
or internals were used. native.rs observes process-local NSApplication sendEvent:
only to retain main/keypad numeric location before egui-winit merges it. Its
bounded queue copies digit/location/press state only; no native event/text/borrow
is retained or persisted. Every event is forwarded once to the original method.
ABI, main-thread, process lifetime, and no-unwind invariants are documented at the
unsafe calls. Unknown/overflow physical provenance cannot invoke numeric
modeling defaults. Focus/menu/modal rules still own their keys. Safe conversion
and simulated routing tests cover main/keypad separation; native main 1 and
keypad non-alias behavior were exercised. Other OS adapters remain unimplemented
and unvalidated; no Windows/Linux claim is made.

## Typography correction — 2026-10-21

Existing locked `ttf-parser0.25.1` (MIT OR Apache-2.0; notices above) is now a direct
app dependency solely to inspect installed face metadata. No new resolved crate,
version or rasterizer. macOS chrome reads `/System/Library/Fonts/LucidaGrande.ttc`
at runtime and selects Regular. The Apple font is never embedded/copied/bundled.
Explicit missing-font fallback uses existing egui Ubuntu-Light and its notices.
Other platforms remain unvalidated. See [TYPOGRAPHY.md](archive/TYPOGRAPHY.md).

2026-10-22 native-menu follow-up: enable the existing objc2-app-kit0.2.2
NSMenu/NSMenuItem/NSCell features. No new crate/version/license or lockfile change.
Main-thread-only public AppKit adapter safety/lifetime invariants and actual
checks are recorded in [MACOS_MENUS.md](archive/MACOS_MENUS.md).

## Standalone icon generator — 2026-11-01

`tools/operandi-icons` uses Python packages only for SVG authoring, review
renders, and automated artwork validation. No Rust dependency or application
file changes. `requirements.lock.txt` in that directory records the complete
tested package closure; license identifiers below were read from installed
package metadata/license files. Production SVGs embed no libraries or fonts.

| Package | Version | License | Purpose |
|---|---|---|---|
| svgwrite | 1.4.3 | MIT | SVG serialization |
| CairoSVG | 2.8.2 | LGPL-3.0-or-later | Preview and bounds rendering |
| Pillow | 12.0.0 | MIT-CMU | PNG sheets and pixel inspection |
| cairocffi | 1.7.1 | BSD-3-Clause | Cairo binding |
| cffi | 2.1.1 | MIT-0 | Foreign function binding |
| cssselect2 | 0.10.1 | BSD-3-Clause | Renderer dependency |
| defusedxml | 0.7.1 | PSF-2.0 | Renderer XML parsing |
| pycparser | 3.12 | BSD-3-Clause | cffi dependency |
| tinycss2 | 1.5.1 | BSD-3-Clause | Renderer CSS parsing |
| webencodings | 0.6.1 | BSD-3-Clause | Renderer encoding dependency |

Cairo is a system prerequisite for raster previews, used through cairocffi;
it is not vendored or added to application distribution. Package license texts
remain in the installed environment. The svgwrite-only mode does not need the
rendering stack. The six reference SVGs are Jason's approved original artwork,
copied unchanged for side-by-side review; no external application's art is used.

## Project manifest (2026-11-01)

`operandi-core` adds `toml` 0.8.23 (MIT OR Apache-2.0) to read and write
`operandi.project.toml` (docs/PROJECT_SYSTEM.md). It brings `toml_edit`
0.22.27, `toml_datetime` 0.6.11, `serde_spanned` 0.6.9, `toml_write` 0.1.2
(all MIT OR Apache-2.0) and `winnow` 0.7.15 (MIT). Exact versions are pinned in
Cargo.lock.

## Native file panels (2026-11-01)

`operandi-app` adds `rfd` 0.17.2 (MIT) for the native Open/Save panels and
folder picker (macOS NSOpenPanel/NSSavePanel; native dialogs on Windows and
Linux too, untested there). On macOS it adds only `block2` 0.6.2 (MIT); the
other AppKit bindings were already in use. Exact versions are in Cargo.lock.

`operandi-core` adds three crates for interchange import/export (2026-11-02):
`gltf` 1.4.1 (MIT OR Apache-2.0; with `gltf-json`/`gltf-derive` 1.4.1, same
license) for reading glTF/GLB, built without its `import` feature so no image
decoders are linked (its `extras` feature reads material extras; no new crates); `stl_io` 0.11.0 (MIT) for reading STL and writing binary
STL; and `base64` 0.22.1 (MIT OR Apache-2.0) for glTF data URIs. OBJ, ASCII
STL, glTF writing, VideoScape GEO and SVG are written in Operandi's own code.
Exact versions are in Cargo.lock.

FBX, Alembic and PLY (2026-11-11, branch fbx-abc-ply). Licences checked
upstream that day; all are permissive and compatible with GPL v3.

- FBX import: `ufbx` 0.11.5 (MIT OR Unlicense, Samuli Raivio;
  github.com/ufbx/ufbx and github.com/ufbx/ufbx-rust). The crate bundles
  ufbx.c and compiles it with `cc`, already in the tree. ufbx only reads,
  so FBX is import-only.
- Alembic import and export: the new `operandi-alembic` crate vendors
  Alembic 1.8.12 (BSD-3-Clause, Lucasfilm and Sony Pictures Imageworks;
  github.com/alembic/alembic) and Imath 3.2.3 (BSD-3-Clause, Contributors to
  the OpenEXR Project; github.com/AcademySoftwareFoundation/Imath) under
  `crates/operandi-alembic/vendor`, each with its LICENSE file beside it.
  Only Alembic's Util, AbcCoreAbstract, Ogawa, AbcCoreOgawa, Abc and AbcGeom
  folders are kept, unchanged except a hand-written `Util/Config.h` in place
  of the CMake template; Imath's `ImathConfig.h` is the one its CMake
  configure step writes. HDF5 archives are not supported, so no HDF5. The
  sources and Operandi's C shim (`shim/`, GPL-3.0-or-later) are compiled
  with `cc` 1.x; no CMake needed. Binary releases must carry both BSD
  notices.
- PLY is written in Operandi's own code; no crate.

TextureCraft pieces (2026-11-17, branch claude/project-thread-rccgyy).
Code adapted from TextureCraft (github.com/storytold/texturecraft at
9c1b9cbc) and craft-libs (github.com/storytold/craft-libs), both MIT OR
Apache-2.0, Copyright (c) 2026 ArtCraft Team and their contributors; MIT
texts in `docs/licenses/texturecraft-LICENSE-MIT.txt` and
`docs/licenses/craft-libs-LICENSE-MIT.txt`, credited at the top of each
file. No ArtCraft names or logos are used.

- `operandi-core/src/bake/padding.rs` (UV padding and pull-push fill, from
  `crates/texel/src/dilate.rs` and `crates/bake/src/map/diffusion.rs`),
  `bake/curvature.rs` (from `crates/bake/src/map/curvature.rs`),
  `uv_atlas.rs` (from `crates/geom/src/unwrap.rs`) and
  `interchange/threemf.rs` (structure of craft-libs `crates/mesh/src/threemf.rs`).
- `operandi-core/src/time_tick.rs` (exact time: `Tick`, `FrameRate`), from
  craft-libs `crates/time/src/lib.rs` at 42bb9d35, cut to the Tick and
  FrameRate part (phase 0 doors, 2026-11-17).
- FBX export (`interchange/fbx_write.rs`) is Operandi's own; the header and
  footer constants are the ones Blender's exporter writes (numbers only).
- New direct core dependencies, already in the tree: `flate2` 1.1
  (MIT OR Apache-2.0) and `crc32fast` 1.5 (MIT OR Apache-2.0) for 3MF's ZIP
  packages, `png` 0.18 (MIT OR Apache-2.0) for bake output and padding.
- `parked/texture-painting/` holds TextureCraft's `paint`, `texel` and
  `compose` crates unchanged with their LICENSE-MIT, LICENSE-APACHE and
  NOTICE. They are not built or linked (see its README).

## Blender sculpt brush maths (2026-11-11)

`crates/operandi-core/src/sculpt_blender/` is adapted from Blender's sculpt
brushes (`source/blender/editors/sculpt_paint/mesh/brushes/`, `sculpt.cc`,
`blenkernel/intern/brush.cc` and `kelvinlet.cc`) at commit
b4eed6fd498e76a7e93404bac2326e9f84deba22 (<https://projects.blender.org/blender/blender>).
Copyright Blender Authors, **GPL-2.0-or-later**; each file keeps Blender's
SPDX notice and says what it was adapted from, and the licence text is
`LICENSES/GPL-2.0-or-later.txt`. Thank you to the Blender sculpt developers.
Only the per-brush maths is ported (falloff presets, hardness, square tips,
brush-plane sampling, the brush frame, Kelvinlets and each brush's vertex
rule); Blender's UI, PBVH, undo and stroke code are not. Contributions to
this module are taken in under GPL-2.0-or-later (CONTRIBUTING.md). No new
crates.

## Viewport matcap (2026-11-09)

The Matcap display style uses matcaps from Blender: Blender matcaps,
CC0; thank you to the Blender community. Eight of them (clay_studio,
basic_grey, pearl, resin, ceramic_lightbulb, metal_carpaint, fullmetal,
check_reflection_horizontal), 256×256 PNG in
`crates/operandi-app/assets/studiolights/` (about 0.5 MB, built into the
app); Blender's licence note is in `docs/licenses/blender-studiolights/`.
No new crates.

## Viewport materials: Blender's principled surface (2026-11-11)

The Advanced viewport style lights materials with an approximation of
Cycles' principled surface adapted from **Blender**'s GPU material shaders
(GPL-2.0-or-later, copyright Blender Authors): the layer order and weights
of `gpu_shader_material_principled.bsl.hh` and
`gpu_shader_material_open_pbr_util.bsl.hh`, the F82-tint metal and
multiple-scattering terms of EEVEE's `eevee_nodetree_lib.bsl.hh`, and the
bump and normal-map nodes (`gpu_shader_material_bump.bsl.hh`,
`gpu_shader_material_normal_map.bsl.hh`), from
`source/blender/gpu/shaders/material` and
`source/blender/draw/engines/eevee/shaders` at commit
b4eed6fd498e76a7e93404bac2326e9f84deba22. Rewritten as WGSL and Rust in
one file, `crates/operandi-core/src/blender_principled.rs`, which carries
Blender's notice and the GPL-2.0-or-later SPDX header; the GPL v2 text is
in `docs/licenses/blender-gpu-shaders/`. The reflection table is
Operandi's own integration. Thank you to the Blender developers. No new
crates.

## Cycles renderer

Operandi renders with **Cycles**, the path tracer made by the Blender
Foundation and the Cycles contributors as part of the Blender project
(<https://www.cycles-renderer.org>). Thank you to everyone who built it.
Cycles is Apache-2.0 (copyright Blender Authors / Blender Foundation), used
at commit `3eb35f85996baa6c1b2e19bf297b22745c85e321` (2026-10-26, 5.3.0
alpha), pinned in `scripts/build-cycles.sh`, with Operandi's changes in
`crates/operandi-cycles/cycles/patches/` (each file says what it changes
and why; Apache-2.0 §4(b): changed files carry an "Operandi:" comment).
Today that is one: caustics-only lights (Direct Caustics, 2026-11-13). `crates/operandi-cycles`
links it only when built with `--features cycles`; default builds contain
none of it. Its licence, contributor list and the licences of the parts
inside it (MikkTSpace, sky model, OSL shader code) are in
`docs/licenses/cycles/`.

The viewport's drawing of node graph layers (MaterialX stage 5,
2026-11-20) rewrites some of Cycles' kernel sums in Operandi's own
source, in every build: `operandi-core/src/cycles_noise.rs` (hashes,
Perlin noise and the Noise, White Noise, Checker, Gradient and Wave
textures; the Perlin part is BSD-3-Clause, from Open Shading Language via
Cycles' `noise.h`), `shader_func.rs` (Math, Vector Math, colour blends,
Map Range, Mapping, Fresnel...) and their WGSL twin `shader_func.wgsl`,
and `cycles_voronoi.rs`/`.wgsl` (the Voronoi texture, MIT after Inigo
Quilez, plus Apache-2.0 hashes).
Apache-2.0, BSD-3-Clause and MIT are compatible with GPL v3; each file carries
the SPDX notices, and `docs/licenses/cycles/README.md` lists what came
from where. No new crate.

The Preview's kept renderer (`OcSession` in `operandi_cycles.cpp`) follows
the design of Blender's own Cycles sync, `intern/cycles/blender` in the
Blender source (Apache-2.0): build each piece once under a key, reuse it,
delete what the next scene no longer uses, and pass meshes in their own
space with corner normals. The code is Operandi's own; nothing was copied.

Linux and Windows release builds (`scripts/build-release.sh`) take the same
libraries from the Linux and Windows packs, next to the program (`usr/lib/` in
the AppImage, beside `Operandi.exe` on Windows). Those packs can differ in
versions and in which libraries are shared rather than static, so re-check
the table below against the first Linux and Windows builds. The release
builds also ship:

- the AppImage type2 runtime (MIT, AppImage project,
  https://github.com/AppImage/type2-runtime), the start of the Linux file;
- the Microsoft Visual C++ runtime DLLs (`msvcp140.dll`, `vcruntime140*.dll`
  and friends) from Visual Studio's `Redist` folder, which Microsoft's
  Visual Studio licence lets applications ship.

Cycles brings these C/C++ libraries from Blender's precompiled library pack.
A Cycles build of the app ships them in `Contents/Frameworks/` (or built into
one of those files). Licence texts, exactly as published at each version, are
in `docs/licenses/native/`. The `image` crate (already listed) is a
dev-dependency of operandi-cycles for its example renderer.

| Library | Version | Licence | Ships as / inside | Upstream |
|---|---|---|---|---|
| OpenImageIO | 3.1.16.0 | Apache-2.0 | libOpenImageIO, libOpenImageIO_Util | [source](https://github.com/AcademySoftwareFoundation/OpenImageIO) |
| OpenImageDenoise | 2.5.0 | Apache-2.0 | libOpenImageDenoise* | [source](https://github.com/RenderKit/oidn) |
| Embree | 4.4.1 | Apache-2.0 | libembree4 | [source](https://github.com/RenderKit/embree) |
| oneTBB | 2022.3.0 | Apache-2.0 | libtbb | [source](https://github.com/uxlfoundation/oneTBB) |
| OpenEXR | 3.4.10 | BSD-3-Clause | libOpenEXR, libOpenEXRCore, libIex, libIlmThread | [source](https://github.com/AcademySoftwareFoundation/openexr) |
| Imath | 3.2.2 | BSD-3-Clause | libImath | [source](https://github.com/AcademySoftwareFoundation/Imath) |
| OpenColorIO | 2.5.0 | BSD-3-Clause | libOpenColorIO | [source](https://github.com/AcademySoftwareFoundation/OpenColorIO) |
| OpenJPH | 0.25.2 | BSD-2-Clause | libopenjph | [source](https://github.com/aous72/OpenJPH) |
| libtiff | 4.7.1 | libtiff (BSD-style) | inside OpenImageIO | [source](https://gitlab.com/libtiff/libtiff) |
| libpng | 1.6.58 | libpng-2.0 | inside OpenImageIO | [source](https://github.com/pnggroup/libpng) |
| libjpeg-turbo | 2.1.3 | IJG AND BSD-3-Clause AND Zlib | inside OpenImageIO | [source](https://github.com/libjpeg-turbo/libjpeg-turbo) |
| libwebp | 1.6.0 | BSD-3-Clause (+ patent grant) | inside OpenImageIO | [source](https://github.com/webmproject/libwebp) |
| OpenJPEG | 2.5.3 | BSD-2-Clause | inside OpenImageIO | [source](https://github.com/uclouvain/openjpeg) |
| pugixml | 1.10 | MIT | inside OpenImageIO | [source](https://github.com/zeux/pugixml) |
| fmt | 12.1.0 | MIT | inside OpenImageIO | [source](https://github.com/fmtlib/fmt) |
| robin-map | 1.3.0 | MIT | inside OpenImageIO | [source](https://github.com/Tessil/robin-map) |
| libdeflate | 1.18 | MIT | inside OpenImageIO, OpenEXRCore | [source](https://github.com/ebiggers/libdeflate) |
| zlib | 1.3.1 | Zlib | inside OpenImageIO, OpenColorIO | [source](https://zlib.net) |
| zstd | 1.5.7 | BSD-3-Clause | inside OpenImageIO, OpenColorIO, liboperandi_cycles | [source](https://github.com/facebook/zstd) |
| yaml-cpp | 0.8.0 | MIT | inside OpenColorIO | [source](https://github.com/jbeder/yaml-cpp) |
| Expat | 2.7.5 | MIT | inside OpenColorIO | [source](https://github.com/libexpat/libexpat) |
| minizip-ng | 4.2.2 | Zlib | inside OpenColorIO | [source](https://github.com/zlib-ng/minizip-ng) |
| pystring | 1.1.3 | BSD-3-Clause | inside OpenColorIO | [source](https://github.com/imageworks/pystring) |

Versions come from the pack's `deps.md`; "inside" entries were confirmed by
the version strings in the built libraries (2026-11-05). OpenImageIO's own
list of code it borrows is in `docs/licenses/native/openimageio-3.1.16.0/THIRD-PARTY.md`.
Re-check this table whenever `build-cycles.sh` changes its pinned commit.

## USD export (2026-11-11, `crates/operandi-usd`)

USD files are written by **OpenUSD** (Pixar Animation Studios and the
OpenUSD contributors, <https://openusd.org>), version 26.08, taken prebuilt
from Blender's library pack (the same download as Cycles). Thank you to
everyone who built it. OpenUSD is licensed under the **Tomorrow Open Source
Technology License 1.0 (TOST-1.0)**: the Apache License 2.0 with section 6
(Trademarks) reworded to cover the licensor's affiliates too. That section
only withholds trademark rights, which GPL v3 section 7(e) allows, so
TOST-1.0 is compatible with Operandi's GPL-3.0-or-later as Apache-2.0 is
(checked upstream at the v26.08 tag, 2026-11-11; text in
`docs/licenses/native/openusd-26.08/`, which also lists the code OpenUSD
bundles, with Pixar's `NOTICE.txt` beside it). Both travel in every app
bundle's `Contents/Resources/licenses/native/openusd-26.08/`; Operandi uses
no Pixar or OpenUSD branding. `crates/operandi-usd` links it only with
`--features usd`, which macOS QA builds turn on by default since USD import
(2026-11-18); plain `cargo` builds contain none of it.

`crates/operandi-usd/bridge/blender/` is adapted from Blender's USD exporter
(`source/blender/io/usd/intern/usd_writer_mesh.cc`, `usd_writer_material.cc`,
`usd_writer_camera.cc`, `usd_writer_light.cc`, `usd_light_convert.cc`,
`usd_writer_transform.cc`, `usd_utils.cc` and their headers) at commit
`e1bec63572c7092feb1af6719f8b4f76acce15fc`
(<https://projects.blender.org/blender/blender>). Copyright Blender Authors,
**GPL-2.0-or-later**; each file keeps Blender's notice and SPDX line and says
what it was adapted from, and the GPL v2 text is
`LICENSES/GPL-2.0-or-later.txt`. Thank you to the Blender USD developers.
Changes to those files come in under GPL-2.0-or-later (CONTRIBUTING.md).

Blender's OpenUSD is built with Python support and leaves Python's symbols to
the program. The bridge links the 178 that OpenUSD names from the pack's
static CPython 3.13.13 (PSF-2.0, GPL-compatible; text in
`docs/licenses/native/python-3.13.13/`), so the library loads; no Python code
runs.

A USD build of the app ships these in `Contents/Frameworks/` beside the
bridge (`liboperandi_usd`), plus OpenUSD's plugin descriptions in
`Contents/Resources/usd/`. Libraries already listed under Cycles (oneTBB,
Imath, OpenEXR, OpenImageIO, OpenColorIO, OpenJPH) are the same files.

| Library | Version | Licence | Ships as / inside | Upstream |
|---|---|---|---|---|
| OpenUSD | 26.08 | TOST-1.0 | libusd_ms | [source](https://github.com/PixarAnimationStudios/OpenUSD) |
| OpenSubdiv | 3.7.0 | TOST-1.0 | libosdCPU, libosdGPU | [source](https://github.com/PixarAnimationStudios/OpenSubdiv) |
| MaterialX | 1.39.4 | Apache-2.0 | libMaterialX* | [source](https://github.com/AcademySoftwareFoundation/MaterialX) |
| OpenVDB | 13.0.0 | Apache-2.0 | libopenvdb | [source](https://github.com/AcademySoftwareFoundation/openvdb) |
| CPython | 3.13.13 | PSF-2.0 | inside liboperandi_usd (symbols only) | [source](https://github.com/python/cpython) |

## Booleans (2026-11-11, `operandi-core/src/boolean.rs`)

| Crate | Version | Licence | Use |
|---|---|---|---|
| manifold-csg (+ manifold-csg-sys) | 0.4.1 (3.5.105) | Apache-2.0 OR MIT | Rust bindings to the Manifold engine |
| Manifold (C++, built by manifold-csg-sys) | v3.5.3 | Apache-2.0 | the boolean engine; linked statically |
| Clipper2 (inside Manifold) | Manifold's builtin copy | BSL-1.0 | 2D polygon clipping used by Manifold |
| boolmesh | 0.2.2 | MPL-2.0 | second-opinion engine (every boolean is cross-checked) |
| robust | 1.2.0 | MIT OR Apache-2.0 | exact orientation tests in the pre-flight check |

boolmesh pulls in a second glam (0.32), MIT OR Apache-2.0. MPL-2.0 is
file-level copyleft and compatible with GPL-3.0-or-later; boolmesh is
used unmodified from crates.io.

Build note: manifold-csg-sys fetches Manifold's source with `git clone`
(pinned tag v3.5.3) into its build folder on the first build and compiles it
with CMake, so a fresh machine needs network, git, CMake and a C++ compiler
for that one build. Manifold's thread pool (TBB) is turned off
(`default-features = false`) to keep the C++ build plain on Windows and
Linux. `MANIFOLD_CSG_LIB_DIR` points the build at a prebuilt library
instead. Windows (mingw) and Linux (zigbuild) cross builds with Manifold
haven't been tried yet.

UV unwrapping and packing (2026-11-11, branch claude/project-thread-iwegbz)
adds the `operandi-uv` crate: a port of Blender's UV parametrizer, packer
and SLIM solver at Blender commit b4eed6fd498e76a7e93404bac2326e9f84deba22
(Blender Authors, GPL-2.0-or-later; SLIM core by Michael Rabinovich and
Alec Jacobson, MPL-2.0, text in `LICENSES/MPL-2.0.txt`). Each ported file
keeps its notice and names its source; `crates/operandi-uv/README.md` lists
them. Its sparse solves use faer 0.24.4 (MIT, Sarah El Kazdadi) with
`default-features = false, features = ["std"]`, which brings in
faer-traits, dyn-stack, equator, gemm, nano-gemm, private-gemm-x86, pulp,
qd, reborrow, num-complex, num-traits, bytemuck, libm, sysctl, raw-cpuid
and their macro crates, all MIT or MIT OR Apache-2.0 (byteorder Unlicense
OR MIT).

## Remesh back ends (2026-11-11)

`crates/operandi-remesh` builds QuadriFlow and its bundled C++ libraries with the `cc`
build crate (already in Cargo.lock; no new crates.io crate) into one static
library, and wraps them in safe Rust. Licences were checked upstream on
2026-11-11; each file keeps its own notice, and the crate's `README.md`
lists every Operandi change.

| Code | From | Licence | Used for |
|---|---|---|---|
| QuadriFlow (Jingwei Huang, Yichao Zhou, Matthias Niessner, Jonathan Shewchuk, Leonidas Guibas, 2018) | Blender `extern/quadriflow` at e1bec635 (upstream github.com/hjwdzh/QuadriFlow 27a6867 plus Blender's patch, which drops Boost) | BSD-3-Clause, with an optional grant-back clause for shared enhancements (Blender's README labels it MIT; the text is BSD-3-Clause) | Automatic Retopology (`mesh.retopo`) |
| LEMON 1.3.1 graph library (EGRES) | bundled with QuadriFlow | BSL-1.0 | QuadriFlow's flow solvers |
| pcg32 (Wenzel Jakob, after Melissa O'Neill) | bundled with QuadriFlow | Apache-2.0 | QuadriFlow's random start |
| Parallel stable sort (Intel, 2014) | bundled with QuadriFlow | BSD-3-Clause | QuadriFlow |
| Boykov-Kolmogorov max flow (Stephan Diederich 2006, Blender Authors 2024) | Blender `extern/quadriflow/patches` | MIT (adapted from Boost.Graph) | QuadriFlow without Boost |
| QuadriFlow C API (Blender Authors, 2019) | Blender `intern/quadriflow/quadriflow_capi.cpp` | GPL-2.0-or-later | Operandi's glue, adapted |
| Eigen 3.4.0 (headers only) | gitlab.com/libeigen/eigen, tarball sha256 8586084f… | MPL-2.0; built with `EIGEN_MPL2_ONLY` so no LGPL part is used | QuadriFlow's linear algebra |

QuadriFlow's optional "aggressive SAT" pass runs an external `minisat`
program; Operandi never turns it on. Voxel Remesh (`sculpt.remesh`) and
Blocks / Smooth / Sharp (`operandi.remesh`) are Operandi's own Rust code (`crates/operandi-core/src/remesh_voxel.rs`) and
uses no outside code; the exact orientation tests come from the `robust`
crate already listed. Thank you to the QuadriFlow authors and the Blender
developers.

## MaterialX and OpenPBR (2026-11-11, `operandi-core/src/materialx`, `openpbr.rs`)

Operandi reads and writes `.mtlx` with its own Rust code; no MaterialX
library is linked. It bundles, unmodified, the definitions its importer
needs, compiled in with `include_str!` from
`crates/operandi-core/materialx/` (licence texts beside them and in
`docs/licenses/materialx-stdlib-1.39.4` and `docs/licenses/openpbr-1.1.2`,
which the app bundles copy to `Resources/licenses`).

| Source | Version | Licence | Files |
|---|---|---|---|
| MaterialX standard library | 1.39.4 | Apache-2.0 | `standard_surface.mtlx`, `gltf_pbr.mtlx`, `usd_preview_surface.mtlx`, `standard_surface_to_open_pbr.mtlx`; `stdlib_defs.mtlx` (2026-11-20: standard nodedef names for USD networks, and tests' check of the tree library), `stdlib_ng.mtlx` (2026-11-20: MaterialX's nodegraph implementations, so imported materials using those nodes render) |
| MaterialX example materials | 1.39.4 | Apache-2.0 | `fixtures/materialx-examples/` (test fixtures only, the `.mtlx` files of `resources/Materials/Examples`, unmodified) |
| OpenPBR Surface reference | 1.1.2 (commit 51b14a8a) | Apache-2.0 | `open_pbr_surface.mtlx` |
| roxmltree (crate) | 0.21.1 | MIT OR Apache-2.0 | XML reading (already in the tree through the app; now also a core dependency) |

## Free memory (2026-11-12, `operandi-app/src/free_memory.rs`)

| Crate | Version | Licence | Use |
|---|---|---|---|
| sysinfo | 0.40.3 (default features off, `system` on) | MIT | Available memory for Subdivide's check before a big result (macOS, Windows, Linux) |

## lxoxide (2026-11-14, `operandi-core/src/preset/modo.rs`)

**lxoxide** (<https://github.com/rondreas/lxoxide>) at commit ea7ede6,
Copyright (c) 2026 Andreas Rånman, **MIT OR Apache-2.0**, used under MIT
(`LICENSES/MIT-lxoxide.txt`; also in `NOTICE`). Thank you, Andreas. Jason
approved its use on 2026-11-14. Adapted: the `PRVW` preview header layout
(u16 width, u16 height, u32 pixel format, u32 flags; flags 1 = PNG) from
its `src/meta.rs` `Preview` record; the attribution is at the top of
`modo.rs`. Its list of known chunk ids was also used as a checklist so the
LXO readers skip unused records with a note. Not a crate dependency; its
docs, tests and fixtures are not used. No new crates.

## Curve fitting from VectorCraft (2026-11-17)

`crates/operandi-core/src/curve_fit.rs` is adapted from VectorCraft
(`crates/pathops/src/fit.rs` at commit
36b5a5920c50d0818d1b3994527f5349b3484dbe; Copyright (c) 2026 ArtCraft Team
and the VectorCraft contributors; MIT OR Apache-2.0, used here under MIT).
The least-squares cubic fit was moved from 2D kurbo types to 3D glam
points; the MIT notice is kept at the top of the file. It fits hand-drawn
path strokes (the path tools' Sketch mode). No new crate.

## 3D painting stage 1 (2026-11-17, branch painting-stage1)

Only builds with the app's `painting` feature link these; default
(release, tester) builds don't.

- `crates/operandi-paint` (ours, GPL-3.0-or-later): paintable images.
  Uses `image` 0.25.10 (already in the build).
- `vendor/craft-psd`: craft-libs `crates/psd` at
  d6b5b16bbb1448193e246ff640adf7ab7ae45037
  (<https://github.com/storytold/craft-libs>, from PhotoCraft), sources
  unchanged; Copyright (c) 2026 ArtCraft Team and the craft-libs
  contributors, **MIT OR Apache-2.0** (its `LICENSE-MIT`, `LICENSE-APACHE`
  and `NOTICE` beside it; see `vendor/craft-psd/OPERANDI.md`). Lossless
  PSD/PSB reading and writing.
- New crates through craft-psd: `encoding_rs` 0.8.44 ((Apache-2.0 OR MIT)
  AND BSD-3-Clause), `rayon` 1.12.0, `rayon-core` 1.13.0,
  `crossbeam-deque` 0.8.8, `crossbeam-epoch` 0.9.21 (MIT OR Apache-2.0).
  `core_detect` 1.0.0 and `multiversion_no_op` 1.0.0 are in Cargo.lock as
  optional dependencies of `encoding_rs` that aren't built.
  `thiserror` 2 and `flate2` were already in the build.
- `parked/texture-painting/` is still parked: stage 1 has no brushes, so
  nothing from it was taken yet (`bake/padding.rs`, already adapted from
  its `dilate.rs`, does Expand UV Seams).

Painting stage 4: `paint_tips.rs` adapts TextureCraft's `pcg3d` hash
(`crates/texel/src/noise.rs` at 9c1b9cbc, MIT OR Apache-2.0, credited at the
top of the file). The Text tip draws with `ttf-parser` and
`epaint_default_fonts`, already in core. No new crates.

Utah Teapot data (2026-11-21, branch missing-tools-2): the 32 Bezier patches
of the 1987 rounded-bottom version (Frank Crow, after Martin Newell and Jim
Blinn), from graphics.cs.utah.edu/teapot, stored in Operandi's own text
format as `crates/operandi-core/src/utah_teapot.patches`. Terms: freely
available for any use, including commercial use, credited as a "Utah
Teapot"; the credit is in that file's header and the About window. Data
only, no code.
