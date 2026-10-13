#!/usr/bin/env bash
# Faraday Companion's Android shell, checked offline (docs/COMPANION.md §6.2).
#
# The Companion is offline because its manifest asks for no network, and it
# stays so only while nothing in the shell adds a permission, a library or
# a way out. This lint reads the shell's sources and fails when:
#
#   - the manifest has a <uses-permission> (of any form) other than CAMERA
#     and USE_BIOMETRIC, or any <uses-library>;
#   - the manifest declares a component beyond the one activity
#     (.MainActivity) and the one provider (.ShareProvider, the source of
#     Share, which must say android:exported="false"): any service,
#     receiver, activity alias or other provider;
#   - an intent filter names an action other than MAIN (the launcher) and
#     SEND (the share target), a category other than LAUNCHER and DEFAULT,
#     a <data> with anything but a mimeType, or there are more than those
#     two filters;
#   - a Gradle file declares a dependency: app/build.gradle.kts's
#     `dependencies {}` is not empty, or any Gradle file names a
#     configuration that brings a library in;
#   - a `plugins {}` block names a plugin other than the two upstream's
#     shell has (com.android.application, org.jetbrains.kotlin.android);
#   - the Kotlin opens a link or writes the clipboard: ACTION_VIEW,
#     `Uri.parse("http`, setPrimaryClip, `java.net.` or HttpURLConnection.
#
# Each failure names the file and the line. Run from the repository root;
# `just lint` does.
set -u
cd "$(dirname "$0")/.."

SHELL_DIR=faraday/shells/android
MANIFEST=$SHELL_DIR/app/src/main/AndroidManifest.xml
ALLOWED_PERMISSIONS="android.permission.CAMERA android.permission.USE_BIOMETRIC"
ALLOWED_PLUGINS="com.android.application org.jetbrains.kotlin.android"

status=0
fail() {
    echo "lint-companion-android: $1"
    status=1
}

if [ ! -f "$MANIFEST" ]; then
    fail "$MANIFEST is missing"
    exit 1
fi

# The manifest: what it asks for, and no shared library. Comments are
# skipped, so the explanation above the permissions can name others.
while IFS=$'\t' read -r line text; do
    case "$text" in
    *"<uses-library"*)
        fail "$MANIFEST:$line: <uses-library> is not allowed"
        ;;
    *"<uses-permission"*)
        name=$(echo "$text" | sed -n 's/.*android:name="\([^"]*\)".*/\1/p')
        allowed=0
        for p in $ALLOWED_PERMISSIONS; do
            [ "$name" = "$p" ] && allowed=1
        done
        case "$text" in
        *"<uses-permission "* | *"<uses-permission>"*) ;;
        # <uses-permission-sdk-23> and any other form.
        *)
            fail "$MANIFEST:$line: only <uses-permission> is allowed, not $(echo "$text" | sed -n 's/.*\(<uses-permission[^ >]*\).*/\1/p')"
            continue
            ;;
        esac
        if [ "$allowed" = 0 ]; then
            fail "$MANIFEST:$line: permission ${name:-?} is not one of: $ALLOWED_PERMISSIONS"
        fi
        ;;
    esac
done < <(awk '
    { line = $0; out = "" }
    {
        while (length(line) > 0) {
            if (incomment) {
                end = index(line, "-->")
                if (end == 0) { line = ""; break }
                line = substr(line, end + 3); incomment = 0
            } else {
                start = index(line, "<!--")
                if (start == 0) { out = out line; line = ""; break }
                out = out substr(line, 1, start - 1)
                line = substr(line, start + 4); incomment = 1
            }
        }
        if (out ~ /[^[:space:]]/) printf "%d\t%s\n", NR, out
    }' "$MANIFEST")

# The manifest's components and intent filters: one element a line, with
# the line it starts on, comments removed and attributes joined.
elements() {
    awk '
        { line = $0; out = "" }
        {
            while (length(line) > 0) {
                if (incomment) {
                    end = index(line, "-->")
                    if (end == 0) { line = ""; break }
                    line = substr(line, end + 3); incomment = 0
                } else {
                    start = index(line, "<!--")
                    if (start == 0) { out = out line; line = ""; break }
                    out = out substr(line, 1, start - 1)
                    line = substr(line, start + 4); incomment = 1
                }
            }
            print out
        }' "$MANIFEST" | awk 'BEGIN { RS = "<"; line = 1 }
        {
            text = $0; gsub(/[[:space:]]+/, " ", text)
            if (NR > 1 && text !~ /^\//) printf "%d\t%s\n", line, text
            line += gsub(/\n/, "\n", $0)
        }'
}
attr() { echo "$1" | sed -n "s/.* android:$2=\"\([^\"]*\)\".*/\1/p"; }
activities=0
providers=0
filters=0
while IFS=$'\t' read -r line text; do
    case "$text" in
    "activity-alias"* | "service"* | "receiver"*)
        fail "$MANIFEST:$line: <${text%% *}> is not allowed"
        ;;
    "activity "* | "activity>"*)
        activities=$((activities + 1))
        name=$(attr "$text" name)
        [ "$name" = ".MainActivity" ] || fail "$MANIFEST:$line: activity ${name:-?} is not .MainActivity"
        ;;
    "provider "* | "provider>"*)
        providers=$((providers + 1))
        name=$(attr "$text" name)
        [ "$name" = ".ShareProvider" ] || fail "$MANIFEST:$line: provider ${name:-?} is not .ShareProvider"
        [ "$(attr "$text" exported)" = "false" ] || fail "$MANIFEST:$line: the provider must say android:exported=\"false\""
        ;;
    "intent-filter"*)
        filters=$((filters + 1))
        ;;
    "action "*)
        name=$(attr "$text" name)
        case "$name" in
        android.intent.action.MAIN | android.intent.action.SEND) ;;
        *) fail "$MANIFEST:$line: action ${name:-?} is not MAIN or SEND" ;;
        esac
        ;;
    "category "*)
        name=$(attr "$text" name)
        case "$name" in
        android.intent.category.LAUNCHER | android.intent.category.DEFAULT) ;;
        *) fail "$MANIFEST:$line: category ${name:-?} is not LAUNCHER or DEFAULT" ;;
        esac
        ;;
    "data "* | "data>"* | "data/"*)
        rest=$(echo "$text" | sed 's/ android:mimeType="[^"]*"//; s/^data//; s/[ \/>]//g')
        [ -z "$rest" ] || fail "$MANIFEST:$line: <data> may carry a mimeType only"
        ;;
    esac
done < <(elements)
[ "$activities" -le 1 ] || fail "$MANIFEST: more than one activity"
[ "$providers" -le 1 ] || fail "$MANIFEST: more than one provider"
[ "$filters" -le 2 ] || fail "$MANIFEST: more than the launcher's and SEND's intent filters"

# Gradle: no dependency, and only the two plugins.
gradle_files=$(find "$SHELL_DIR" -name '*.gradle.kts' -not -path '*/build/*' | sort)
for f in $gradle_files; do
    while IFS=: read -r line text; do
        fail "$f:$line: a dependency is not allowed: $(echo "$text" | sed 's/^[[:space:]]*//')"
    done < <(grep -nE '^[^/]*\b(implementation|api|compileOnly|runtimeOnly|kapt|ksp|annotationProcessor|classpath|testImplementation|androidTestImplementation|coreLibraryDesugaring)[[:space:]]*\(' "$f")
    while IFS=: read -r line text; do
        fail "$f:$line: dependencies must be empty: $(echo "$text" | sed 's/^[[:space:]]*//')"
    done < <(grep -nE '^[^/]*\bdependencies[[:space:]]*\{' "$f" | grep -vE 'dependencies[[:space:]]*\{[[:space:]]*\}')
    while IFS=$'\t' read -r line id; do
        allowed=0
        for p in $ALLOWED_PLUGINS; do
            [ "$id" = "$p" ] && allowed=1
        done
        [ "$allowed" = 0 ] && fail "$f:$line: plugin $id is not one of: $ALLOWED_PLUGINS"
    done < <(awk '
        /^[[:space:]]*plugins[[:space:]]*\{/ { inside = 1; next }
        inside && /^[[:space:]]*\}/ { inside = 0; next }
        inside && /[^[:space:]]/ && !/^[[:space:]]*\/\// {
            if (match($0, /id\("[^"]*"\)/)) {
                printf "%d\t%s\n", NR, substr($0, RSTART + 4, RLENGTH - 6)
            } else {
                printf "%d\t%s\n", NR, $0
            }
        }' "$f")
done
app_gradle=$SHELL_DIR/app/build.gradle.kts
if ! grep -qE '^dependencies[[:space:]]*\{[[:space:]]*\}' "$app_gradle"; then
    fail "$app_gradle: no empty \`dependencies {}\`"
fi

# The Kotlin: no link opened, no clipboard written, no network class.
while IFS= read -r hit; do
    fail "$hit"
done < <(grep -rnE 'ACTION_VIEW|Uri\.parse\("http|setPrimaryClip|java\.net\.|HttpURLConnection' \
    --include='*.kt' --include='*.java' "$SHELL_DIR/app/src" | sed 's/$/: not allowed (docs\/COMPANION.md §6.3)/')

if [ "$status" -eq 0 ]; then
    echo "lint-companion-android: ok"
fi
exit "$status"
