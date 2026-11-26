# Samsung Store

Technische Checkliste für die Vorbereitung eines Samsung-Store-Pakets.

## Paket und Unterlagen

- WGT 0.6.4 vorhanden: Releases/Dado-Player-Tizen-0.6.4.wgt. Aktuelle Signierung ist für
  den eingerichteten Entwickler-TV; sie ist noch keine bestätigte Store-Paketabnahme.
- App-Eintrag unter Applications und geeignete Paketprüfung/Signierung vorbereiten.
  Noch kein WGT-Upload oder Zertifizierungsantrag in diesem Onboarding-Schritt.
- Store-Bilder nach Samsungs Vorgaben: Icons 1920×1080 und 512×423, jeweils höchstens 300 KB;
  beim großen Icon transparentes RGBA-Logo plus RGB/JPG-Hintergrund getrennt bereitstellen.
- Eigener Store-Hintergrund: 1920×1080 JPG, höchstens 2 MB; wichtige Inhalte rechts platzieren.
- Vier echte App-Screenshots: jeweils 1920×1080 JPG, höchstens 500 KB. Testbilder nur mit
  eigenen/lizenzierten Demoinhalten; keine echten Anbieterzugänge, Kundendaten oder Rechtebehauptungen.
- Titel, Beschreibungen und mindestens drei Suchbegriffe je veröffentlichter Sprache.
  Titel der Standardsprache muss dem config.xml-Titel entsprechen.
- Serviceinformationen: Kategorie, Altersfreigabe, Sprachen, erforderliche Datenschutzerklärung
  und Verkäuferangaben prüfen. EU-Zugänglichkeit und gegebenenfalls die verlangte EAA-Erklärung
  vor Freigabe klären; keine unbestätigte Konformität erklären.
- Reale TV-Abnahme einschließlich Fernbedienung, Live/VOD/Serien, Ton/Untertitel,
  Netzwerkausfall, Suspend/Resume und Trial-/Kaufablauf abschließen.
- Endgültiges Kaufmodell und Store-Anforderungen abstimmen, bevor Preise oder Verfügbarkeit
  zugesagt werden. Der einmalige Kaufpreis ist im Projektauftrag noch nicht festgelegt.

## Quellen und Zuständigkeit

Stand 16.11.2026, offizielle Samsung-Dokumentation:

- https://developer.samsung.com/tv-seller-office/guides/applications/registering-application.html
- https://developer.samsung.com/tv-seller-office/guides/applications/entering-application-information.html
- https://developer.samsung.com/tv-seller-office/guides/membership/becoming-partner.html

Partneraufnahme, ein eingereichter Supportfall und eine veröffentlichte Store-App sind
getrennte Zustände. Keinen davon allein aus einem vorhandenen Build oder Supportticket ableiten.

## Vor der Zertifizierung noch zu klären

- Partneraufnahme und gegebenenfalls geforderter Vertrag über den genannten Kontakt.
- Samsung-Abstimmung des 7-Tage-Trials und der manuellen einmaligen Freischaltung;
  kein Kaufmodell als bereits von Samsung genehmigt darstellen.
- Tatsächlich erreichbare, aktuelle Homepage prüfen: Im Portal ist eine andere
  Netlify-Schreibweise gespeichert als in website/README.md. Nicht ungeprüft als
  öffentlichen Support-Link übernehmen.
- Öffentliche, zur tatsächlichen Geräte-/MAC-Lizenzverarbeitung passende
  Datenschutzerklärung bereitstellen; keine noch nicht vorhandene URL einreichen.
- Versionsangaben der öffentlichen Homepage vor Einreichung mit dem aktuellen Build
  abgleichen; die lokale Downloads-Seite enthält noch 0.6.0.
- Vollständig nutzbarer, rechtmäßig bereitgestellter Prüfinhalt/Testzugang und ein
  Test-Freischaltweg für Samsung, damit der Trial die Zertifizierung nicht blockiert.
- Aktuelle Store-Bilder und UI-Beschreibung erzeugen; neue Scroll-/Player-Korrekturen
  auf Hardware abnehmen. Ein installiertes Entwicklerpaket ist keine Store-Abnahme.
