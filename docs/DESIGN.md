# GRUB Studio Universal — projekt interfejsu 2.3.6

## Cel

Zwięzły, czytelny menedżer GRUB działający natywnie w GTK4 na Waylandzie i X11.
Interfejs jest inspirowany podziałem na kolekcję i szczegóły w Lutris, ale
pozostaje własnym projektem. Stosuje również zasadę hierarchii i ujawniania
szczegółów na żądanie z [GNOME HIG](https://developer.gnome.org/hig/).

## Struktura

- **Stała powłoka:** pasek górny, nawigacja i pasek stanu są tworzone raz.
  Zmiana karty zastępuje wyłącznie zawartość obszaru roboczego.
- **Tytuł okna:** standardowy `GtkHeaderBar` dostarcza kontrolki zamykania,
  minimalizacji i maksymalizacji, zastępując podwójny pasek widoczny w 2.3.4.
- **Nawigacja:** sześć sekcji, zawsze widocznych w panelu bocznym. Nie ma
  przycisku do chowania ani ponownego wyświetlania nawigacji. Panel ma
  szerokość 196 px w dużym oknie i 62 px z samymi ikonami poniżej 860 px.
  Długie nazwy nie zmieniają szerokości panelu; ikony mają podpowiedzi.
  Własne wektorowe ikony są rysowane w GTK4, więc motywy Breeze, Adwaita i inne
  nie powodują mieszania ikon monochromatycznych z kolorowymi.
- **Pasek górny:** identyfikacja aplikacji, stan uprawnień, odblokowanie przez
  Polkit, globalne odświeżenie i jedno menu preferencji (kolorystyka oraz język).
  Odblokowanie zastępuje powtarzający jego znaczenie znacznik blokady. Grafika
  programu jest osadzona w binarium, więc działa również w AppImage bez motywu ikon.
- **Obszar treści:** nagłówek karty, zwięzły opis i jeden poziom istotnych
  paneli. Zadania drugorzędne są składane w rozwijane sekcje.
- **Wykryte wpisy:** dane GRUB, UEFI/EFI i opcjonalnego `os-prober` są pokazane
  w jednym panelu, identyczne nazwy łączą źródła, a dokładny skan jest
  drugorzędną akcją. Tryb bez edycji nie nazywa już strony edytorem.
- **Kolory i kursor systemowy:** tryby systemowy/jasny/ciemny; motyw i wielkość
  kursora z sesji (`XCURSOR_*`) lub KDE Plasma (`kcminputrc`). Nie dołączamy
  do AppImage własnego pakietu kursorów.
- **Stany operacji:** brak autoryzacji jest odrębny od trybu tylko do odczytu.
  W trybach demonstracyjnym i tylko do odczytu formularze edycji ustępują
  podsumowaniu odczytanych danych, bez pozornie aktywnych pól zapisu.
- **Trwałość pracy w interfejsie:** niezapisane ustawienia GRUB i dane
  edytowanego wpisu EFI przechowuje stan aplikacji, a nie pojedyncza karta.
  Po zmianie karty, języka lub motywu wyglądu edycja jest nadal dostępna.

## Jedno miejsce dla każdej czynności

| Karta | Jedyny cel i powiązane działania |
| --- | --- |
| Pulpit | Status, aktywny motyw, liczba motywów i systemów, profil rozruchu. Bez powtórzonych przycisków nawigacji. |
| Menu GRUB | Własne wpisy EFI: dodaj, edytuj, zapisz, gdy instalacja wspiera edycję. Przesunięcie, ukrycie i usunięcie dostępne w kontekstowym menu. **Jeden widok wykrytych systemów i wpisów** łączy źródła GRUB i EFI, bez powtarzania tych samych nazw; opcjonalny `os-prober`. |
| Motywy | Wykrywanie zainstalowanych plików `theme.txt`, lista ze ścieżkami plików, import i stały dolny pasek „Zastosuj / Bez motywu”, który nie znika przy przewijaniu. Brak symulowanego menu GRUB. |
| Ustawienia | Czas menu, tryb, rozdzielczość, widoczność i obsługiwane parametry jądra. Bez drugiego selektora motywów. |
| Narzędzia | Tworzenie/przywracanie kopii oraz dziennik w oddzielnych rozwijanych panelach. |
| Informacje | Dokumentacja obsługi i bezpieczeństwa. |

**Skanowanie standardowe** ma jeden przycisk w górnym pasku i nie jest powielane
w kolejnych kartach. Odmowa Polkit nie blokuje przeglądania danych dostępnych
bez uprawnień. Funkcje zapisu GRUB zachowują walidację i kopię bezpieczeństwa.

## Zachowanie na mniejszym ekranie

- Minimalny obszar okna 620 × 440 px; pionowy przewijany widok karty.
- Poniżej 860 px panel boczny jest ikonowy, ale cały czas widoczny i zachowuje wskazówki po najechaniu.
- Edytor wpisów zawija przyciski do kolejnego wiersza.
- Na szerokim ekranie edytor ma dwie kolumny (lista wpisów i formularz);
  przy mniejszej szerokości kolumny ustawiają się jedna pod drugą.
- Biblioteka motywów jest jednokolumnowa: ścieżki są widoczne na liście,
  wybrany motyw i działania są w nieruchomym dolnym pasku, również przy 700 × 540 px.
- Preferencje wyglądu i języka nie zabierają osobnego miejsca w górnym pasku.
- Działania na zaznaczonym wpisie są dostępne w menu kontekstowym, a nie w sześciu równych przyciskach.
- Elementy nawigacji nie zwiększają szerokości pod wpływem tekstu innej karty.
- Każda nawigacja wywołuje tylko przebudowę aktywnej karty; powłoka jest stała.

## Weryfikacja

```bash
cargo test --all-targets
cargo build --release --bins
GDK_BACKEND=x11 GSK_RENDERER=cairo xvfb-run -a target/release/grub-studio-universal --ui-smoke
GDK_BACKEND=x11 GSK_RENDERER=cairo xvfb-run -a target/release/grub-studio-universal --ui-smoke-wide
```

Tryb `--ui-smoke` (bez operacji na GRUB) automatycznie otwiera wszystkie sześć
kart w oknie 720 px i sprawdza, że nawigacja pozostaje zwinięta (62 px).
`--ui-smoke-wide` sprawdza analogiczny układ przy 1100 px i panelu 196 px.
GitHub Actions uruchamia oba testy przed budową AppImage.
Osobny test `--ui-resize-report` wymusza rozmiar 720 px poleceniem `xdotool`
**po otwarciu okna** i sprawdza przejście z 196 do 62 px; dzięki temu test
obejmuje też menedżery okien niewysyłające powiadomień o zmianie szerokości.
Wersja 2.3.6 sprawdza obecność osadzonego PNG aplikacji, komplet komunikatów
uprawnień w siedmiu językach oraz zachowuje testy kursora i stałego paska akcji Motywów.

**Decyzja projektowa:** usunięto pozorny podgląd motywu. Sam odczyt
`desktop-image` nie wystarcza, aby wiarygodnie pokazać układ, czcionki,
grafikę i zachowanie GRUB. Prawdziwy wygląd należy zweryfikować w GRUB
podczas rozruchu, dlatego interfejs pokazuje tylko wykrywalne dane.

## Odniesienia projektowe

- [GNOME HIG — nawigacja](https://developer.gnome.org/hig/guidelines/navigation.html)
- [GNOME HIG — skalowanie interfejsu](https://developer.gnome.org/hig/guidelines/adaptive.html)
- [GNOME HIG — panele boczne](https://developer.gnome.org/hig/patterns/nav/sidebars.html)
- [Lutris — projekt Linux gaming](https://github.com/lutris/lutris)

To dokument projektu, a nie gwarancja obsługi wszystkich bootloaderów.
Zapis pozostaje ograniczony do wykrytych, obsługiwanych konfiguracji GRUB.
