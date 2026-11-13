# GRUB Studio Universal — projekt interfejsu 2.1

## Cel

Zwięzły, czytelny menedżer GRUB działający natywnie w GTK4 na Waylandzie i X11.
Interfejs jest inspirowany podziałem na kolekcję i szczegóły w Lutris, ale
pozostaje własnym projektem. Stosuje również zasadę hierarchii i ujawniania
szczegółów na żądanie z [GNOME HIG](https://developer.gnome.org/hig/).

## Struktura

- **Stała powłoka:** pasek górny, nawigacja i pasek stanu są tworzone raz.
  Zmiana karty zastępuje wyłącznie zawartość obszaru roboczego.
- **Nawigacja:** panel o szerokości 196 px, dwie grupy (GRUB, Narzędzia),
  identyczny zestaw kart i kolejność na każdej podstronie. Długie nazwy są
  skracane wielokropkiem zamiast poszerzać panel.
- **Pasek górny:** identyfikacja aplikacji, stan uprawnień, odblokowanie przez
  Polkit, globalne odświeżenie, przełączanie kolorystyki oraz języka.
- **Obszar treści:** nagłówek karty, zwięzły opis i jeden poziom istotnych
  paneli. Zadania drugorzędne są składane w rozwijane sekcje.
- **Ciemny i jasny motyw:** wspólny akcent niebieski, spokojne tło oraz wyraźny
  kontrast tekstu i pól formularzy.

## Jedno miejsce dla każdej czynności

| Karta | Jedyny cel i powiązane działania |
| --- | --- |
| Pulpit | Status, aktywny motyw, liczba motywów i systemów, profil rozruchu. Bez powtórzonych przycisków nawigacji. |
| Menu GRUB | Własne wpisy EFI: dodaj, edytuj, przenieś, ukryj, usuń, zapisz. Wygenerowane wpisy są tylko do podglądu. |
| Systemy | Wykryte systemy; opcjonalnie głębokie skanowanie `os-prober`. |
| Motywy | Lista instalacji z `theme.txt`, wybór, import folderu, zastosowanie lub wyłączenie motywu. |
| Ustawienia | Czas menu, tryb, rozdzielczość, widoczność i obsługiwane parametry jądra. Bez drugiego selektora motywów. |
| Kopie | Tworzenie i przywracanie kopii. |
| Dziennik | Informacje o działaniach i kopiowanie tekstu. |
| Informacje | Dokumentacja obsługi i bezpieczeństwa. |

**Skanowanie standardowe** ma jeden przycisk w górnym pasku i nie jest powielane
w kolejnych kartach. Odmowa Polkit nie blokuje przeglądania danych dostępnych
bez uprawnień. Funkcje zapisu GRUB zachowują walidację i kopię bezpieczeństwa.

## Zachowanie na mniejszym ekranie

- Minimalny obszar okna 620 × 440 px; pionowy przewijany widok karty.
- Edytor wpisów zawija przyciski do kolejnego wiersza.
- Biblioteka motywów przechodzi z dwóch kolumn do jednej, gdy brak miejsca.
- Elementy nawigacji nie zwiększają szerokości pod wpływem tekstu innej karty.
- Każda nawigacja wywołuje tylko przebudowę aktywnej karty; powłoka jest stała.

## Weryfikacja

```bash
cargo test --all-targets
cargo build --release --bins
GDK_BACKEND=x11 GSK_RENDERER=cairo xvfb-run -a target/release/grub-studio-universal --ui-smoke
```

Tryb `--ui-smoke` (bez operacji na GRUB) automatycznie otwiera wszystkie osiem
kart w oknie szerokości 720 px i porównuje rzeczywistą szerokość nawigacji
po każdym przejściu. GitHub Actions uruchamia ten test przed budową AppImage.

## Odniesienia projektowe

- [GNOME HIG — nawigacja](https://developer.gnome.org/hig/guidelines/navigation.html)
- [GNOME HIG — skalowanie interfejsu](https://developer.gnome.org/hig/guidelines/adaptive.html)
- [GNOME HIG — panele boczne](https://developer.gnome.org/hig/patterns/nav/sidebars.html)
- [Lutris — projekt Linux gaming](https://github.com/lutris/lutris)

To dokument projektu, a nie gwarancja obsługi wszystkich bootloaderów.
Zapis pozostaje ograniczony do wykrytych, obsługiwanych konfiguracji GRUB.
