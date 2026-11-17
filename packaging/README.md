# Empaquetado

Scripts que convierten el binario de `usdx-desktop` en el instalador nativo de
cada sistema operativo. El workflow de release los invoca después de compilar y
de superar `--selftest`; en local se pueden ejecutar a mano en el sistema
destino.

| Plataforma | Script | Artefacto | Herramientas |
|---|---|---|---|
| Linux | `packaging/linux/package.sh` | `.deb` | `dpkg-deb`, `dpkg-shlibdeps` (`dpkg-dev`) |
| macOS | `packaging/macos/package.sh` | `.dmg` con `.app` | `hdiutil`, `iconutil`, `plutil` |
| Windows | `packaging/windows/package.ps1` | `.msi` | WiX Toolset v5 (`dotnet tool install --global wix`) |

## Entradas comunes

Los scripts no compilan nada. Esperan un binario de release ya construido y la
versión calculada por `packaging/version.sh` (o `version.ps1` en Windows):

```bash
eval "$(packaging/version.sh)"
cargo build --release -p usdx-desktop
packaging/linux/package.sh
```

Variables de entorno reconocidas:

- `BINARY`: ruta al ejecutable. Por defecto `target/release/usdx-desktop`
  (`target/release/usdx-desktop.exe` en Windows).
- `OUT_DIR`: carpeta de salida. Por defecto `dist/`.
- `DEB_ARCH`: arquitectura del `.deb`. Por defecto la que informe `dpkg
  --print-architecture` (`amd64` en CI). Solo lo usa el script de Linux.

Cada script crea `dist/` si no existe y escribe un único artefacto con el
nombre que espera el workflow:

- `ultrastar-song-generator_<versión>_amd64.deb`
- `UltraStar-Song-Generator-<versión>-macos-arm64.dmg`
- `UltraStar-Song-Generator-<versión>-windows-x64.msi`

## Requisitos

- **Linux:** construir en la misma distribución que se quiere soportar
  (`ubuntu-24.04` en CI). `dpkg-shlibdeps` toma las dependencias del sistema
  donde se empaqueta.
- **macOS:** Apple Silicon; el script usa `iconutil` y `hdiutil`, incluidos en
  macOS. `assets/icons/AppIcon.iconset` aporta todos los tamaños salvo 512 y
  1024, que el script deriva del PNG original.
- **Windows:** WiX v5 instalado como herramienta global de .NET. El MSI es de
  usuario (`Scope="perUser"`): no pide UAC y no necesita permisos de
  administrador.

## Verificación

Cada script imprime la ruta del artefacto. La comprobación completa (instalar,
ejecutar `usdx-desktop --selftest`, desinstalar) la hace el job de release de
cada plataforma; ver `docs/install.md` para el detalle de usuario.
