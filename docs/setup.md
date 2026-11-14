# Entorno de desarrollo

Requisitos para compilar y ejecutar la app en Windows, macOS y Linux.

## Versiones de referencia

Estas son las versiones con las que se validó el proyecto por primera vez:

| Herramienta | Versión validada |
|---|---|
| Rust | 1.100.0 (`stable-x86_64-pc-windows-msvc`) |
| Visual Studio Build Tools | 2022, MSVC 14.44, Windows SDK |
| CMake | 4.5.0 |
| LLVM/clang | 23.1.5 |
| Node | 22.14.0 (runtime JavaScript para `yt-dlp`) |
| ffmpeg | 8.0 |
| yt-dlp | 2026.10.21 |

Las versiones mínimas del proyecto son Rust 1.89 (por `charon-audio`/`ort`) y CMake 3.20. Las versiones más recientes de las herramientas deberían funcionar, pero si cambias una, vuelve a ejecutar la verificación.

## Windows

### 1. Visual Studio Build Tools

Instala **Visual Studio Build Tools 2022** con la carga de trabajo **Desarrollo para el escritorio con C++**. Incluye MSVC, el Windows SDK y las librerías de enlace que necesita Rust.

Descarga: <https://visualstudio.microsoft.com/visual-cpp-build-tools/>

Instalación silenciosa desde PowerShell elevado:

```powershell
winget install --id Microsoft.VisualStudio.2022.BuildTools `
  --override "--quiet --wait --norestart --nocache --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

Comprueba que existe `link.exe`:

```powershell
Get-ChildItem "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC" -Recurse -Filter link.exe |
  Select-Object -First 1 FullName
```

### 2. Rust

```powershell
Invoke-WebRequest https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe -OutFile rustup-init.exe
.\rustup-init.exe -y --default-toolchain stable-x86_64-pc-windows-msvc --profile default
```

Cierra y vuelve a abrir la terminal para que `cargo` entre en el `PATH`.

### 3. CMake y LLVM

`whisper-rs` necesita CMake y `libclang` para compilar whisper.cpp y generar los bindings.

```powershell
$tools = "$env:LOCALAPPDATA\Programs"
New-Item -ItemType Directory -Force $tools | Out-Null

Invoke-WebRequest https://github.com/Kitware/CMake/releases/download/v4.5.0/cmake-4.5.0-windows-x86_64.zip -OutFile "$env:TEMP\cmake.zip"
Expand-Archive "$env:TEMP\cmake.zip" -DestinationPath $tools -Force

Invoke-WebRequest https://github.com/llvm/llvm-project/releases/download/llvmorg-23.1.5/clang+llvm-23.1.5-x86_64-pc-windows-msvc.tar.xz -OutFile "$env:TEMP\llvm.tar.xz"
New-Item -ItemType Directory -Force "$tools\LLVM" | Out-Null
tar -xf "$env:TEMP\llvm.tar.xz" -C "$tools\LLVM" --strip-components=1
```

Añade las rutas a tu `PATH` de usuario y define `LIBCLANG_PATH`:

```powershell
$cmakeBin = "$env:LOCALAPPDATA\Programs\cmake-4.5.0-windows-x86_64\bin"
$llvmBin  = "$env:LOCALAPPDATA\Programs\LLVM\bin"
[Environment]::SetEnvironmentVariable(
  "Path",
  "$env:USERPROFILE\.cargo\bin;$cmakeBin;$llvmBin;" + [Environment]::GetEnvironmentVariable("Path", "User"),
  "User"
)
[Environment]::SetEnvironmentVariable("LIBCLANG_PATH", $llvmBin, "User")
```

Reinicia la terminal.

### 4. Runtime de medios

```powershell
winget install ffmpeg
winget install yt-dlp
winget install DenoLand.Deno
```

`yt-dlp` necesita un runtime JavaScript para resolver los desafíos de YouTube. La app detecta **Deno** primero y **Node** después; Node 22 también sirve:

```powershell
node --version
```

## macOS

```bash
xcode-select --install
brew install cmake llvm ffmpeg yt-dlp deno
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Añade LLVM al entorno:

```bash
export LIBCLANG_PATH="$(brew --prefix llvm)/lib"
export PATH="$(brew --prefix llvm)/bin:$(brew --prefix cmake)/bin:$PATH"
```

## Linux (Debian/Ubuntu)

```bash
sudo apt update
sudo apt install -y build-essential cmake clang libclang-dev pkg-config \
  libasound2-dev libxkbcommon-dev libgl1-mesa-dev ffmpeg yt-dlp
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Instala Deno o Node para `yt-dlp`. `rodio`/`cpal` necesitan ALSA; sin `libasound2-dev` la compilación falla.

### 5. ONNX Runtime en Windows

`charon-audio` usa `ort` con `copy-dylibs`, que en Windows espera que existan `target\<perfil>\build\examples` y `target\<perfil>\build\deps`. Créalas antes de compilar:

```powershell
New-Item -ItemType Directory -Force target\debug\build\examples,target\debug\build\deps
```

Para tests y ejecución, añade `target\debug\build` al `PATH` (contiene `DirectML.dll`):

```powershell
$env:Path = "$PWD\target\debug\build;$env:Path"
```

Para `--release`, usa `target\release\build`.

## Verificación

Desde la raíz del repositorio:

```bash
rustc --version
cargo --version
cmake --version
clang --version

cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Compilación real de `whisper-rs` (la que valida CMake, LLVM y MSVC):

```bash
cargo new "$TEMP/whisper-smoke" --bin --vcs none
cd "$TEMP/whisper-smoke"
cargo add whisper-rs@0.16
cargo build
```

Si `whisper-rs-sys` falla con `Unable to find libclang`, revisa `LIBCLANG_PATH`. Si falla al encontrar `link.exe`, faltan las Build Tools o el Windows SDK.

## Modelos

La app no incluye modelos en el repositorio. En la primera ejecución descarga HTDemucs ONNX, Whisper GGML y los recursos auxiliares en el directorio de datos de la aplicación. Consulta [models.md](models.md) y [licenses.md](licenses.md).

## Problemas frecuentes

| Síntoma | Causa probable | Solución |
|---|---|---|
| `link.exe not found` | Build Tools/SDK ausentes | Instala la carga VCTools |
| `Unable to find libclang` | `LIBCLANG_PATH` mal definido | Apunta a la carpeta que contiene `libclang.dll` |
| `CMake not found` | CMake no está en `PATH` | Añade la carpeta `bin` de CMake |
| `No supported JavaScript runtime` | Falta Deno/Node | Instala Deno o Node |
| `ffmpeg not found` | `ffmpeg` no está en `PATH` | Instálalo y reinicia la terminal |
