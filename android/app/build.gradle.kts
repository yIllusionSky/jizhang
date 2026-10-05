plugins { id("com.android.application") }

val releaseVersion = providers.environmentVariable("APP_VERSION").orElse("0.2.1")
val releaseCode = providers.environmentVariable("APP_VERSION_CODE").orElse("2001")
val signingFile = providers.environmentVariable("ANDROID_KEYSTORE_FILE")

android {
    namespace = "app.jizhang.wallet"
    compileSdk = 35
    defaultConfig {
        applicationId = "app.jizhang.wallet"
        minSdk = 26
        targetSdk = 35
        versionCode = releaseCode.get().toInt()
        versionName = releaseVersion.get()
        ndk { abiFilters += "arm64-v8a" }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    signingConfigs {
        if (signingFile.isPresent) {
            create("release") {
                storeFile = file(signingFile.get())
                storePassword = providers.environmentVariable("ANDROID_KEYSTORE_PASSWORD").get()
                keyAlias = providers.environmentVariable("ANDROID_KEY_ALIAS").get()
                keyPassword = providers.environmentVariable("ANDROID_KEY_PASSWORD").get()
            }
        }
    }
    buildTypes {
        getByName("release") {
            isDebuggable = false
            if (signingFile.isPresent) signingConfig = signingConfigs.getByName("release")
        }
    }
    listOf("debug", "release").forEach { variant ->
        sourceSets[variant].jniLibs.srcDir(layout.buildDirectory.dir("rustJniLibs/$variant"))
    }
    packaging { jniLibs.useLegacyPackaging = true }
}

listOf("debug", "release").forEach { variant ->
    val capitalized = variant.replaceFirstChar { it.uppercase() }
    val buildRust = tasks.register<Exec>("buildRust$capitalized") {
        workingDir(rootProject.projectDir.parentFile)
        commandLine("bash", "scripts/build-native.sh", variant)
        inputs.dir("../../crates")
        inputs.file("../../Cargo.toml")
        inputs.file("../../Cargo.lock")
        inputs.file("../../scripts/build-native.sh")
        outputs.dir(layout.buildDirectory.dir("rustJniLibs/$variant"))
    }
    tasks.matching { it.name == "pre${capitalized}Build" }.configureEach { dependsOn(buildRust) }
}
