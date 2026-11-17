﻿# Gera o APK do AbyssHub (Rust + a parte em Java do login/Firebase).
#   .\build-android.ps1            -> gera APK\AbyssHub-release.apk
#   .\build-android.ps1 -Instalar  -> gera, instala no celular ligado no USB e abre
#   .\build-android.ps1 -Debug     -> versão debug (compila mais rápido)
param(
    [switch]$Instalar,
    [switch]$Debug
)
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

# --- Android SDK ---
if (-not $env:ANDROID_HOME) {
    $sdk = Join-Path $env:LOCALAPPDATA "Android\Sdk"
    if (Test-Path $sdk) { $env:ANDROID_HOME = $sdk }
    else { throw "Android SDK não encontrado. Instale pelo Android Studio ou defina ANDROID_HOME." }
}
$env:ANDROID_SDK_ROOT = $env:ANDROID_HOME

# --- NDK (pega a versão mais nova instalada) ---
if (-not $env:ANDROID_NDK_ROOT) {
    $ndk = Get-ChildItem (Join-Path $env:ANDROID_HOME "ndk") -Directory -ErrorAction SilentlyContinue |
        Sort-Object { [version]($_.Name) } -Descending | Select-Object -First 1
    if ($ndk) { $env:ANDROID_NDK_ROOT = $ndk.FullName }
    else { throw "NDK não encontrado. Android Studio > SDK Manager > SDK Tools > marque 'NDK (Side by side)'." }
}
$env:ANDROID_NDK_HOME = $env:ANDROID_NDK_ROOT

# --- Java (o Gradle precisa do Java 17+; o Android Studio já traz um) ---
if (-not $env:JAVA_HOME) {
    foreach ($j in @("C:\Program Files\Android\Android Studio\jbr", "$env:LOCALAPPDATA\Programs\Android Studio\jbr")) {
        if (Test-Path $j) { $env:JAVA_HOME = $j; break }
    }
}
if ($env:JAVA_HOME) { $env:Path = "$env:JAVA_HOME\bin;$env:Path" }
$env:Path = "$env:ANDROID_HOME\platform-tools;$env:Path"

Write-Host "SDK:  $env:ANDROID_HOME"
Write-Host "NDK:  $env:ANDROID_NDK_ROOT"
Write-Host "Java: $env:JAVA_HOME"

# --- Ferramentas Rust ---
rustup target add aarch64-linux-android | Out-Null
if (-not (Get-Command cargo-ndk -ErrorAction SilentlyContinue)) {
    Write-Host "Instalando cargo-ndk (só na primeira vez)..."
    cargo install cargo-ndk
}

# --- 1. Compila o Rust para o celular (libabysshub.so) ---
$perfil = @()
if (-not $Debug) { $perfil = @("--release") }
cargo ndk -t arm64-v8a -P 26 -o android\app\src\main\jniLibs build --lib @perfil
if ($LASTEXITCODE -ne 0) { throw "O Rust não compilou (veja o erro acima)." }

# --- 2. Monta o APK com o Gradle ---
"sdk.dir=$($env:ANDROID_HOME -replace '\\', '/')" | Set-Content -Encoding ASCII android\local.properties
if (-not (Test-Path android\app\google-services.json)) {
    Write-Host ""
    Write-Host "Aviso: android\app\google-services.json não existe. O app funciona, mas sem login/Leaderboard." -ForegroundColor Yellow
    Write-Host "       Veja a seção Para desenvolvedores do README." -ForegroundColor Yellow
}
$tipo = if ($Debug) { "debug" } else { "release" }
$tarefa = if ($Debug) { "assembleDebug" } else { "assembleRelease" }
Push-Location android
try {
    .\gradlew.bat $tarefa
    if ($LASTEXITCODE -ne 0) { throw "O Gradle não conseguiu montar o APK (veja o erro acima)." }
} finally {
    Pop-Location
}
$apk = "APK\AbyssHub-$tipo.apk"
Write-Host ""
Write-Host "APK pronto em: $apk" -ForegroundColor Green

# --- 3. Instala no celular ---
if ($Instalar) {
    adb install -r $apk
    adb shell am start -n com.abysshub.app/.MainActivity
}
