plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

val lifestatsUrls = providers.gradleProperty("lifestats.urls").get()
val bedtimeStatisticId = providers.gradleProperty("lifestats.bedtimeStatisticId").get()

android {
    namespace = "dev.lifestats.bedtime"
    compileSdk = 36

    defaultConfig {
        applicationId = "dev.lifestats.bedtime"
        minSdk = 30
        targetSdk = 36
        versionCode = 1
        versionName = "1.0"

        buildConfigField("String", "LIFESTATS_URLS", "\"$lifestatsUrls\"")
        buildConfigField("long", "BEDTIME_STATISTIC_ID", "${bedtimeStatisticId}L")
    }

    buildFeatures {
        buildConfig = true
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            // Personal sideloaded app: sign with the debug key so `adb install -r` keeps working.
            signingConfig = signingConfigs.getByName("debug")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

kotlin {
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)
    }
}

dependencies {
    implementation("androidx.wear.tiles:tiles:1.5.0")
    implementation("androidx.wear.protolayout:protolayout:1.3.0")
    implementation("androidx.wear.protolayout:protolayout-material:1.3.0")
    implementation("androidx.work:work-runtime-ktx:2.10.1")
    implementation("com.google.guava:guava:33.4.0-android")

    testImplementation("junit:junit:4.13.2")
}
