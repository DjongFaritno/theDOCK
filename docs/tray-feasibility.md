# Validasi tray aplikasi lain

Syarat produk: ikon tray aplikasi lain tersembunyi sampai tombol Tray di theDOCK dibuka. Ikon serta tindakan klik harus muncul dalam panel theDOCK, tanpa membuka panel tray bawaan Windows.

API publik Windows untuk area notifikasi, seperti `Shell_NotifyIcon`, digunakan aplikasi untuk mendaftarkan **ikon miliknya sendiri**. API tersebut tidak menyediakan daftar lengkap ikon yang didaftarkan aplikasi lain. `Shell_NotifyIconGetRect` memerlukan identitas ikon yang sudah diketahui. Karena itu, panel theDOCK saat ini hanya memuat kontrol sistem yang implementasinya terpisah dari ikon tray pihak ketiga.

## Probe read-only

Artefak build Windows menyertakan `thedock-tray-probe.exe`. Jalankan dari PowerShell:

```powershell
.\thedock-tray-probe.exe
```

Program ini hanya mencari jendela taskbar dan overflow Explorer, lalu menanyakan jumlah tombol pada `ToolbarWindow32` jika ada. Program tidak membuka panel, membaca memori proses lain, mengklik ikon, atau mengirim data ke jaringan. Outputnya hanya status `found`, `visible`, dan `button_count`/`timeout`.

Jalankan sekali sebelum theDOCK aktif dan sekali saat theDOCK aktif. Perbedaan hasil menunjukkan apakah toolbar tetap tersedia ketika taskbar disembunyikan. Jumlah tombol saja **tidak** membuktikan ikon dapat digambar dan diklik dari theDOCK; itu hanya menentukan apakah pendekatan toolbar patut diprototipekan di versi Windows yang diuji.

Sebelum fitur tray pihak ketiga dinyatakan selesai, perlu dibuktikan pada perangkat Windows 11 nyata bahwa seluruh ikon dapat ditemukan, digambar, dan menerima klik kiri serta menu kanan, termasuk setelah Explorer dimulai ulang. Integrasi ke struktur internal Explorer berisiko berubah antarversi Windows, sehingga hasil probe perlu menjadi dasar keputusan implementasi.
