// The core arrives as `app/src/main/jniLibs/<abi>/libopensigner.so`, put
// there by `just android-lib`. That directory is not committed, so a
// checkout that skips the step fails the build rather than producing an
// APK with no core in it.

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "app.opensigner"
    compileSdk = 36

    // Pinned, because a release APK has to come out of one set of tools
    // (docs/PLANNING.md §11.1). tools/build/android/Dockerfile installs
    // exactly these two; a host build wants them in $ANDROID_HOME too.
    buildToolsVersion = "36.0.0"
    ndkVersion = "29.0.14206865"

    defaultConfig {
        applicationId = "app.opensigner"
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"

        // The ABIs `just android-lib` builds. An APK with no other ABI is
        // an APK that cannot silently ship an unbuilt architecture.
        ndk {
            abiFilters += listOf("arm64-v8a", "x86_64")
        }
    }

    buildFeatures {
        // BuildConfig.VERSION_NAME is what the About screen shows.
        buildConfig = true
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
            // would have to match, and the app is 30 kB of Kotlin.
            isMinifyEnabled = false

            // AGP otherwise packages whatever it can learn about the
            // version control of the directory it built in. What that
            // comes out as depends on the builder's checkout rather than
            // on the source, and the release manifest already names the
            // commit.
            vcsInfo { include = false }
        }
    }

    // The dependency blob is a signed list of libraries for the Play
    // Console. It is bytes no second builder can reproduce, in an app with
    // no dependencies, for a store this app is not on.
    dependenciesInfo {
        includeInApk = false
        includeInBundle = false
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    lint {
        // The shell has no accessibility surface by design (§4.5): the
        // core renders its own keyboard and never uses a system text field.
        disable += "ClickableViewAccessibility"
    }
}

kotlin {
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)
    }
}

// Nothing beyond the Kotlin standard library the plugin adds. Not a
// stylistic choice: every library in the shell is a library inside the
// trust boundary of a signing device (§2.4). No AndroidX, no Material, no
// Play services, no analytics, no crash reporting.
//
// The authentication prompt behind a kept key is the platform's own
// `android.hardware.biometrics.BiometricPrompt`, not `androidx.biometric`.
// The AndroidX library exists to back-port the prompt and the
// biometric-or-credential choice to older releases; a kept key needs
// Android 11 for the Keystore properties it stands on anyway, and there
// the platform prompt does everything this shell asks of it. So the one
// dependency it would have justified is not here either.
dependencies {}
