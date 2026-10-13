// Only the two repositories the Android and Kotlin plugins come from.
// Nothing in this build talks to the network after the first resolve, and
// the app itself has no dependencies at all (no AndroidX, no Play
// services, no analytics — docs/COMPANION.md §0).
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

rootProject.name = "FaradayCompanion"
include(":app")
