# Konsep produk theDOCK

Dokumen ini mencatat kebutuhan produk dari diskusi awal. Statusnya **rancangan**, bukan daftar fitur yang sudah diimplementasikan.

## Tujuan dan platform

theDOCK adalah pengganti taskbar untuk Windows 11. Bilah bergaya *liquid glass* menggabungkan akses aplikasi dan informasi singkat, sambil tetap ringan saat berjalan sepanjang hari. Dukungan Windows 10 dapat dipertimbangkan setelah versi Windows 11 terbukti bekerja.

Implementasi yang diusulkan memakai Rust dan API Windows native. Rust tidak memakai *garbage collector*, tetapi penggunaan RAM tetap harus dibuktikan lewat pengukuran aplikasi nyata. Untuk tampilan, Acrylic Windows dapat menjadi dasar blur; efek pembiasan tambahan perlu implementasi dan pengukuran GPU tersendiri.

## Perilaku utama

1. Ketika theDOCK diaktifkan, taskbar Windows disembunyikan. Keadaan taskbar sebelumnya dicatat agar dapat dipulihkan saat theDOCK ditutup atau gagal berjalan. Mekanisme pemulihan harus diuji untuk keluar normal, crash, dan masuk ulang ke Windows.
2. Tombol Tray membuka panel kaca yang menempel pada dock. Ikon tray tersembunyi sampai panel dibuka. Panel tidak memunculkan tray bawaan Windows.
3. Panel Tray dirancang untuk mendukung klik kiri dan menu klik kanan ikon. Kemampuan menampilkan serta mengendalikan **semua** ikon tray milik aplikasi lain masih memerlukan prototipe teknis di Windows 11; API publik tidak menyediakan daftar umum semua ikon tersebut. Jangan menyatakan kemampuan ini selesai sebelum diuji.
4. Pengaturan selalu dapat dicapai melalui menu konteks dock dan pintasan keyboard, termasuk ketika semua komponen tampilan dimatikan.

## Komponen yang dapat dipilih

Setiap komponen yang ditampilkan memiliki pengaturan aktif/nonaktif tersendiri:

| Komponen | Perilaku yang direncanakan |
| --- | --- |
| Aplikasi yang dipin | Membuka aplikasi dan mengubah urutan pin. |
| Aplikasi berjalan | Menampilkan dan berpindah ke jendela aplikasi. |
| Jam, tanggal, kalender | Informasi waktu dan agenda sederhana. |
| Musik | Informasi dan kontrol media yang sedang diputar. |
| CPU, RAM, baterai | Informasi sistem singkat. |
| Tray | Panel ikon tersembunyi yang menyatu dengan dock. |
| Efek liquid glass | Bisa dimatikan atau diatur tingkat efeknya. |

Pengaturan lain yang direncanakan: posisi atas/bawah, ukuran dock, sembunyi otomatis, serta tema kaca terang/gelap. Opsi penyembunyian taskbar juga tersedia di Settings; keadaan awalnya aktif sesuai tujuan theDOCK sebagai pengganti taskbar.

## Rancangan visual

Bilah berbentuk kapsul dengan latar transparan buram, tepi bercahaya tipis, dan pantulan singkat saat kursor menyentuhnya. Panel Tray memakai bahasa visual yang sama dan muncul sebagai bagian dari dock. [Mockup](mockup-thedock.png) memperlihatkan keadaan Tray terbuka dan panel Settings untuk menjelaskan pilihan komponen.

## Target performa dan validasi

Target desain awal, **belum hasil uji**: RAM saat diam di bawah 80 MB dan CPU mendekati 0% ketika tidak ada perubahan. Ikon dimuat seperlunya; pembaruan komponen mengikuti peristiwa sistem bila tersedia; animasi berhenti saat dock diam. Efek pembiasan tingkat lanjut mendapat sakelar tersendiri karena biaya GPU dapat berbeda dari Acrylic dasar.

Validasi prototipe Windows perlu mencakup:

- Dock tampil, aplikasi bisa dibuka, dan taskbar tersembunyi saat aktif.
- Taskbar pulih setelah keluar normal, crash, dan sesi Windows berikutnya.
- Setiap sakelar komponen mengubah tampilan tanpa menghilangkan akses ke Settings.
- Panel Tray tetap menyatu dengan dock dan diuji dengan beberapa aplikasi pihak ketiga, termasuk klik kiri serta menu klik kanan.
- Penggunaan RAM, CPU saat diam, dan biaya GPU efek kaca diukur pada perangkat Windows nyata.

Integrasi akun dan katalog widget tambahan berada di luar cakupan pertama. Status, izin, dan prasyaratnya harus dijelaskan sebelum ditambahkan.
