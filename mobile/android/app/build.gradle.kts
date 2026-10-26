plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.plugin.compose")
    id("org.jlleitschuh.gradle.ktlint")
}

// The app ships with the desktop release of the same version (add-android-app D12).
val workspaceVersion: String =
    Regex("""^version = "([0-9]+\.[0-9]+\.[0-9]+)"""", RegexOption.MULTILINE)
        .find(rootDir.resolve("../../crates/hq-cli/Cargo.toml").readText())
        ?.groupValues
        ?.get(1)
        ?: error("no version in crates/hq-cli/Cargo.toml")
val (major, minor, patch) = workspaceVersion.split(".").map(String::toInt)

android {
    namespace = "com.alfonsusenrico.headquarter"
    compileSdk = 37

    defaultConfig {
        applicationId = "com.alfonsusenrico.headquarter"
        minSdk = 29
        targetSdk = 37
        versionCode = major * 10000 + minor * 100 + patch
        versionName = workspaceVersion
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    buildFeatures {
        compose = true
        buildConfig = true
    }

    lint {
        warningsAsErrors = true
        abortOnError = true
        lintConfig = file("lint.xml")
    }

    packaging {
        jniLibs {
            // Stripped by the `mobile` Cargo profile already.
            keepDebugSymbols += "**/libhq_mobile.so"
        }
    }
}

ktlint {
    version.set("1.8.0")
    filter {
        exclude { it.file.path.contains("/uniffi/") }
    }
}

// The phone's Rust core and its Kotlin binding, built before anything is compiled.
val androidCore by tasks.registering(Exec::class) {
    workingDir = rootDir.resolve("../..")
    commandLine("./scripts/android-core")
}
tasks.named("preBuild") { dependsOn(androidCore) }

dependencies {
    implementation(platform("androidx.compose:compose-bom:2026.09.00"))
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.activity:activity-compose:1.13.0")
    implementation("androidx.lifecycle:lifecycle-process:2.11.0")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.11.0")
    implementation("androidx.biometric:biometric:1.1.0")
    // The unlock prompt needs a FragmentActivity. Biometric alone brings an old fragment.
    implementation("androidx.fragment:fragment-ktx:1.9.1")
    implementation("androidx.camera:camera-camera2:1.6.2")
    implementation("androidx.camera:camera-lifecycle:1.6.2")
    implementation("androidx.camera:camera-view:1.6.2")
    implementation("com.google.zxing:core:3.5.4")
    // UniFFI's Kotlin binding loads the core through JNA.
    implementation("net.java.dev.jna:jna:5.19.1@aar")

    testImplementation("junit:junit:4.13.2")
    // Android's org.json is a stub in local unit tests.
    testImplementation("org.json:json:20260814")
}
