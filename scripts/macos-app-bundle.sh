# Shared by scripts/bundle-dev.sh (QA apps) and scripts/release-macos.sh
# (signed releases): puts a built operandi-app into a macOS .app folder.
# Source this file, then call
#
#   assemble_app <app folder> <name> <bundle id> <version> <build number> \
#                <scratch folder> [<Cycles install folder>]
#
# from the repo root. The app binary comes from target/release (or
# target/$PROFILE when PROFILE is set). With a Cycles install folder
# (scripts/build-cycles.sh's development/cycles/install), the Cycles
# libraries go into Contents/Frameworks and its kernel sources into
# Contents/Resources/cycles. With USD_INSTALL set (scripts/build-usd.sh's
# development/usd/install), the USD bridge, OpenUSD and its libraries go
# into Contents/Frameworks too, and OpenUSD's plugin descriptions into
# Contents/Resources/usd. With PYTHON_INSTALL set (scripts/fetch-python.sh's
# development/python/python), libpython goes into Contents/Frameworks and
# the standard library into Contents/Resources/python (Python scripting;
# embed.rs finds it there). Nothing here signs: each caller signs the way
# it needs to (ad hoc for QA, Developer ID for releases).

assemble_app() {
    app="$1"
    name="$2"
    bundle_id="$3"
    version="$4"
    build_number="$5"
    scratch="$6"
    cycles_install="${7:-}"
    plist="$app/Contents/Info.plist"

    mkdir -p "$app/Contents/MacOS"
    cp "target/${PROFILE:-release}/operandi-app" "$app/Contents/MacOS/operandi-app"
    cat > "$plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>operandi-app</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>NSHighResolutionCapable</key><true/>
<key>LSMinimumSystemVersion</key><string>12.0</string>
</dict></plist>
PLIST
    /usr/libexec/PlistBuddy -c "Add :CFBundleIdentifier string $bundle_id" "$plist"
    /usr/libexec/PlistBuddy -c "Add :CFBundleName string $name" "$plist"
    /usr/libexec/PlistBuddy -c "Add :CFBundleDisplayName string $name" "$plist"
    /usr/libexec/PlistBuddy -c "Add :CFBundleVersion string $build_number" "$plist"
    /usr/libexec/PlistBuddy -c "Add :CFBundleShortVersionString string $version" "$plist"

    # Generate platform containers only from the admitted square raster master
    # (the logo Dock tile).
    # sips scales uniformly; no crop, recolor or alpha/background substitution.
    iconset="$scratch/Operandi.iconset"
    mkdir -p "$iconset" "$app/Contents/Resources"
    for logical in 16 32 128 256 512; do
        sips -z "$logical" "$logical" assets/branding/operandi-dock-icon.png --out "$iconset/icon_${logical}x${logical}.png" >/dev/null
        physical=$((logical * 2))
        sips -z "$physical" "$physical" assets/branding/operandi-dock-icon.png --out "$iconset/icon_${logical}x${logical}@2x.png" >/dev/null
    done
    iconutil -c icns "$iconset" -o "$app/Contents/Resources/Operandi.icns"
    cp assets/branding/operandi-logo.png "$app/Contents/Resources/operandi-logo.png"
    cp assets/branding/branding-manifest.json "$app/Contents/Resources/branding-manifest.json"
    /usr/libexec/PlistBuddy -c 'Add :CFBundleIconFile string Operandi.icns' "$plist"

    # .opscene files belong to Operandi: the document type and its exported
    # identifier (assets/macos/opscene-document-type.plist), and the
    # document icon Finder shows for them (the O on a dark page, made by
    # tools/document-icon/make-document-icon.swift).
    dociconset="$scratch/OperandiScene.iconset"
    mkdir -p "$dociconset"
    for logical in 16 32 128 256 512; do
        sips -z "$logical" "$logical" assets/branding/operandi-document-icon.png --out "$dociconset/icon_${logical}x${logical}.png" >/dev/null
        physical=$((logical * 2))
        sips -z "$physical" "$physical" assets/branding/operandi-document-icon.png --out "$dociconset/icon_${logical}x${logical}@2x.png" >/dev/null
    done
    iconutil -c icns "$dociconset" -o "$app/Contents/Resources/OperandiScene.icns"
    /usr/libexec/PlistBuddy -c "Merge assets/macos/opscene-document-type.plist" "$plist"

    # macOS 26+ shows the Liquid Glass icon (the Icon Composer bundle compiled
    # into Assets.car, named by CFBundleIconName); older systems keep the .icns
    # above. That needs Xcode 26+'s actool: use the selected developer dir if it
    # has one, else Xcode.app directly (so an unset xcode-select doesn't matter),
    # else skip the glass icon with a warning.
    #
    # actool also writes its own Operandi.icns, so compile aside and copy only
    # Assets.car; the black-tile .icns stays the fallback. actool resolves
    # relative paths against its own helper's directory, so pass an absolute one.
    # The logo (assets/branding/Operandi.icon) is the bundle's only icon.
    glass_out="$scratch/glass"
    mkdir -p "$glass_out"
    if compile_glass "$glass_out" "$PWD/assets/branding/Operandi.icon"; then
        cp "$glass_out/Assets.car" "$app/Contents/Resources/Assets.car"
        /usr/libexec/PlistBuddy -c 'Add :CFBundleIconName string Operandi' "$plist"
    else
        echo "warning: could not compile assets/branding/Operandi.icon; skipping the Liquid Glass icon (.icns only)" >&2
        sed 's/^/  actool: /' "$glass_out/actool.log" >&2 || true
    fi

    if [ -n "$cycles_install" ]; then
        # The bridge plus the shared libraries it loads (build-cycles.sh puts
        # them all in one folder).
        mkdir -p "$app/Contents/Frameworks"
        cp "$cycles_install"/lib/*.dylib "$app/Contents/Frameworks/"
        # Kernel sources for the first Metal render, and licence texts.
        cp -R "$cycles_install/share/cycles" "$app/Contents/Resources/cycles"
    fi

    if [ -n "${USD_INSTALL:-}" ]; then
        mkdir -p "$app/Contents/Frameworks"
        cp "$USD_INSTALL"/lib/*.dylib "$app/Contents/Frameworks/"
        # Not in Frameworks: code signing allows only code there. The app
        # registers this folder itself (operandi-usd ffi::plugin_folder).
        cp -R "$USD_INSTALL/lib/usd" "$app/Contents/Resources/usd"
    fi

    if [ -n "${PYTHON_INSTALL:-}" ]; then
        py_minor=$("$PYTHON_INSTALL/bin/python3" -c 'import sys; print(f"{sys.version_info[0]}.{sys.version_info[1]}")')
        py_lib="libpython$py_minor.dylib"
        mkdir -p "$app/Contents/Frameworks" "$app/Contents/Resources/python/lib"
        cp "$PYTHON_INSTALL/lib/$py_lib" "$app/Contents/Frameworks/"
        install_name_tool -id "@executable_path/../Frameworks/$py_lib" "$app/Contents/Frameworks/$py_lib"
        install_name_tool -change "$(otool -D "$PYTHON_INSTALL/lib/$py_lib" | tail -1)" \
            "@executable_path/../Frameworks/$py_lib" "$app/Contents/MacOS/operandi-app"
        # The standard library without its tests, IDLE, Tk and pip.
        rsync -a --exclude test --exclude tests --exclude idlelib --exclude tkinter \
            --exclude turtledemo --exclude ensurepip --exclude __pycache__ \
            --exclude site-packages --exclude 'config-*' \
            "$PYTHON_INSTALL/lib/python$py_minor" "$app/Contents/Resources/python/lib/"
        mkdir -p "$app/Contents/Resources/python/lib/python$py_minor/site-packages"
        mkdir -p "$app/Contents/Resources/licenses/python"
        cp "$PYTHON_INSTALL/lib/python$py_minor/LICENSE.txt" "$app/Contents/Resources/licenses/python/"
        [ -f development/python/RELEASE ] && cp development/python/RELEASE "$app/Contents/Resources/licenses/python/"
    fi

    # Licence texts and credits (GPL v3 asks that they travel with the app).
    mkdir -p "$app/Contents/Resources/licenses"
    cp LICENSE docs/DEPENDENCIES.md "$app/Contents/Resources/licenses/"
    cp -R docs/licenses/. "$app/Contents/Resources/licenses/"
}

run_actool() {
    if xcrun --find actool >/dev/null 2>&1; then
        xcrun actool "$@"
    elif [ -d /Applications/Xcode.app/Contents/Developer ]; then
        DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer xcrun actool "$@"
    else
        echo "actool not found (no Xcode)"
        return 1
    fi
}

# compile_glass <output folder> <.icon bundles and extra actool options...>
compile_glass() {
    out="$1"
    shift
    run_actool "$@" --compile "$out" --app-icon Operandi \
        --platform macosx --target-device mac --minimum-deployment-target 12.0 \
        --output-partial-info-plist "$out/partial.plist" \
        --enable-on-demand-resources NO --development-region en >"$out/actool.log" 2>&1 \
        && [ -f "$out/Assets.car" ]
}
