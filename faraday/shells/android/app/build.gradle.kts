// Faraday Companion's Android shell, copied from upstream's
// opensigner/shells/android at d26b84e (README.md says what was left out).
//
// The core arrives as `app/src/main/jniLibs/<abi>/libfaraday_companion.so`,
// put there by `just companion-lib`. That directory is not committed, so a
// checkout that skips the step fails the build rather than producing an
// APK with no core in it.

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

// The version name is the Companion's crate version, the release's
// (docs/COMPANION.md §3), read here so there is one place to change it.
val companionVersion: String = run {
    val manifest = rootDir.resolve("../../faraday-companion/Cargo.toml").readText()
    Regex("""(?m)^version\s*=\s*"([^"]+)"""").find(manifest)?.groupValues?.get(1)
        ?: error("no version in faraday/faraday-companion/Cargo.toml")
}

android {
    namespace = "app.faradayoffline.companion"
    compileSdk = 36

    // Pinned, as upstream's: a release APK has to come out of one set of
    // tools.
    buildToolsVersion = "36.0.0"
    ndkVersion = "29.0.14206865"

    defaultConfig {
        applicationId = "app.faradayoffline.companion"
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = companionVersion

        // The ABIs `just companion-lib` builds: arm64 phones and the
        // x86_64 emulator (docs/COMPANION.md §2).
        ndk {
            abiFilters += listOf("arm64-v8a", "x86_64")
        }
    }

    packaging {
        jniLibs {
            // The .so is loaded with System.loadLibrary, so it must be a
            // real file in the APK rather than a compressed entry.
            useLegacyPackaging = false
        }
    }

    buildTypes {
        release {
            // Shrinking is off: R8's output is one more thing two builders
            // would have to match.
            isMinifyEnabled = false

            // Nothing about the checkout it was built in goes into the APK
            // (docs/COMPANION.md §3).
            vcsInfo { include = false }
        }
    }

    // The dependency blob is a signed list of libraries for the Play
    // Console: bytes no second builder can reproduce, in an app with no
    // dependencies, for a store this app is not on.
    dependenciesInfo {
        includeInApk = false
        includeInBundle = false
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    lint {
        // The shell has no accessibility surface of its own: the core
        // draws every screen and takes touches on the one view.
        disable += "ClickableViewAccessibility"
    }
}

kotlin {
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)
    }
}

// Nothing beyond the Kotlin standard library the plugin adds
// (docs/COMPANION.md §0): no AndroidX, no Material, no Play services, no
// analytics, no crash reporting. `tools/lint-companion-android.sh` fails
// the build if anything appears here.
dependencies {}
