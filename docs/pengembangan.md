# Pengembangan prototipe

## Fitur yang sudah dikodekan

Prototipe ditulis dengan Rust dan Win32 native. Yang sudah dikodekan:

- Dock di layar utama dengan tombol Start, aplikasi yang dipin, jendela berjalan, jam, tanggal, CPU, RAM, dan baterai. Ruang ikon menyesuaikan lebar layar; menu `···` memberi akses ke aplikasi yang tidak muat.
- Seret `.exe` atau pintasan dari Explorer ke dock untuk memin aplikasi. Klik kiri untuk membuka; klik kanan untuk menggeser urutan atau melepas pin. Klik ikon jendela berjalan untuk berpindah ke jendela tersebut.
- Klik tanggal untuk melihat kalender sederhana. Tombol Media mengirim perintah putar/jeda ke Windows; judul lagu dan kontrol lagu berikutnya belum tersedia.
- Posisi dock dapat dipindah ke atas/bawah. Menu menyediakan tiga pilihan lebar, sembunyi otomatis dengan jalur pembuka di tepi layar, serta kaca terang/gelap.
- Menu pengaturan melalui klik kanan atau `Ctrl+Alt+D`. Setiap komponen yang tampil memiliki sakelar sendiri. Pengaturan disimpan di `%APPDATA%\theDOCK\settings.json`.
- Pengaturan untuk menyembunyikan taskbar aktif secara bawaan. Taskbar yang semula terlihat dipulihkan saat keluar normal. Executable pengawas terpisah mencoba memulihkannya jika aplikasi utama berhenti mendadak. Jika pengawas gagal mulai, taskbar tetap terlihat dan dock masih dapat digunakan.
- Panel Tray terbuka dalam jendela dock yang sama. Volume turun/naik, mute, dan jalan pintas ke pengaturan jaringan/Bluetooth tersedia dengan sakelar masing-masing. Ikon tray milik aplikasi lain masih belum tersedia karena jalur integrasinya belum tervalidasi.
- Efek kaca mencoba memakai *DWM transient system backdrop* Windows 11. Seluruh area dock kini selalu dicat gelap dengan transparansi ringan agar teks tetap terbaca pada tema Windows terang maupun gelap.
- Ikon theDOCK tersedia sebagai [PNG](../assets/thedock-icon.png) dan [ICO](../assets/thedock.ico), serta ditanam ke executable Windows.

### Perbaikan tampilan 0.1.1

Uji pengguna menunjukkan kapsul dock putih dengan tombol Start dan Tray terlihat, tetapi teks utama nyaris hilang. Prototipe sebelumnya memperluas bingkai DWM ke seluruh area klien lalu melewatkan pengecatan latar saat efek kaca aktif. Perbaikan ini menghapus perluasan bingkai tersebut, mewarnai permukaan dock secara eksplisit, memakai teks terang di atas latar gelap, dan memperbesar font. Kontras sekarang tidak bergantung pada warna backdrop Windows. Hasil akhirnya tetap perlu dilihat pada mesin Windows 11.

Ini masih tahap pengembangan. Kode telah dikompilasi untuk target Windows melalui `cargo check`; perilaku UI dan pemulihan taskbar **belum diuji pada Windows 11**. Jangan gunakan sebagai pengganti taskbar utama sebelum pengujian tersebut selesai. `thedock-watchdog.exe` harus berada di folder yang sama dengan `thedock.exe` agar taskbar dapat disembunyikan secara aman.

## Membangun dan menjalankan

Di Windows 11 dengan Rust terpasang:

```powershell
cargo test --locked
cargo build --locked --release --bins
.\target\release\thedock.exe
```

`rust-toolchain.toml` menentukan versi Rust. Klik kanan dock atau tekan `Ctrl+Alt+D` untuk membuka sakelar fitur dan pilihan keluar. Tombol Start membuka menu Start Windows. Menu `Keluar dari theDOCK` memulihkan taskbar. Artefak GitHub Actions berisi **kedua** executable; simpan keduanya dalam satu folder.

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
5. Uji posisi atas/bawah, tiga lebar dock, sembunyi otomatis, kaca terang/gelap, kontrol volume, serta jalan pintas jaringan/Bluetooth.
6. Validasi teknik membaca, menggambar, dan mengklik ikon tray aplikasi lain sebelum mengganti panel status prototipe.
7. Ukur RAM, CPU diam, dan biaya GPU DWM pada perangkat nyata. Target di [spesifikasi konsep](konsep.md) belum merupakan hasil ukur.

Fitur kontrol tray pihak ketiga, metadata musik, dan efek pembiasan kaca masih dalam rencana. Pilihan ukuran saat ini mengubah **lebar**, belum tinggi atau ukuran ikon.
