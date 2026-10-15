#!/usr/bin/env bash
# The APK fixtures for faraday-apk's tests (docs/APK-SIGNING.md §8), made
# with Android's own tools at the build-tools version the release build
# pins (aapt2, zipalign) and, for the variants, tools/vectors/apk/variants.py.
#
#   tools/vectors/apk/make.sh              writes the committed fixtures here
#   tools/vectors/apk/make.sh DIR          writes them into DIR
#   tools/vectors/apk/make.sh --large DIR  writes large-unsigned.apk into DIR:
#                                          the small one with a stored 2.5 MiB
#                                          asset, not committed
#
# Needs ANDROID_HOME (or FARADAY_BUILD_TOOLS for the build-tools directory
# and FARADAY_ANDROID_JAR for android.jar) and python3. Nothing here has a
# key: the fixtures are unsigned.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PINNED="36.0.0"

large=0
if [ "${1:-}" = "--large" ]; then
    large=1
    shift
fi
OUT="${1:-$HERE}"
mkdir -p "$OUT"

TOOLS="${FARADAY_BUILD_TOOLS:-${ANDROID_HOME:?set ANDROID_HOME or FARADAY_BUILD_TOOLS}/build-tools/$PINNED}"
AAPT2="$TOOLS/aapt2"
ZIPALIGN="$TOOLS/zipalign"
JAR="${FARADAY_ANDROID_JAR:-${ANDROID_HOME:-}/platforms/android-36/android.jar}"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# aapt2's own manifest-only APK, then a stored library of fixed bytes
# aligned with `zipalign -P 16 4`.
base() { # min-sdk output
    local min="$1" out="$2"
    cat >"$work/AndroidManifest.xml" <<'EOF'
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="app.faradayoffline.fixture">
    <application android:hasCode="false" android:label="Fixture" />
</manifest>
EOF
    "$AAPT2" link -o "$work/linked.apk" --manifest "$work/AndroidManifest.xml" \
        -I "$JAR" --min-sdk-version "$min" --target-sdk-version 36 \
        --version-code 1 --version-name 1.0
    python3 -I "$HERE/variants.py" add-library "$work/linked.apk" "$work/unaligned.apk"
    "$ZIPALIGN" -f -P 16 4 "$work/unaligned.apk" "$out"
    "$ZIPALIGN" -c -P 16 4 "$out"
}

if [ "$large" = 1 ]; then
    python3 -I "$HERE/variants.py" add-asset "$HERE/small-unsigned.apk" "$work/big.apk"
    "$ZIPALIGN" -f -P 16 4 "$work/big.apk" "$OUT/large-unsigned.apk"
    "$ZIPALIGN" -c -P 16 4 "$OUT/large-unsigned.apk"
    exit 0
fi

base 26 "$OUT/small-unsigned.apk"
base 21 "$OUT/old-sdk-unsigned.apk"
base 33 "$OUT/sdk33-unsigned.apk"

# The same, with its library off its boundary: never run through zipalign.
python3 -I "$HERE/variants.py" unaligned "$OUT/small-unsigned.apk" "$OUT/unaligned-unsigned.apk"

# One file per rule of "One reading only" (docs/APK-SIGNING.md §5).
python3 -I "$HERE/variants.py" variants "$OUT/small-unsigned.apk" "$OUT"
ls -l "$OUT"/*.apk
