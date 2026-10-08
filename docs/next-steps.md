# ModelShelf — Sıradaki işler

Son güncelleme: 8 Ekim 2026

Bu dosya mevcut Windows MVP'sinden devam etmek için uygulanabilir iş sırasını belirtir. Kutular yalnızca ilgili kabul ölçütleri doğrulandıktan sonra işaretlenmelidir. Tamamlanan altyapı yeniden kurulmayacak; mevcut kod ve veriler korunacaktır.

## Mevcut başlangıç noktası

- Tauri/Rust masaüstü uygulaması ve Windows NSIS paketi oluşturuldu.
- Gerçek Hugging Face arama, dosya seçimi, indirme, kitaplık ve yeniden açılış akışı doğrulandı.
- SQLite kalıcılığı, koşullu HTTP Range devam ettirme, SHA-256 kontrolü ve güvenli harici dosya indeksleme mevcut.
- Son doğrulamada 19 Rust testi ve 6 ön yüz testi geçti. Canlı sağlayıcı testi ayrıca çalıştırıldı.
- Ana arayüz İngilizce/Türkçe; açık/koyu/sistem temaları mevcut.
- Sürüm deneysel ve imzasızdır. Geniş kapsamlı kararlı sürüm kabul testleri tamamlanmadı.

Ayrıntılı kanıt ve sınırlamalar: [implementation-status.md](implementation-status.md).

## P0 — Önce güvenilirlik ve veri güvenliği

### 1. Geliştirme ve test verilerini kullanıcı verilerinden ayır

- [ ] Geliştirme, otomasyon ve dağıtılan uygulama için ayrı uygulama veri dizinleri tanımla.
- [ ] Yerel masaüstü testlerini geçici bir profil ve test indirme diziniyle çalıştır.
- [ ] Mevcut kullanıcı veritabanını taşımadan veya üzerine yazmadan profil seçimini uygula.

**Kabul ölçütü:** Testleri çalıştırmak gerçek kitaplığı, ayarları veya kayıtlı hesabı değiştirmemeli. Mevcut sürümde geliştirme ve normal uygulama aynı uygulama kimliğini kullanıyor.

### 2. Windows kabul testlerini tamamla

- [ ] Windows 10 ve Windows 11 x64 üzerinde temiz kurulum, açılış, kapanış ve yeniden kurulum senaryolarını çalıştır.
- [ ] WebView2 mevcutken ve mevcut değilken kurulum davranışını doğrula.
- [ ] Yerel dosya/klasör seçicilerini, içe aktarmayı ve kalıcı silme onayını gerçek masaüstünde dene.
- [ ] Boşluk, Türkçe karakter ve uzun yol içeren dizinleri dene.
- [ ] Harici disk bağlantısının kesilmesi, sürücü harfinin değişmesi, yazma izni olmaması ve düşük disk alanını dene.
- [ ] Sonuçları işletim sistemi, senaryo, beklenen sonuç ve gözlenen sonuç bilgileriyle kaydet.

**Kabul ölçütü:** Hatalar anlaşılır biçimde gösterilmeli; harici dosyalar ve ilgisiz dosyalar değişmemeli. Yeniden açılışta kitaplık ve indirme geçmişi korunmalı.

### 3. Büyük indirmeleri ve kurtarma sınır durumlarını doğrula

- [ ] Çok gigabaytlık gerçek bir dosyada bağlantı kesilmesi, uygulamanın zorla kapanması ve bilgisayarın yeniden başlamasını test et.
- [ ] 1–3 eşzamanlı aktarımda bellek kullanımı, disk yazımı, ilerleme olayları ve arayüz tepkisini ölç.
- [ ] Hızlı duraklat/devam et/iptal/yeniden dene işlemleri ve kuyruk sıralaması için regresyon testleri ekle.
- [ ] Eksik/bozuk checkpoint, değiştirilmiş kısmi dosya, zayıf/eksik ETag, geçersiz Content-Range ve zaman aşımı kapsamını genişlet.
- [ ] 401, 403, 404, 429 ve süresi dolmuş indirme bağlantısı davranışını doğrula.
- [ ] Aktarım tamamlanmadan önce diskin dolmasını ve tamamlanmış dosyanın dışarıdan değişmesini test et.

**Kabul ölçütü:** Yeniden kullanılan bayt sayısı gerçek olmalı; güvenli devam mümkün değilse açıkça yeniden başlatılmalı veya hata verilmeli. Eksik/bozuk dosya tamamlandı sayılmamalı. Hız ve ETA, diskten yeniden kullanılan baytları ağ aktarımı olarak saymamalı.

### 4. Dosya işlemlerini ve disk uyumluluğunu güçlendir

- [ ] Yol kontrolü ile dosya işlemi arasındaki yarış koşullarını incele; mümkün olan yerlerde işletim sistemi dosya tanıtıcıları üzerinden doğrulama uygula.
- [ ] Junction/symlink üzerinden kapsam dışına çıkma girişimlerini doğrudan yönetilen dosya silme akışında test et.
- [ ] Hardlink desteğini indirme başlamadan önce denetle; desteklenmeyen dosya sisteminde erken ve anlaşılır hata göster.
- [ ] Gerekirse hardlink desteklemeyen diskler için mevcut dosyanın üzerine yazmayan güvenli bir yayınlama yöntemi tasarla ve test et.
- [ ] İptal edilmiş indirmelerin yalnızca uygulamaya ait kısmi dosyalarını, kullanıcı onayıyla temizleyen bir işlem ekle.

**Kabul ölçütü:** Silme veya temizleme yetkili kapsam dışına çıkmamalı; ilgisiz dosyalar korunmalı. Uyumsuz disk nedeniyle saatler süren bir aktarımın sonunda sürpriz hata oluşmamalı.

### 5. Hesap ve kısıtlı depo akışını doğrula

- [ ] Geçerli/geçersiz token, token iptali, hesap bağlantısını kesme ve kimlik deposuna erişilememesi senaryolarını dene.
- [ ] Meşru erişim izni olan ve olmayan gated depoları test et.
- [ ] Yönlendirme testleriyle token'ın sağlayıcı dışındaki sunuculara gönderilmediğini doğrula.
- [ ] Günlük ve tanılama çıktılarında token bulunmadığını denetle.

**Kabul ölçütü:** Lisans koşulları otomatik kabul edilmemeli; erişim kısıtları aşılmamalı. Anonim kullanım ve çevrimdışı yerel kitaplık çalışmaya devam etmeli.

## P1 — Ürün deneyimini tamamla

### 6. Kitaplık ve model inceleme

- [ ] Harici modele elle Hugging Face deposu ilişkilendirme ve ilişkiyi kaldırma ekle; elle girilen ilişkiyi doğrulanmış indirme kaynağından ayır.
- [ ] GGUF/SafeTensors başlıklarından güvenli, boyutu sınırlı metadata okuma ekle; dosya içeriğini veya kodunu çalıştırma.
- [ ] Yinelenen kayıt analizine aynı depo/revizyon ve aynı boyut kategorilerini ekle. Boyut veya benzer ada dayalı sonuçları kesin eşleşme olarak sunma.
- [ ] Modelin ek dosyalara ihtiyaç duyabileceğini açıklayan, yalnızca mevcut metadata ile desteklenen uyarılar ekle.

**Kabul ölçütü:** Eksik metadata uydurulmamalı; indeksleme hiçbir zaman dosya sahipliği kazandırmamalı. Yinelenen dosyalar otomatik silinmemeli.

### 7. Keşif, çeviri ve erişilebilirlik

- [ ] Arama sonuçlarındaki mevcut 48 kayıt sınırı için sayfalama veya daha fazla sonuç yükleme ekle.
- [ ] Hata mesajlarını ve yerel onay diyaloglarını İngilizce/Türkçe çeviri sistemine bağla.
- [ ] Dosya ağacı, kısmi klasör seçimi, modal odak yönetimi ve klavye gezinmesini bileşen testleriyle doğrula.
- [ ] Ekran okuyucu, yüksek ölçekleme ve dar pencere senaryolarını test et.
- [ ] Uzun süren bütünlük kontrolü ve yeniden tarama işlemlerinde açık işlem durumu ve uygun iptal davranışı sağla.

**Kabul ölçütü:** Ana akışlar yalnızca klavyeyle tamamlanabilmeli; kontroller yaptıkları işlemi doğru adlandırmalı ve başarısız işlemler başarı olarak gösterilmemeli.

### 8. Büyük kitaplıklarda performans

- [ ] Büyük klasör ve çok dosyalı depolarda SQLite güncellemelerini, snapshot maliyetini, açık dosya tanıtıcılarını ve arayüz oluşturma süresini ölç.
- [ ] Gerektiğinde depolama hesabını önbellekle ve büyük listeleri sanallaştır.
- [ ] Ölçümler gerektirirse dosya bazlı sorgular için normalize edilmiş SQL tablolarına sürümlü geçiş hazırla.
- [ ] Şema geçişini mevcut veritabanı kopyaları üzerinde test et; yedekleme ve hata durumunda kurtarma prosedürünü belgele.

**Kabul ölçütü:** Arka plan indirmeleri sürerken arayüz kullanılabilir kalmalı. Şema değişiklikleri mevcut kitaplık ve indirme kayıtlarını kaybetmemeli.

## P2 — Açık kaynak yayın hazırlığı

- [ ] Gerçek GitHub depo adresini belirle; README ve Hakkında bölümüne depo, sorun bildirme ve katkı bağlantılarını ekle.
- [ ] Bakımcı iletişimini ve özel güvenlik bildirim kanalını tanımla; GitHub özel güvenlik bildirimlerini etkinleştir.
- [ ] Kaynakları ve kilit dosyalarını gözden geçirerek ilk Git commit'ini ve uzak depoyu hazırla; yayınlamayı ayrı bir adım olarak gerçekleştir.
- [ ] GitHub Actions işlerini gerçek uzak depoda çalıştır; Rust bağımlılık güvenlik taraması dahil sonuçları düzelt.
- [ ] Kod imzalama sertifikası ve CI sırlarını bakımcı tarafından sağlandıktan sonra Windows paket imzalamayı yapılandır.
- [ ] Sürüm notlarını, güncel gerçek ekran görüntülerini, kurulum/kaldırma talimatlarını ve paket SHA-256 özetini hazırla.
- [ ] P0 kabul ölçütleri tamamlanana kadar sürümü ön sürüm olarak yayımla; kararlı sürüm iddiasında bulunma.

**Kabul ölçütü:** İndirilebilir paket ile etiketlenen kaynak aynı olmalı; CI yeşil olmalı; imza durumu ve bilinen sınırlamalar açıkça belirtilmeli. Depo adresi veya destek iletişimi uydurulmamalı.

## P3 — Sonraki sürümlere ait genişletmeler

Bu işler güvenilir MVP ve yayın hazırlığını geciktirmemeli. [Genel yol haritası](roadmap.md) ile birlikte takip edilmeli.

- [ ] Kullanıcı onaylı, salt okunur Hugging Face önbellek taraması; symlink ve paylaşılan blob farkındalığı.
- [ ] Native Xet aktarımı; protokole özgü bütünlük, kimlik doğrulama ve kurtarma testleri tamamlandıktan sonra etkinleştirme.
- [ ] İsteğe bağlı yönetilen depolamaya kopyalayarak içe aktarma; kaynak dosyaları taşıma veya değiştirme yok.
- [ ] Açılış davranışı ayarları, diskler arası kontrollü yeniden konumlandırma ve indirme hız sınırı.
- [ ] Ollama/LM Studio envanteri ve ModelScope gibi ek sağlayıcılar.
- [ ] Linux/macOS paketleri ve platforma özgü kimlik deposu testleri.
- [ ] İmzalı otomatik güncelleme, taşınabilir mod, CLI ve ek diller.

## Her anlamlı değişiklikten sonra doğrulama

Değişiklikle ilgili testleri önce çalıştır. Yayın adayı için tam kontrol dizisi:

```powershell
pnpm install --frozen-lockfile
pnpm typecheck
pnpm lint
pnpm test
pnpm build:ui
pnpm format:check
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
pnpm audit --prod
pnpm build
```

Sağlayıcı entegrasyonu değiştiğinde, internet erişimiyle:

```powershell
cargo test -p modelshelf-download --test live_hub -- --ignored --nocapture
```

Masaüstü kabul testi için [geliştirme belgesindeki](development.md) WebView2 otomasyon talimatlarını kullan. Mevcut otomasyon gerçek uygulama durumuna yazar; profil izolasyonu tamamlanana kadar bunu test ortamında çalıştır.

Her tamamlanan işten sonra bu dosyayı ve [implementation-status.md](implementation-status.md) dosyasını güncelle. Çalıştırılmayan bir testi veya üretilmeyen bir paketi başarılı olarak işaretleme.

## Bir sonraki oturumun başlangıcı

1. `git status --short` ile mevcut değişiklikleri kontrol et; var olan çalışmayı koru.
2. Bu dosyayı ve uygulama durum belgesini oku.
3. **P0 / 1: Geliştirme ve test verilerini kullanıcı verilerinden ayır** maddesinden başla.
4. İlgili kabul ölçütlerini doğrula ve sonucu belgelerde kaydet.
