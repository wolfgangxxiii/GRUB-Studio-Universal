<p align="center">
  <img src="assets/grub-studio.svg" alt="GRUB Studio Universal logo" width="100">
</p>

<h1 align="center">GRUB Studio Universal</h1>

<p align="center">
  <strong>A modern, portable GRUB configuration studio for Linux.</strong><br>
  Rust · GTK4 · libadwaita · Wayland · Light & Dark
</p>

<p align="center">
  <a href="#polski">🇵🇱 Polski</a> &nbsp;·&nbsp; <a href="#english">🇬🇧 English</a>
</p>

<p align="center">
  <a href="https://github.com/wolfgangxxiii/GRUB-Studio-Universal/actions/workflows/test-and-build.yml"><img src="https://github.com/wolfgangxxiii/GRUB-Studio-Universal/actions/workflows/test-and-build.yml/badge.svg" alt="Build status"></a>
  <img src="https://img.shields.io/badge/Rust-2021-CE422B?logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/GTK-4-4A86CF?logo=gtk&logoColor=white" alt="GTK4">
  <img src="https://img.shields.io/badge/libadwaita-native-3584E4?logo=gnome&logoColor=white" alt="libadwaita">
  <img src="https://img.shields.io/badge/Linux-x86__64-1793D1?logo=linux&logoColor=white" alt="Linux x86_64">
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-GPL--3.0--or--later-38A169" alt="GPL-3.0-or-later"></a>
</p>

<p align="center">
  <a href="https://github.com/wolfgangxxiii/GRUB-Studio-Universal/releases/download/v3.0.2-rc.1/GRUB-Studio-Universal-v3.0.2-x86_64.AppImage"><strong>⬇️ Download AppImage · v3.0.2 RC1</strong></a>
  &nbsp;·&nbsp;
  <a href="https://github.com/wolfgangxxiii/GRUB-Studio-Universal/releases">All releases</a>
</p>

<p align="center"><sub>Actual GTK4 interface screenshots are generated as CI artifacts, without storing historical images in the source repository.</sub></p>

---

<a id="polski"></a>

## 🇵🇱 Polski

**GRUB Studio Universal** to graficzne narzędzie do zarządzania konfiguracją programu rozruchowego GRUB. Powstało w **Rust**, korzysta z **GTK4 oraz libadwaita** i działa natywnie pod Waylandem oraz X11.

### ✨ Najważniejsze funkcje

| Obszar | Możliwości |
| --- | --- |
| 🎨 **Interfejs** | **Natywna stylistyka libadwaita** — tryb systemowy/jasny/ciemny, akcent systemowy (gdy obsługiwany), cztery równe kafelki na pulpicie, czytelne przyciski i rozwijane sekcje; stabilne dekoracje okna, zawsze widoczna nawigacja 196/62 px, siedem języków i kursor systemowy. |
| 🐧 **Dystrybucje** | Rozpoznawanie Bazzite, Fedora, Ubuntu/Debian, Arch/CachyOS i openSUSE. |
| 🖥️ **Systemy** | Wspólny widok informacji GRUB/EFI bez duplikowania systemów o tej samej nazwie; opcjonalny skan `os-prober`. |
| 🖼️ **Motywy GRUB** | Wykrywanie `theme.txt`, lista z lokalizacją plików, stan aktywności, import folderów i archiwów ZIP/TAR.GZ/TAR.XZ oraz przyciski zastosowania/wyłączenia dostępne nawet przy przewijaniu. Bez symulowanego podglądu. |
| ⚙️ **Ustawienia** | Czas wyświetlania menu, przetłumaczone style menu, lista rozdzielczości i parametry rozruchu. Niezapisane wartości formularza pozostają przy zmianie kart; bez prawa zapisu wyświetla się podsumowanie tylko do odczytu. |
| 📋 **Wpisy rozruchowe** | Prostsza karta „Menu rozruchowe” z czytelnym stanem blokady i jednym panelem wykrywania; edycja własnych EFI tylko na obsługiwanych konfiguracjach. Niezapisane dane pozostają także po ręcznym odświeżeniu. |
| 🧰 **Narzędzia** | Kopie zapasowe i dziennik działań dostępne razem bez duplikowania pozycji menu. |
| 🛡️ **Ochrona zmian** | Polkit, osobny status blokady i trybu tylko do odczytu, walidacja ścieżek EFI i kopie bezpieczeństwa. Generator tworzy plik GRUB osobno, sprawdza składnię i dopiero wtedy zastępuje konfigurację rozruchową. |

### 🎨 Skąd pobrać motywy GRUB?

- **[Gorgeous-GRUB](https://github.com/jacksaur/Gorgeous-GRUB)** — galeria motywów społecznościowych z odnośnikami do stron poszczególnych projektów.
- **[Distro GRUB Themes — podgląd i pobieranie](https://k1ng.dev/distro-grub-themes/preview)** — motywy dopasowane do wielu dystrybucji Linuksa.

**Importowanie w 3.0.2:** pobierz motyw i wybierz **Motywy → Importuj archiwum** dla `.zip`, `.tar.gz` lub `.tar.xz`. Program sam wyszuka pojedynczy `theme.txt`, także w podfolderze, **rozpakuje wyłącznie pliki motywu bez uprawnień administratora**, sprawdzi ich typy, rozmiary i ścieżki, a następnie pokaże nazwę i liczbę plików przed potwierdzeniem. Dopiero po potwierdzeniu i autoryzacji Polkit kopiuje motyw do katalogu GRUB. Istnieje też opcja **Importuj folder** dla już rozpakowanych motywów. Archiwa zawierające *wiele* motywów (`theme.txt`) należy rozpakować samodzielnie i wybrać konkretny folder. Archiwa z niedozwolonymi ścieżkami, dowiązaniami lub nadmiernym rozmiarem są odrzucane. Pliki instalatorów/skryptów są pomijane i **nigdy nie są wykonywane**. Import **nie włącza motywu automatycznie** — wybierz go i naciśnij „Zastosuj motyw”.

### ⬇️ Pobieranie i uruchamianie

**[Pobierz GRUB Studio Universal 3.0.2 — AppImage (x86_64)](https://github.com/wolfgangxxiii/GRUB-Studio-Universal/releases/download/v3.0.2-rc.1/GRUB-Studio-Universal-v3.0.2-x86_64.AppImage)**

AppImage nie wymaga instalacji pakietu ani ingerencji w obraz systemu Bazzite. Po pobraniu nadaj mu uprawnienia wykonywania:

```bash
chmod +x GRUB-Studio-Universal-v3.0.2-x86_64.AppImage
./GRUB-Studio-Universal-v3.0.2-x86_64.AppImage
```

**[Pobierz sumę SHA-256](https://github.com/wolfgangxxiii/GRUB-Studio-Universal/releases/download/v3.0.2-rc.1/GRUB-Studio-Universal-v3.0.2-SHA256.txt)** · [Zobacz wydanie i zmiany](https://github.com/wolfgangxxiii/GRUB-Studio-Universal/releases/tag/v3.0.2-rc.1)

> **Wersja 3.0.2 RC:** Bezpośredni import ZIP, TAR.GZ i TAR.XZ z wykrywaniem `theme.txt` i kontrolą bezpieczeństwa. Zachowano ulepszony pulpit, kontrast przycisków, **libadwaita**, stabilne menu języków/wyglądu oraz dotychczasowe ograniczenia uprawnień. Wymagany jest jeszcze test z zapisem motywu na rzeczywistym KDE/Wayland.

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

W środowisku budowania (np. Fedora Toolbox) wymagane są Rust/Cargo, pakiety deweloperskie GTK4 i libadwaita, GCC oraz `pkg-config`. Nie trzeba instalować tych narzędzi w podstawowym obrazie Bazzite.

```bash
sudo dnf install -y gcc pkgconf-pkg-config gtk4-devel
cargo test --all-targets
cargo build --release --bins
cargo run --bin grub-studio-universal -- --demo
GDK_BACKEND=x11 GSK_RENDERER=cairo xvfb-run -a target/release/grub-studio-universal --ui-smoke
GDK_BACKEND=x11 GSK_RENDERER=cairo xvfb-run -a target/release/grub-studio-universal --ui-smoke-wide
GDK_BACKEND=x11 GSK_RENDERER=cairo xvfb-run -a target/release/grub-studio-universal --ui-home-layout-smoke
```

Skrypt `build-appimage.sh` wymaga dodatkowo `linuxdeploy`. Automatyczny workflow budujący i testujący AppImage znajduje się w [GitHub Actions](https://github.com/wolfgangxxiii/GRUB-Studio-Universal/actions).

</details>

### 🤝 Projekt

Masz pomysł lub znalazłeś błąd? Otwórz [Issues](https://github.com/wolfgangxxiii/GRUB-Studio-Universal/issues). Kod jest udostępniany na licencji **GPL-3.0-or-later**. To niezależny projekt, niezwiązany oficjalnie z Bazzite ani twórcami GRUB.

---

<a id="english"></a>

## 🇬🇧 English

**GRUB Studio Universal** is a graphical tool for managing the GRUB bootloader configuration. Built with **Rust**, **GTK4** and **libadwaita**, it features a responsive native interface for Wayland and X11.

### ✨ Highlights

| Area | Features |
| --- | --- |
| 🎨 **Interface** | **Native libadwaita** surfaces, full-width equal dashboard tiles, stronger action/expander contrast, desktop accent and system/light/dark color scheme; system-managed window decorations, a fixed 196/62 px sidebar, system cursor and seven languages. |
| 🐧 **Distributions** | Detection of Bazzite, Fedora, Ubuntu/Debian, Arch/CachyOS, and openSUSE. |
| 🖥️ **Operating systems** | One combined GRUB/EFI discovery view, merging records with the same system name; optional `os-prober` scan. |
| 🖼️ **GRUB themes** | Installed `theme.txt` discovery, selection, active-theme status, folder/ZIP/TAR.GZ/TAR.XZ import and pinned Apply/Disable buttons even in compact windows. The inaccurate simulated preview is gone. |
| ⚙️ **Settings** | Menu timeout, localized menu styles, resolution presets, and supported boot options. Unsaved edits survive navigation, while unauthorized sessions show a read-only summary. |
| 📋 **Boot entries** | Clear boot-menu view, consolidated system discovery, drafts retained across refresh, and editing restricted to supported GRUB profiles. |
| 🧰 **Tools** | Backups and activity log consolidated into one destination. |
| 🛡️ **Safety** | Polkit authentication, distinct locked/read-only statuses, EFI path validation, backups, and checked temporary GRUB generation before atomic installation. |

### 🎨 Where to find GRUB themes

- **[Gorgeous-GRUB](https://github.com/jacksaur/Gorgeous-GRUB)** — a curated gallery of community themes with links to their original projects.
- **[Distro GRUB Themes — previews and downloads](https://k1ng.dev/distro-grub-themes/preview)** — themes for a variety of Linux distributions.

**Importing in 3.0.2:** download a theme and choose **Themes → Import archive** for ZIP, TAR.GZ or TAR.XZ. The app finds one `theme.txt` automatically, including nested folders, extracts the theme **as an unprivileged user** into private temporary storage, validates paths/file types/sizes, then shows its name and file count for confirmation. Polkit authorization is requested only after validation. **Import folder** remains available for extracted themes. If an archive contains *multiple* `theme.txt` files, extract it yourself and select the intended folder. Archives with unsafe paths, links, or excessive sizes are rejected; installers and scripts are skipped and **never executed**. Imported themes are **not activated** automatically — select the theme and click Apply theme.

### ⬇️ Download & run

**[Download GRUB Studio Universal 3.0.2 — AppImage (x86_64)](https://github.com/wolfgangxxiii/GRUB-Studio-Universal/releases/download/v3.0.2-rc.1/GRUB-Studio-Universal-v3.0.2-x86_64.AppImage)**

The portable AppImage does not require installing an application package or modifying the Bazzite base image:

```bash
chmod +x GRUB-Studio-Universal-v3.0.2-x86_64.AppImage
./GRUB-Studio-Universal-v3.0.2-x86_64.AppImage
```

**[Download SHA-256 checksum](https://github.com/wolfgangxxiii/GRUB-Studio-Universal/releases/download/v3.0.2-rc.1/GRUB-Studio-Universal-v3.0.2-SHA256.txt)** · [Release notes](https://github.com/wolfgangxxiii/GRUB-Studio-Universal/releases/tag/v3.0.2-rc.1)

> **Version 3.0.2 RC:** Direct ZIP, TAR.GZ and TAR.XZ theme import with automatic `theme.txt` discovery and safety validation. The improved dashboard, native **libadwaita** appearance and responsive language/color preferences remain unchanged. Real KDE/Wayland theme installation still requires on-device validation.

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

The build environment needs Rust/Cargo, GTK4 and libadwaita development files, GCC, and `pkg-config`. A Fedora Toolbox can provide these without installing them into Bazzite's immutable base image.

```bash
sudo dnf install -y gcc pkgconf-pkg-config gtk4-devel
cargo test --all-targets
cargo build --release --bins
cargo run --bin grub-studio-universal -- --demo
GDK_BACKEND=x11 GSK_RENDERER=cairo xvfb-run -a target/release/grub-studio-universal --ui-smoke
GDK_BACKEND=x11 GSK_RENDERER=cairo xvfb-run -a target/release/grub-studio-universal --ui-smoke-wide
GDK_BACKEND=x11 GSK_RENDERER=cairo xvfb-run -a target/release/grub-studio-universal --ui-home-layout-smoke
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
