plugins {
    alias(libs.plugins.android.application)
}

android {
    namespace = "org.rcouto.zeroute"
    compileSdk =
        libs.versions.compileSdk
            .get()
            .toInt()
    buildToolsVersion = libs.versions.buildTools.get()

    defaultConfig {
        applicationId = "org.rcouto.zeroute"
        minSdk =
            libs.versions.minSdk
                .get()
                .toInt()
        targetSdk =
            libs.versions.targetSdk
                .get()
                .toInt()
        versionCode =
            libs.versions.androidVersionCode
                .get()
                .toInt()
                .also { require(it > 0) }
        versionName =
            providers
                .gradleProperty("zerouteVersion")
                .orElse(providers.environmentVariable("ZEROUTE_VERSION"))
                .get()
        ndk.abiFilters.add("arm64-v8a")
    }

    compileOptions {
        sourceCompatibility = JavaVersion.toVersion(libs.versions.java.get())
        targetCompatibility = JavaVersion.toVersion(libs.versions.java.get())
    }

    lint {
        abortOnError = true
        warningsAsErrors = true
        textReport = true
        // Review toolchain upgrades together with the pinned nixpkgs Gradle.
        informational.add("AndroidGradlePluginVersion")
    }
}
