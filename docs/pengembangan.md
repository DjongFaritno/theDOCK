# Pengembangan prototipe

## Milestone pertama

Prototipe ditulis dengan Rust dan Win32 native. Yang sudah dikodekan:

- Dock di layar utama dengan tombol Start, jam, tanggal, penggunaan RAM, dan baterai.
- Menu pengaturan melalui klik kanan atau `Ctrl+Alt+D`. Setiap komponen yang tampil memiliki sakelar sendiri. Pengaturan disimpan di `%APPDATA%\theDOCK\settings.json`.
- Pengaturan untuk menyembunyikan taskbar aktif secara bawaan. Taskbar yang semula terlihat dipulihkan saat keluar normal. Proses pengawas terpisah mencoba memulihkannya jika proses utama berhenti mendadak. Jika pengawas gagal mulai, taskbar tidak disembunyikan.
- Panel Tray terbuka dalam jendela dock yang sama. Panel ini masih menampilkan status prototipe karena akses ke seluruh ikon tray aplikasi lain belum tervalidasi.
- Efek kaca mencoba memakai *DWM transient system backdrop* Windows 11, dengan tampilan gelap cadangan bila efek tidak tersedia.

Ini masih tahap pengembangan. Kode telah dikompilasi untuk target Windows melalui `cargo check`; perilaku UI dan pemulihan taskbar **belum diuji pada Windows 11**. Jangan gunakan sebagai pengganti taskbar utama sebelum pengujian tersebut selesai. Proses pengawas memakai executable yang sama; perintah yang menghentikan semua proses `thedock.exe` sekaligus juga menghentikan pengawas.

## Membangun dan menjalankan

Di Windows 11 dengan Rust terpasang:

```powershell
cargo test --locked
cargo build --locked --release
.\target\release\thedock.exe
```

`rust-toolchain.toml` menentukan versi Rust. Klik kanan dock atau tekan `Ctrl+Alt+D` untuk membuka sakelar fitur dan pilihan keluar. Tombol Start membuka menu Start Windows. Menu `Keluar dari theDOCK` memulihkan taskbar. Alur build Windows di GitHub Actions menyimpan executable sebagai artefak ketika berhasil.

Di Linux, unit test dan pemeriksaan kode Windows dapat dijalankan tanpa menjalankan UI:

```sh
rustup target add x86_64-pc-windows-gnu
cargo test --locked
cargo check --locked --target x86_64-pc-windows-gnu
```

`cargo check` tidak menghasilkan `.exe`. Build lintas platform penuh memerlukan linker dan alat MinGW; gunakan build Windows di atas untuk executable.

## Pengujian Windows yang masih diperlukan

1. Jalankan di mesin atau VM Windows 11 dengan akses pemulihan desktop. Periksa ukuran, posisi, kaca, dan respons klik pada skala DPI berbeda.
2. Periksa taskbar tersembunyi saat dock aktif dan kembali setelah keluar lewat menu. Ulangi ketika taskbar awalnya sudah tersembunyi.
3. Hentikan hanya proses utama berdasarkan PID; pastikan proses pengawas mengembalikan taskbar. Uji juga saat Explorer dimulai ulang.
4. Ubah setiap sakelar, jalankan ulang, dan periksa pengaturan tetap tersimpan serta menu masih dapat diakses.
5. Validasi teknik membaca, menggambar, dan mengklik ikon tray aplikasi lain sebelum mengganti panel status prototipe.
6. Ukur RAM, CPU diam, dan biaya GPU DWM pada perangkat nyata. Target di [spesifikasi konsep](konsep.md) belum merupakan hasil ukur.

Fitur aplikasi yang dipin, aplikasi berjalan, kalender, musik, CPU, kontrol tray pihak ketiga, auto-hide, dan posisi dock masih dalam rencana.
