# GRUB Studio Universal 2.3.0 RC1 — raport jakości / QA

Data: 13 listopada 2026

## Zakres przebudowy

- **Sześć stron**: Przegląd, Menu rozruchowe, Motywy, Ustawienia GRUB, Narzędzia, O programie i bezpieczeństwo.
- Wykrywanie systemów jest rozwijaną sekcją w **Menu rozruchowym**; kopie zapasowe i dziennik działań są połączone w **Narzędziach**.
- Boczne menu ma stabilną szerokość **196 px** w szerokim oknie i **62 px** w wąskim, z podpowiedziami po ukryciu podpisów. Rozmiar nie zmienia się przy przełączaniu stron.
- Karta Motywy wykrywa katalogi `theme.txt`, wyświetla lokalne miniatury tapety `desktop-image`, jeśli są bezpiecznie dostępne, i umożliwia wybór wykrytego motywu. Większy podgląd ma oznaczenie aktywnej pozycji, a ścieżki znajdują się w rozwijanych szczegółach.
- Lista motywów mieści kilka wierszy i ma przewijanie dla większych kolekcji.
- Edytor EFI ma układ lista/formularz w dwóch kolumnach, które układają się pionowo na wąskim ekranie. Operacje kontekstowe są schowane w menu.
- Ustawienia wyglądu i języka są połączone w jednym menu. Nieuprawniony stan pokazuje jeden przycisk odblokowania, bez powtórzonego znacznika.
- Przegląd menu GRUB pozostaje **przybliżony**: obraz tła można pokazać, ale GTK nie renderuje skryptu `theme.txt` tak jak GRUB podczas startu systemu.

## Kontrole automatyczne

| Kontrola | Oczekiwany wynik / sposób sprawdzenia |
| --- | --- |
| `cargo test --all-targets` | 11 testów biblioteki + 1 test pełnego tłumaczenia nowych elementów w siedmiu językach |
| `cargo build --release --bins` | Kompilacja aplikacji GTK4 i modułu uprzywilejowanego |
| `--ui-smoke` w `xvfb-run` | 6 stron, nawigacja **62 px** w oknie 720 × 540 px |
| `--ui-smoke-wide` w `xvfb-run` | 6 stron, nawigacja **196 px** w oknie 1100 × 540 px |
| `--ui-resize-report` + `xdotool` | Zwinięcie panelu z **196 px do 62 px**, gdy już otwarte okno zmniejszy się do 720 px |
| Zrzuty GTK4 | Motywy (ciemny i jasny), Menu (ciemny), Narzędzia (jasny), Menu kompaktowe (polski) |
| `build-appimage.sh` | Wytworzenie przenośnego AppImage x86_64 |
| `AppImage --version` i `AppImage --self-test` | Sprawdzenie pliku wynikowego |

Pierwszy przebieg 2.3: [GitHub Actions #43514921581](https://github.com/wolfgangxxiii/GRUB-Studio-Universal/actions/runs/43514921581) — testy Rust, nawigacja, kompilacja oraz AppImage zaliczone. Zrzut wąskiego okna wykazał, że przy zmianie rozmiaru już otwartego okna panel pozostawał rozwinięty; dlatego dodano niezależny test przejścia na żywo. Wynik po poprawce jest widoczny w [GitHub Actions](https://github.com/wolfgangxxiii/GRUB-Studio-Universal/actions).

## Zrzuty z rzeczywistej aplikacji

Zrzuty pochodzą z procesu GTK4 w wirtualnym środowisku X11; nie są renderami marketingowymi:

- [Motywy — ciemny](preview/GRUB-Studio-2.3-GTK4-Dark.png)
- [Motywy — jasny](preview/GRUB-Studio-2.3-GTK4-Light.png)
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
