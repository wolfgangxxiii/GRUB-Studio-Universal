# GRUB Studio Universal 2.1.0 RC1 — raport jakości

Data: 13 listopada 2026

## Automatyczne testy

| Kontrola | Wynik | Środowisko |
| --- | --- | --- |
| `cargo test --all-targets` | **PASS** — 10 testów, zero błędów | Ubuntu / GitHub Actions |
| `cargo build --release --bins` | **PASS** | Rust stable + GTK4 |
| Osiem zakładek GTK4 i stała szerokość nawigacji 196 px | **PASS** | `xvfb-run`, okno 720 × 540 |
| Uruchomienie w trybie demonstracyjnym (ciemny i jasny) | **PASS** | GTK4 / wirtualny ekran X11 |
| Zrzuty rzeczywistego interfejsu | **PASS** | `preview/GRUB-Studio-2.1-GTK4-{Dark,Light}.png` |
| `build-appimage.sh`, tworzenie AppDir i AppImage | **PASS** | linuxdeploy, x86_64 |
| Test `AppImage --version` i `AppImage --self-test` | **PASS** | GitHub Actions |
| Wykrywanie zainstalowanych motywów i Windows EFI | **PASS** — testy jednostkowe na danych testowych | Rust |

Przed włączeniem finalnych zmian wizualnych kompilacja kontrolna zakończyła się
powodzeniem: [GitHub Actions — test projektu 2.1](https://github.com/wolfgangxxiii/GRUB-Studio-Universal/actions/runs/43474357857).
Wersja wydana na `main` przechodzi ten sam zestaw kontroli; jej wynik jest
widoczny na stronie [Actions](https://github.com/wolfgangxxiii/GRUB-Studio-Universal/actions).

## Sprawdzenie interfejsu

- Powłoka okna, menu, pasek górny i pasek stanu są tworzone jeden raz.
- Szerokość nawigacji nie zależy od wybranej zakładki i jej treści.
- Wyrzucono powtórzone czynności: wybór motywu tylko w **Motywach**,
  odświeżenie podstawowe tylko w pasku górnym, bez duplikatów na pulpicie.
- Przyciski edytora niestandardowych wpisów EFI zawijają się w małym oknie.
- Widok biblioteki motywów przechodzi między jedną a dwiema kolumnami.
- Zestawienie prawdziwych zrzutów z obu palet pokazało i pozwoliło poprawić
  kontrast etykiet listy i wygląd rozwijanego wyboru języka.

## Co wymaga próby na sprzęcie użytkownika

Automatyczna kompilacja **nie jest** testem modyfikacji działającego bootloadera.
Następujące rzeczy pozostają do weryfikacji na prawdziwym komputerze, najlepiej
we własnym środowisku testowym lub maszynie wirtualnej:

1. Polkit i wyświetlenie monitu hasła w KDE Plasma/Wayland oraz GNOME/Wayland.
2. Montowanie i modyfikowanie `/boot` na konkretnej wersji Bazzite OSTree.
3. Zapis, tworzenie i odtwarzanie `/etc/default/grub` oraz rollback po awarii.
4. Import motywu, wykonanie `grub2-mkconfig` i ponowny rozruch w trybie UEFI.
5. Renderowanie na Wayland bez XWayland, przy skalowaniu 125–200%.
6. Praca AppImage na innych dystrybucjach i starszych bibliotekach systemowych.

**Wniosek:** przeszły testy kodu, nawigacji GTK4 i przenośnego artefaktu AppImage.
Operacje zmieniające program rozruchowy wymagają nadal ręcznego testu regresji
na wspieranych systemach. Aplikacja nie zastępuje kopii zapasowej i nośnika ratunkowego.
