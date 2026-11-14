# GRUB Studio Universal 2.3.1 RC1 — raport jakości / QA

Data: 14 listopada 2026

## Zakres przebudowy

- **Sześć stron**: Przegląd, Menu rozruchowe, Motywy, Ustawienia GRUB, Narzędzia, O programie i bezpieczeństwo.
- Wykrywanie systemów jest rozwijaną sekcją w **Menu rozruchowym**; kopie zapasowe i dziennik działań są połączone w **Narzędziach**.
- Boczne menu ma stabilną szerokość **196 px** w szerokim oknie i **62 px** w wąskim, z podpowiedziami po ukryciu podpisów. Rozmiar nie zmienia się przy przełączaniu stron.
- Karta Motywy wykrywa katalogi `theme.txt`, pokazuje zainstalowane motywy i stan aktywności. Pełna ścieżka zaznaczonego pliku jest od razu widoczna w sekcji szczegółów.
- Lista motywów mieści kilka wierszy i ma przewijanie dla większych kolekcji.
- Edytor EFI ma układ lista/formularz w dwóch kolumnach, które układają się pionowo na wąskim ekranie. Operacje kontekstowe są schowane w menu.
- Ustawienia wyglądu i języka są połączone w jednym menu. Nieuprawniony stan pokazuje jeden przycisk odblokowania, bez powtórzonego znacznika.
- Usunięto pozorny podgląd menu GRUB i odczytywanie tła `desktop-image`; wcześniej pokazywał symulację niezależną od faktycznego `theme.txt`.

## Kontrole automatyczne

| Kontrola | Oczekiwany wynik / sposób sprawdzenia |
| --- | --- |
| `cargo test --all-targets` | Testy biblioteki + kontrola tłumaczeń kluczowych elementów w siedmiu językach |
| `cargo build --release --bins` | Kompilacja aplikacji GTK4 i modułu uprzywilejowanego |
| `--ui-smoke` w `xvfb-run` | 6 stron, nawigacja **62 px** w oknie 720 × 540 px |
| `--ui-smoke-wide` w `xvfb-run` | 6 stron, nawigacja **196 px** w oknie 1100 × 540 px |
| `--ui-resize-report` + `xdotool` | Zwinięcie panelu z **196 px do 62 px**, gdy już otwarte okno zmniejszy się do 720 px |
| Zrzuty GTK4 | Motywy (ciemny i jasny), Menu (ciemny), Narzędzia (jasny), Menu kompaktowe (polski) |
| `build-appimage.sh` | Wytworzenie przenośnego AppImage x86_64 |
| `AppImage --version` i `AppImage --self-test` | Sprawdzenie pliku wynikowego |

Wynik 2.3.1 po usunięciu podglądu jest widoczny w [GitHub Actions](https://github.com/wolfgangxxiii/GRUB-Studio-Universal/actions). Testy nie zapisują zmian do bootloadera.

## Zrzuty z rzeczywistej aplikacji

Zrzuty pochodzą z procesu GTK4 w wirtualnym środowisku X11; nie są renderami marketingowymi:

- [Motywy — ciemny](preview/GRUB-Studio-2.3.1-Themes-Dark.png)
- [Motywy — jasny](preview/GRUB-Studio-2.3.1-Themes-Light.png)
- [Menu — ciemny](preview/GRUB-Studio-2.3-Menu-Dark.png)
- [Narzędzia — jasny](preview/GRUB-Studio-2.3-Tools-Light.png)
- [Menu — kompaktowy układ po polsku](preview/GRUB-Studio-2.3-Menu-Compact-PL.png)

## Ograniczenia weryfikacji

Wirtualny ekran X11 umożliwia porównanie geometrii kart i podstawowych interakcji, lecz **nie zastępuje testu na rzeczywistym komputerze**. W szczególności pozostają do ręcznego sprawdzenia:

1. Graficzny monit Polkit z hasłem administratora przy uruchomieniu na KDE/Wayland i GNOME/Wayland.
2. Odczyt zasobów `/boot` przy uprawnieniach i etykietach SELinux Bazzite.
3. Kopia zapasowa, odtworzenie oraz modyfikacja `/etc/default/grub` na rzeczywistej instalacji.
4. Aktualizacja konfiguracji GRUB i ponowny rozruch na systemach UEFI z Windows.
5. Skalowanie HiDPI 125–200%, obsługa dotyku i różne kompozytory Wayland.
6. Uruchomienie AppImage na dystrybucjach innych niż obraz systemu kompilującego.

Przed zmianami bootloadera użytkownik powinien mieć dostęp do nośnika ratunkowego oraz kopii zapasowej. **Automatyczny PASS nie oznacza przetestowania zapisu GRUB na sprzęcie.**
