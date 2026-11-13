# GRUB Studio Universal 2.2.0 — raport jakości / QA

Data: 13 listopada 2026

## Zakres przebudowy

- **Sześć stron**: Przegląd, Menu rozruchowe, Motywy, Ustawienia GRUB, Narzędzia, O programie i bezpieczeństwo.
- Wykrywanie systemów jest rozwijaną sekcją w **Menu rozruchowym**; kopie zapasowe i dziennik działań są połączone w **Narzędziach**.
- Boczne menu ma stabilną szerokość **196 px** w szerokim oknie i **62 px** w wąskim, z podpowiedziami po ukryciu podpisów. Rozmiar nie zmienia się przy przełączaniu stron.
- Karta Motywy wykrywa katalogi `theme.txt`, wyświetla lokalne miniatury tapety `desktop-image`, jeśli są bezpiecznie dostępne, i umożliwia wybór wykrytego motywu.
- Lista motywów mieści kilka wierszy i ma przewijanie dla większych kolekcji.
- Przegląd menu GRUB pozostaje **przybliżony**: obraz tła można pokazać, ale GTK nie renderuje skryptu `theme.txt` tak jak GRUB podczas startu systemu.

## Kontrole automatyczne

| Kontrola | Oczekiwany wynik / sposób sprawdzenia |
| --- | --- |
| `cargo test --all-targets` | Wszystkie testy Rust zakończone sukcesem, w tym walidacja ścieżek tapet i motywów |
| `cargo build --release --bins` | Kompilacja aplikacji GTK4 i modułu uprzywilejowanego |
| `--ui-smoke` w `xvfb-run` | 6 stron, nawigacja **62 px** w oknie 720 × 540 px |
| `--ui-smoke-wide` w `xvfb-run` | 6 stron, nawigacja **196 px** w oknie 1100 × 540 px |
| Zrzuty GTK4 | Motywy (ciemny i jasny), Menu (ciemny), Narzędzia (jasny) |
| `build-appimage.sh` | Wytworzenie przenośnego AppImage x86_64 |
| `AppImage --version` i `AppImage --self-test` | Sprawdzenie pliku wynikowego |

Przebieg poprzedzający końcową korektę listy motywów: [GitHub Actions #43510267031](https://github.com/wolfgangxxiii/GRUB-Studio-Universal/actions/runs/43510267031) — wszystkie etapy zakończone sukcesem, w tym testy Rust, interfejsu i AppImage. Bieżące wyniki każdej późniejszej kompilacji są dostępne w [GitHub Actions](https://github.com/wolfgangxxiii/GRUB-Studio-Universal/actions).

## Zrzuty z rzeczywistej aplikacji

Zrzuty pochodzą z procesu GTK4 w wirtualnym środowisku X11; nie są renderami marketingowymi:

- [Motywy — ciemny](preview/GRUB-Studio-2.2-GTK4-Dark.png)
- [Motywy — jasny](preview/GRUB-Studio-2.2-GTK4-Light.png)
- [Menu — ciemny](preview/GRUB-Studio-2.2-Menu-Dark.png)
- [Narzędzia — jasny](preview/GRUB-Studio-2.2-Tools-Light.png)

## Ograniczenia weryfikacji

Wirtualny ekran X11 umożliwia porównanie geometrii kart i podstawowych interakcji, lecz **nie zastępuje testu na rzeczywistym komputerze**. W szczególności pozostają do ręcznego sprawdzenia:

1. Graficzny monit Polkit z hasłem administratora przy uruchomieniu na KDE/Wayland i GNOME/Wayland.
2. Odczyt zasobów `/boot` przy uprawnieniach i etykietach SELinux Bazzite.
3. Kopia zapasowa, odtworzenie oraz modyfikacja `/etc/default/grub` na rzeczywistej instalacji.
4. Aktualizacja konfiguracji GRUB i ponowny rozruch na systemach UEFI z Windows.
5. Skalowanie HiDPI 125–200%, obsługa dotyku i różne kompozytory Wayland.
6. Uruchomienie AppImage na dystrybucjach innych niż obraz systemu kompilującego.

Przed zmianami bootloadera użytkownik powinien mieć dostęp do nośnika ratunkowego oraz kopii zapasowej. **Automatyczny PASS nie oznacza przetestowania zapisu GRUB na sprzęcie.**
