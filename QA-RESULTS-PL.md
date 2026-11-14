# GRUB Studio Universal 2.3.5 RC1 — raport jakości / QA

Data: 14 listopada 2026

## Zakres przebudowy

- **Sześć stron**: Przegląd, Menu rozruchowe, Motywy, Ustawienia GRUB, Narzędzia, O programie i bezpieczeństwo.
- Wykryte wpisy GRUB i EFI są połączone w **jednym panelu Menu rozruchowego**. Identyczne nazwy systemów nie są powtarzane; ich źródła są widoczne razem. Kopie zapasowe i dziennik działań pozostają w Narzędziach.
- Podwójny nagłówek zastąpił natywny `GtkHeaderBar` z systemowymi przyciskami okna i rzeczywistą grafiką programu.
- Ikony bocznej nawigacji są rysowanymi w aplikacji monochromatycznymi wektorami; nie zmieniają stylu na kolorowe ikony zależnie od dystrybucji.
- Boczne menu jest **zawsze widoczne**. Usunięto przycisk chowania/ponownego pokazywania panelu. Szerokość **196 px** w szerokim oknie i **62 px** w wąskim; ikony kompaktowe mają podpowiedzi. Panel nie zmienia rozmiaru przy przełączaniu stron.
- Karta Motywy wykrywa katalogi `theme.txt`, pokazuje zainstalowane motywy i stan aktywności. Ścieżki widać bezpośrednio na liście, a operacje pozostają w nieruchomym dolnym pasku.
- Lista motywów mieści kilka wierszy i ma przewijanie dla większych kolekcji.
- Edytor EFI ma układ lista/formularz w dwóch kolumnach, które układają się pionowo na wąskim ekranie. Operacje kontekstowe są schowane w menu.
- Przełączanie kart, języka lub wyglądu zachowuje niezapisane wartości ustawień GRUB i formularza wpisów EFI. Tryb demonstracyjny i brak uprawnień pokazują informacje bez formularzy sugerujących możliwość zapisu.
- Ustawienia wyglądu (systemowy/jasny/ciemny) i języka są połączone w jednym menu. Kursor GTK4 używa ustawień sesji lub KDE Plasma i nie dostaje własnego motywu z AppImage.
- W menu preferencji dodano nagłówek, rozdzielacz i ujednolicono pola. Zwiększono rozmiar opisów, ostrzeżeń i podpisów nawigacji.
- Tymczasową literę G w nagłówku zastąpiła rzeczywista grafika aplikacji osadzona bezpośrednio w binarium; przyciski i pola niedostępne bez uprawnień są wyszarzone.
- Przed autoryzacją program pokazuje stan „Zablokowany” zamiast mylącego „Tylko odczyt”.
- Usunięto pozorny podgląd menu GRUB i odczytywanie tła `desktop-image`; wcześniej pokazywał symulację niezależną od faktycznego `theme.txt`.

## Kontrole automatyczne

| Kontrola | Oczekiwany wynik / sposób sprawdzenia |
| --- | --- |
| `cargo test --all-targets` | Testy biblioteki + konfiguracji kursora KDE, obecności PNG programu i tłumaczeń w siedmiu językach |
| Łączenie wpisów GRUB/EFI | Ten sam system zgłoszony z GRUB i UEFI wyświetla się jednokrotnie, z obiema informacjami źródłowymi |
| `cargo build --release --bins` | Kompilacja aplikacji GTK4 i modułu uprzywilejowanego |
| `--ui-smoke` w `xvfb-run` | 6 stron, **widoczna nawigacja 62 px**, brak przycisku jej ukrywania i dolny pasek działań wyłącznie na karcie Motywy; test motywu/rozmiaru kursora |
| `--ui-smoke-wide` w `xvfb-run` | 6 stron, nawigacja **196 px** w oknie 1100 × 540 px |
| `--ui-resize-report` + `xdotool` | Automatyczne przejście z panelu z podpisami **196 px** do ikonek **62 px** po zmianie szerokości okna na 720 px; panel cały czas pozostaje widoczny |
| Zrzuty GTK4 | Motywy (ciemny, jasny i kompaktowy polski), Menu (ciemny i kompaktowy polski), Narzędzia (jasny) |
| `build-appimage.sh` | Wytworzenie przenośnego AppImage x86_64 |
| `AppImage --version` i `AppImage --self-test` | Sprawdzenie pliku wynikowego |

Raport dla wersji 2.3.5 opiera się na [GitHub Actions](https://github.com/wolfgangxxiii/GRUB-Studio-Universal/actions). Testy nie zapisują zmian do bootloadera. Test wyświetlania rzeczywistego kursora KDE/Wayland wymaga komputera użytkownika.

## Zrzuty z rzeczywistej aplikacji

Zrzuty pochodzą z procesu GTK4 w wirtualnym środowisku X11; nie są renderami marketingowymi:

- [Motywy — ciemny](preview/GRUB-Studio-2.3.5-Themes-Dark.png)
- [Motywy — jasny](preview/GRUB-Studio-2.3.5-Themes-Light.png)
- [Motywy — kompaktowy układ po polsku](preview/GRUB-Studio-2.3.5-Themes-Compact-PL.png)
- [Menu — ciemny](preview/GRUB-Studio-2.3.5-Menu-Dark.png)
- [Narzędzia — jasny](preview/GRUB-Studio-2.3.5-Tools-Light.png)
- [Menu — kompaktowy układ po polsku](preview/GRUB-Studio-2.3.5-Menu-Compact-PL.png)

## Ograniczenia weryfikacji

Wirtualny ekran X11 umożliwia porównanie geometrii kart i podstawowych interakcji, lecz **nie zastępuje testu na rzeczywistym komputerze**. Szczególnie nowe dekoracje `GtkHeaderBar` należy sprawdzić na KDE Plasma / Wayland. Pozostają także do ręcznego sprawdzenia:

1. Graficzny monit Polkit z hasłem administratora przy uruchomieniu na KDE/Wayland i GNOME/Wayland.
2. Odczyt zasobów `/boot` przy uprawnieniach i etykietach SELinux Bazzite.
3. Kopia zapasowa, odtworzenie oraz modyfikacja `/etc/default/grub` na rzeczywistej instalacji.
4. Aktualizacja konfiguracji GRUB i ponowny rozruch na systemach UEFI z Windows.
5. Skalowanie HiDPI 125–200%, obsługa dotyku, różne kompozytory Wayland oraz fizyczny wygląd wskaźnika KDE/Wayland.
6. Uruchomienie AppImage na dystrybucjach innych niż obraz systemu kompilującego.

Przed zmianami bootloadera użytkownik powinien mieć dostęp do nośnika ratunkowego oraz kopii zapasowej. **Automatyczny PASS nie oznacza przetestowania zapisu GRUB na sprzęcie.**
