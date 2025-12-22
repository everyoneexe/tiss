# ii-greetd-qml — DEV Plan

Amaç: `greetd` için **grafik (Wayland) greeter** yazmak ve UI’yi **Qt6/QML** ile kurmak. Böylece SDDM yerine `greetd` kullanırken “ii-niri lockscreen hissi” veren bir giriş ekranı elde edilir.

> Not: ii-niri içindeki mevcut lockscreen QML’i (Quickshell) “greetd greeter” olarak direkt çalışmaz; çünkü greeter farklı bir yaşam döngüsünde (TTY/seat) çalışır ve **greetd IPC** üzerinden kimlik doğrulama + session başlatma yapmalıdır. Bu yüzden ayrı bir uygulama/protokol yazıyoruz.

---

## 0) Hedefler / Non-goals

### Hedefler
- `greetd` ile uyumlu greeter: kullanıcı adı + parola al, doğrula, session başlat.
- UI: Qt6 + QML (Wayland).
- Varsayılan session: `niri` (opsiyonel olarak seçim).
- “ii-niri lockscreen”e benzer görünüm (renk/typography/blur/clock vs.).
- Güvenlik: parola hiçbir log’a düşmez, bellek temizliği, başarısız denemelerde geri bildirim.

### Non-goals
- SDDM’nin tüm özellikleri (çoklu kullanıcı listeleri, çoklu ekran, network login vb.) ilk aşamada yok.
- Mevcut ii-niri lockscreen’inin Quickshell bağımlılıklarını olduğu gibi taşımak yok; gerekirse UI parçası yeniden yazılır/ayrıştırılır.

---

## 1) Mimari Karar (kritik)

### Neden tek proses zor?
- `greetd` greeter protokolü, Rust ekosisteminde `greetd_ipc` ile kolay; C++/Qt tarafında yeniden implement etmek zor.
- QML UI ise Qt tarafında en rahat.

### Önerilen mimari (2 proses)
1) **Backend (Rust)**: `ii-greetd-backend`
   - `GREETD_SOCK` üzerinden greetd’ye bağlanır.
   - UI’den gelen istekleri alır (JSON üzerinden stdin/stdout veya Unix socket).
   - greetd’ye `create_session` / `post_auth_message` / `start_session` akışını yürütür.

2) **UI (Qt6/QML)**: `ii-greetd-ui`
   - Sadece arayüz + input + state.
   - Backend ile konuşur (JSON message protocol).

### UI’yi Wayland’da nasıl çalıştıracağız?
- Greeter TTY’de çalışır; Qt6 Wayland client olması için bir compositor gerekir.
- Pratik çözüm: **`cage`** (kiosk compositor) ile UI’yi çalıştırmak:
  - greetd `default_session.command`: `cage -s -- ii-greetd-backend --spawn-ui`

---

## 2) Protokol Tasarımı (UI <-> Backend)

Basit ve denetlenebilir bir **JSON Lines** protokolü (satır başına 1 JSON) önerisi.

### UI -> Backend
- `{ "type": "hello", "ui_version": 1 }`
- `{ "type": "auth", "username": "…", "password": "…" }`
- `{ "type": "start", "command": ["niri"], "env": {…} }`
- `{ "type": "power", "action": "reboot"|"poweroff" }` (opsiyonel)

### Backend -> UI
- `{ "type": "state", "phase": "idle"|"authenticating"|"failed"|"starting" }`
- `{ "type": "error", "message": "…" }`
- `{ "type": "success" }`

Güvenlik: password alanı asla loglanmaz; UI tarafında mümkünse submit sonrası alan temizlenir.

---

## 3) greetd Akışı (Backend)

Minimum akış:
1) greetd’ye bağlan.
2) `create_session { username }`
3) PAM konuşması gerekiyorsa greetd’nin istediği prompt’lara yanıt ver:
   - prompt türleri/parola prompt’unu yakala, UI’den gelen password ile `post_auth_message`.
4) Başarılıysa `start_session` (ör. `niri`).

Not: Bu kısım greetd’nin mevcut API’sine göre uygulanacak; en doğru referans `greetd_ipc` crate örnekleri.

---

## 4) UI Planı (QML)

### MVP ekranları
- Saat + tarih + arkaplan (statik/blur opsiyonel)
- Username input (opsiyonel: tek kullanıcı ise gizli)
- Password input
- Hata mesajı / “Caps lock açık” gibi ipuçları (opsiyonel)
- “Restart/Poweroff” (opsiyonel, yetki modeline dikkat)

### ii-niri lockscreen ile görsel yakınlık
- Renkler: mevcut ii-niri tema değişkenlerini kopyala/uyarla.
- Fontlar: Oxanium vb.
- Animasyonlar: minimal.

Teknik not: `modules/lock/LockSurface.qml` doğrudan import edilemeyebilir (Quickshell importları). Bu yüzden UI bileşenlerini **kopyalayıp sadeleştirmek** veya **tema-only QML modülü** çıkarmak daha gerçekçi.

---

## 5) Repo/Proje Yapısı

```
ii-greetd-qml/
  backend/          # Rust crate (greetd_ipc)
  ui/               # Qt6/QML app
  protocol/         # JSON schema, örnek message’lar
  docs/             # Arch/greetd/cage konfig örnekleri
  packaging/arch/   # PKGBUILD, install scripts
```

Build sistemi önerisi:
- Backend: `cargo`
- UI: `cmake` (Qt6)
- Üst seviye: `just` veya `make` ile ortak komutlar.

---

## 6) Milestones

### M1 — Backend skeleton (1–2 gün)
- greetd’ye bağlanabilen Rust binary.
- “connect + query + error reporting” tamam.

### M2 — UI skeleton (1–2 gün)
- QML login form + state.
- Backend ile JSON pipe üzerinden konuşma.

### M3 — Auth success path (2–4 gün)
- Username + password ile greetd auth.
- Başarılı login sonrası `niri` session başlatma.

### M4 — Kiosk launch (1 gün)
- `cage -s -- …` ile UI’nin TTY’de düzgün açılması.
- Çoklu monitor davranışı not edilir.

### M5 — Görsel entegrasyon (3–7 gün)
- ii-niri’ye benzer tema, font, background.
- (Opsiyonel) blur; performans testleri.

### M6 — Arch packaging (1–2 gün)
- `PKGBUILD`, örnek `/etc/greetd/config.toml`.
- Dokümantasyon + troubleshooting.

---

## 7) Güvenlik Kontrol Listesi
- Parola hiçbir log’a yazılmamalı (debug çıktıları dahil).
- Backend tarafında parola buffer’ı mümkün olduğunca hızlı sıfırlanmalı.
- UI crash olursa backend greetd session’ı temiz kapatmalı.
- “Poweroff/reboot” eylemleri: polkit/izin modeli net olmalı.
- `cage`/Wayland compositor güvenliği: sadece greeter client çalışmalı.

---

## 8) Arch Konfig Örneği (hedef)

`/etc/greetd/config.toml` (örnek):
- `default_session.command = "cage -s -- ii-greetd-backend --spawn-ui"`
- `default_session.user = "greeter"`

Ek bağımlılıklar:
- `greetd`
- `cage`
- `qt6-base` `qt6-declarative` (UI)
- `rust` toolchain (build)

---

## 9) İlk Next Steps (hemen)
1) `greetd_ipc` API’sini netleştir (backend prototip).
2) QML UI MVP’yi çiz.
3) `cage` ile TTY’de GUI açmayı doğrula.
