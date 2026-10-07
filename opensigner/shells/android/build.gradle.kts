plugins {
    id("com.android.application") version "8.13.1" apply false
    id("org.jetbrains.kotlin.android") version "2.3.0" apply false
}

// Reproducibility (docs/PLANNING.md §11.1). Gradle stamps every archive it
// writes with the time it was written and packs entries in whatever order
// the file system offered them, so two builds of one tree differ. Neither
// is turned off by default; both are here, for every project.
allprojects {
    tasks.withType<AbstractArchiveTask>().configureEach {
        isPreserveFileTimestamps = false
        isReproducibleFileOrder = true
    }
}
