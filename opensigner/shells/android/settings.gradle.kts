// Only the two repositories the Android and Kotlin plugins come from.
// Nothing in this build talks to the network after the first resolve, and
// the app itself has no dependencies at all (no AndroidX, no Play
// services, no analytics — docs/PLANNING.md §11.2).
pluginManagement {
    repositories {
        google()
        mavenCentral()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "OpenSigner"
include(":app")
