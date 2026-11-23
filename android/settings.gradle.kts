pluginManagement { repositories { google(); mavenCentral(); gradlePluginPortal() } }
// 利用状況の集計 (Aptabase) は JitPack から取る。
dependencyResolutionManagement { repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS); repositories { google(); mavenCentral(); maven { url = uri("https://jitpack.io") } } }
rootProject.name = "MeltypeRust"
include(":app")
