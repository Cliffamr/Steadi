# Veil Browser

Browser overlay ringan untuk Windows 10 2004+ dan Windows 11. Berbasis Tauri 2 + WebView2, diturunkan dari [Steadi](https://github.com/Siddharth-Khattar/Steadi).

## Fitur

- Tidak tampil pada capture API Windows standar melalui `WDA_EXCLUDEFROMCAPTURE`
- Always on top, opacity 30–100%, click-through
- Address bar, back, forward, reload
- Maksimal 5 tab; tab latar tidur setelah 2 menit
- Profil incognito sementara; extension, popup, download diblokir
- Navigasi HTTPS-only
- Halaman web dipisah dari capability dan IPC Tauri
- Hotkey sembunyikan darurat `Ctrl+Shift+H`; pulihkan eksplisit `Ctrl+Shift+J`; click-through `Ctrl+Shift+X`
- Self-test affinity OS + checklist Zoom, Meet, Teams, Discord, OBS

## Batasan penting

Perlindungan capture bukan DRM dan tidak menjamin tersembunyi dari capture card, kamera, remote desktop, atau metode capture nonstandar. Selalu periksa preview aplikasi meeting sebelum membagikan layar. Jika perekaman sudah berjalan sebelum browser dibuka dan muncul kotak hitam, hentikan lalu mulai kembali capture setelah Veil Browser aktif.

## Pengembangan

```sh
npm ci
npm test
npm run check
npm run build
npm run tauri build -- --no-sign
```

Build utama menargetkan Windows NSIS. WebView2 Runtime wajib tersedia.

## Privasi

- Tidak ada sinkronisasi akun.
- Tidak ada extension.
- Browser memakai profil incognito WebView2.
- Menutup aplikasi menghapus sesi profil sementara sesuai perilaku WebView2.

## Lisensi

MIT. Lihat `LICENSE` untuk atribusi proyek asal.
