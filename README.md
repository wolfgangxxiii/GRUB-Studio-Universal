<p align="center">
  <img src="assets/grub-studio.svg" alt="GRUB Studio Universal logo" width="100">
</p>

<h1 align="center">GRUB Studio Universal</h1>

<p align="center">
  <strong>A modern, portable GRUB configuration studio for Linux.</strong><br>
  Rust · GTK4 · Wayland · Light & Dark
</p>

<p align="center">
  <a href="#polski">🇵🇱 Polski</a> &nbsp;·&nbsp; <a href="#english">🇬🇧 English</a>
</p>

<p align="center">
  <a href="https://github.com/wolfgangxxiii/GRUB-Studio-Universal/actions/workflows/test-and-build.yml"><img src="https://github.com/wolfgangxxiii/GRUB-Studio-Universal/actions/workflows/test-and-build.yml/badge.svg" alt="Build status"></a>
  <img src="https://img.shields.io/badge/Rust-2021-CE422B?logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/GTK-4-4A86CF?logo=gtk&logoColor=white" alt="GTK4">
  <img src="https://img.shields.io/badge/Linux-x86__64-1793D1?logo=linux&logoColor=white" alt="Linux x86_64">
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-GPL--3.0--or--later-38A169" alt="GPL-3.0-or-later"></a>
</p>

<p align="center">
  <a href="https://github.com/wolfgangxxiii/GRUB-Studio-Universal/releases/download/v2.3.8-rc.1/GRUB-Studio-Universal-v2.3.8-x86_64.AppImage"><strong>⬇️ Download AppImage · v2.3.8 RC1</strong></a>
  &nbsp;·&nbsp;
  <a href="https://github.com/wolfgangxxiii/GRUB-Studio-Universal/releases">All releases</a>
</p>

<p align="center"><sub>Actual GTK4 interface screenshots are generated as CI artifacts, without storing historical images in the source repository.</sub></p>

---

<a id="polski"></a>

## 🇵🇱 Polski

**GRUB Studio Universal** to graficzne narzędzie do zarządzania konfiguracją programu rozruchowego GRUB. Powstało w **Rust** i korzysta z natywnego interfejsu **GTK4**, przystosowanego do Waylanda oraz ekranów o różnej wielkości.

### ✨ Najważniejsze funkcje

| Obszar | Możliwości |
| --- | --- |
| 🎨 **Interfejs** | **Stabilne dekoracje okna obsługiwane przez system**, oddzielny pasek narzędzi w aplikacji, spójne ikony wektorowe, zawsze widoczna nawigacja 196/62 px, jasny/ciemny wygląd, siedem języków i kursor systemowy. |
| 🐧 **Dystrybucje** | Rozpoznawanie Bazzite, Fedora, Ubuntu/Debian, Arch/CachyOS i openSUSE. |
| 🖥️ **Systemy** | Wspólny widok informacji GRUB/EFI bez duplikowania systemów o tej samej nazwie; opcjonalny skan `os-prober`. |
| 🖼️ **Motywy GRUB** | Wykrywanie `theme.txt`, lista z lokalizacją plików, stan aktywności, import oraz przyciski zastosowania/wyłączenia dostępne nawet przy przewijaniu. Bez symulowanego podglądu. |
| ⚙️ **Ustawienia** | Czas wyświetlania menu, przetłumaczone style menu, lista rozdzielczości i parametry rozruchu. Niezapisane wartości formularza pozostają przy zmianie kart; bez prawa zapisu wyświetla się podsumowanie tylko do odczytu. |
| 📋 **Wpisy rozruchowe** | Prostsza karta „Menu rozruchowe” z czytelnym stanem blokady i jednym panelem wykrywania; edycja własnych EFI tylko na obsługiwanych konfiguracjach. Niezapisane dane pozostają także po ręcznym odświeżeniu. |
| 🧰 **Narzędzia** | Kopie zapasowe i dziennik działań dostępne razem bez duplikowania pozycji menu. |
| 🛡️ **Ochrona zmian** | Polkit, osobny status blokady i trybu tylko do odczytu, walidacja ścieżek EFI i kopie bezpieczeństwa. Generator tworzy plik GRUB osobno, sprawdza składnię i dopiero wtedy zastępuje konfigurację rozruchową. |

### ⬇️ Pobieranie i uruchamianie

**[Pobierz GRUB Studio Universal 2.3.8 — AppImage (x86_64)](https://github.com/wolfgangxxiii/GRUB-Studio-Universal/releases/download/v2.3.8-rc.1/GRUB-Studio-Universal-v2.3.8-x86_64.AppImage)**

AppImage nie wymaga instalacji pakietu ani ingerencji w obraz systemu Bazzite. Po pobraniu nadaj mu uprawnienia wykonywania:

```bash
chmod +x GRUB-Studio-Universal-v2.3.8-x86_64.AppImage
./GRUB-Studio-Universal-v2.3.8-x86_64.AppImage
```

**[Pobierz sumę SHA-256](https://github.com/wolfgangxxiii/GRUB-Studio-Universal/releases/download/v2.3.8-rc.1/GRUB-Studio-Universal-v2.3.8-SHA256.txt)** · [Zobacz wydanie i zmiany](https://github.com/wolfgangxxiii/GRUB-Studio-Universal/releases/tag/v2.3.8-rc.1)

> **Wersja 2.3.8 RC:** Naprawiono zawieszanie panelu „Wygląd i język”: zrezygnowano z zagnieżdżonych list GTK, a odświeżenie ekranu następuje dopiero po zamknięciu menu. Dodano automatyczny test sześciu przełączeń wyglądu i języka. Pozostawiono stabilne systemowe dekoracje okna z 2.3.7, stałą nawigację i zabezpieczenia GRUB. Działanie na KDE/Wayland wymaga jeszcze testu na komputerze użytkownika.

Szczegóły projektu wyglądu i przypisania funkcji do kart: **[docs/DESIGN.md](docs/DESIGN.md)**.

### 🔐 Bezpieczeństwo i kompatybilność

- Aplikację uruchamia się jako **zwykły użytkownik**. O hasło administratora prosi systemowy **Polkit**; po odmowie pozostaje tryb tylko do odczytu.
- Program **nie edytuje ręcznie** `grub.cfg`: generuje go do pliku tymczasowego, sprawdza składnię i atomowo instaluje zatwierdzony wynik. Nie uruchamia `grub-install` ani nie zmienia kolejności UEFI.
- Plik `/etc/default/grub`, jeśli go nie ma, może zostać utworzony **dopiero po zatwierdzeniu zapisu** w obsługiwanej konfiguracji.
- Wykrywanie Windows w EFI potwierdza obecność wpisu lub pliku, **nie gwarantuje** poprawnego uruchomienia systemu.
- Na Bazzite/OSTree obowiązują dodatkowe ograniczenia; zmiany wyglądu GRUB nie są oficjalnie wspierane przez Bazzite.
- Przy aktywnym Limine, systemd-boot lub rEFInd aplikacja ogranicza operacje zapisu.
- **Przed zmianą konfiguracji rozruchu wykonaj kopię zapasową i przygotuj nośnik ratunkowy.**

### 🧰 Budowanie ze źródeł

<details>
<summary>Rozwiń instrukcję dla Rust + GTK4</summary>

W środowisku budowania (np. Fedora Toolbox) wymagane są Rust/Cargo, GTK4 development, GCC i `pkg-config`. Nie trzeba instalować tych narzędzi w podstawowym obrazie Bazzite.

```bash
sudo dnf install -y gcc pkgconf-pkg-config gtk4-devel
cargo test --all-targets
cargo build --release --bins
cargo run --bin grub-studio-universal -- --demo
GDK_BACKEND=x11 GSK_RENDERER=cairo xvfb-run -a target/release/grub-studio-universal --ui-smoke
GDK_BACKEND=x11 GSK_RENDERER=cairo xvfb-run -a target/release/grub-studio-universal --ui-smoke-wide
```

Skrypt `build-appimage.sh` wymaga dodatkowo `linuxdeploy`. Automatyczny workflow budujący i testujący AppImage znajduje się w [GitHub Actions](https://github.com/wolfgangxxiii/GRUB-Studio-Universal/actions).

</details>

### 🤝 Projekt

Masz pomysł lub znalazłeś błąd? Otwórz [Issues](https://github.com/wolfgangxxiii/GRUB-Studio-Universal/issues). Kod jest udostępniany na licencji **GPL-3.0-or-later**. To niezależny projekt, niezwiązany oficjalnie z Bazzite ani twórcami GRUB.

---

<a id="english"></a>

## 🇬🇧 English

**GRUB Studio Universal** is a graphical tool for managing the GRUB bootloader configuration. Built with **Rust** and **GTK4**, it features a compact, responsive interface designed for Wayland and different screen sizes.

### ✨ Highlights

| Area | Features |
| --- | --- |
| 🎨 **Interface** | **System-managed window decorations**, separate in-app toolbar, built-in monochrome vector icons, an always-visible 196/62 px sidebar, system cursor, light/dark modes and seven languages. |
| 🐧 **Distributions** | Detection of Bazzite, Fedora, Ubuntu/Debian, Arch/CachyOS, and openSUSE. |
| 🖥️ **Operating systems** | One combined GRUB/EFI discovery view, merging records with the same system name; optional `os-prober` scan. |
| 🖼️ **GRUB themes** | Installed `theme.txt` discovery, selection, active-theme status, import, and pinned Apply/Disable buttons even in compact windows. The inaccurate simulated preview is gone. |
| ⚙️ **Settings** | Menu timeout, localized menu styles, resolution presets, and supported boot options. Unsaved edits survive navigation, while unauthorized sessions show a read-only summary. |
| 📋 **Boot entries** | Clear boot-menu view, consolidated system discovery, drafts retained across refresh, and editing restricted to supported GRUB profiles. |
| 🧰 **Tools** | Backups and activity log consolidated into one destination. |
| 🛡️ **Safety** | Polkit authentication, distinct locked/read-only statuses, EFI path validation, backups, and checked temporary GRUB generation before atomic installation. |

### ⬇️ Download & run

**[Download GRUB Studio Universal 2.3.8 — AppImage (x86_64)](https://github.com/wolfgangxxiii/GRUB-Studio-Universal/releases/download/v2.3.8-rc.1/GRUB-Studio-Universal-v2.3.8-x86_64.AppImage)**

The portable AppImage does not require installing an application package or modifying the Bazzite base image:

```bash
chmod +x GRUB-Studio-Universal-v2.3.8-x86_64.AppImage
./GRUB-Studio-Universal-v2.3.8-x86_64.AppImage
```

**[Download SHA-256 checksum](https://github.com/wolfgangxxiii/GRUB-Studio-Universal/releases/download/v2.3.8-rc.1/GRUB-Studio-Universal-v2.3.8-SHA256.txt)** · [Release notes](https://github.com/wolfgangxxiii/GRUB-Studio-Universal/releases/tag/v2.3.8-rc.1)

> **Version 2.3.8 RC:** Fixes the appearance/language menu freeze by replacing nested GTK dropdown popovers with direct choices and deferring interface rebuilding until the menu has closed. Adds a six-change preferences regression test. Retains the system-managed window decorations, permanent sidebar, safe GRUB generation, and preserved form drafts. Real KDE/Wayland hardware testing is still needed; administrative writes depend on Polkit/`pkexec`.

More about the layout, one-place-per-action design and testing: **[docs/DESIGN.md](docs/DESIGN.md)**.

### 🔐 Safety & compatibility

- Run the GUI as a **regular user**. Administrative authorization is requested through **Polkit**, with a read-only mode when access is denied.
- The application generates `grub.cfg` into a temporary file, checks its syntax, then atomically replaces the generated configuration; it does not run `grub-install` or change UEFI boot order.
- If `/etc/default/grub` is missing, it may be created **only after an explicitly approved write** on a supported setup.
- A Windows EFI entry or file is **not proof** that Windows will successfully boot.
- Bazzite/OSTree systems receive additional safety restrictions; GRUB theme modifications are not officially supported by Bazzite.
- Write operations are restricted when Limine, systemd-boot, or rEFInd is the active bootloader.
- **Back up your boot configuration and keep a recovery USB drive ready before making changes.**

### 🧰 Build from source

<details>
<summary>Show Rust + GTK4 build instructions</summary>

The build environment needs Rust/Cargo, GTK4 development files, GCC, and `pkg-config`. A Fedora Toolbox can provide these without installing them into Bazzite's immutable base image.

```bash
sudo dnf install -y gcc pkgconf-pkg-config gtk4-devel
cargo test --all-targets
cargo build --release --bins
cargo run --bin grub-studio-universal -- --demo
GDK_BACKEND=x11 GSK_RENDERER=cairo xvfb-run -a target/release/grub-studio-universal --ui-smoke
GDK_BACKEND=x11 GSK_RENDERER=cairo xvfb-run -a target/release/grub-studio-universal --ui-smoke-wide
```

The `build-appimage.sh` script additionally requires `linuxdeploy`. Automated builds and AppImage runtime tests are available in [GitHub Actions](https://github.com/wolfgangxxiii/GRUB-Studio-Universal/actions).

</details>

### 🤝 Contributing

Found a bug or have an idea? Open an [issue](https://github.com/wolfgangxxiii/GRUB-Studio-Universal/issues). The source is licensed under **GPL-3.0-or-later**. This is an independent project and is not officially affiliated with Bazzite or the GRUB maintainers.

---

<p align="center">
  <a href="#polski">↑ Polski</a> &nbsp;·&nbsp; <a href="#english">↑ English</a> &nbsp;·&nbsp;
  <a href="https://github.com/wolfgangxxiii/GRUB-Studio-Universal/releases">Releases</a>
</p>
