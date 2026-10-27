# theDOCK

theDOCK adalah konsep pengganti taskbar Windows 11 yang ringan, dengan tampilan *liquid glass*. Proyek ini masih berada pada tahap rancangan; belum ada aplikasi yang dapat dipasang atau dijalankan.

![Mockup konsep theDOCK](docs/mockup-thedock.png)

## Arah produk

- Saat theDOCK aktif, taskbar Windows disembunyikan. Saat theDOCK ditutup atau gagal berjalan, taskbar harus dipulihkan ke keadaan sebelumnya.
- Dock memuat peluncur aplikasi, aplikasi yang sedang berjalan, widget pilihan, jam, dan tombol Tray. Setiap komponen tampilan dapat diaktifkan atau dinonaktifkan sendiri.
- Tombol Tray membuka panel ikon yang menyatu dengan dock. Ikon tersembunyi sebelum tombol diklik; rancangan ini tidak membuka panel tray bawaan Windows.
- Pengaturan harus tetap dapat dibuka lewat menu dock atau pintasan keyboard, termasuk saat seluruh komponen tampilan dimatikan.
- Efek kaca dapat dimatikan atau dikurangi untuk menghemat sumber daya.

Lihat [spesifikasi konsep](docs/konsep.md) untuk cakupan versi pertama, interaksi, target performa, dan risiko teknis. Mockup adalah ilustrasi rancangan, bukan tangkapan layar aplikasi yang sudah berjalan.

## Status

Belum ada kode aplikasi, instalasi, atau hasil pengujian Windows. Langkah berikutnya adalah prototipe Windows untuk memvalidasi penggantian taskbar, pemulihannya, dan akses ke ikon tray aplikasi lain sebelum fitur tersebut dinyatakan siap.
