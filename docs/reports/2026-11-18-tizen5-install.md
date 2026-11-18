# Samsung-Modelljahr 2019 · Tizen 5.0 · Installation

Modell GQ55Q60RGTXZG, vom Gerät gemeldete Plattform 5.0/armv7. Das normale 0.7.2-Paket
verlangt 6.5 und wurde mit Parsing error abgelehnt. Ein gesondertes 2019-Profil setzt
Mindestversion 5.0 und Chrome-63-Transpilation; normale Builds bleiben auf 6.5/Chrome 85.

Fehlende Browser-APIs werden nur im älteren Profil ergänzt. Ein AbortController-Ersatz
verwendet XMLHttpRequest, sodass Zeitüberschreitungen tatsächlich Netzwerkzugriffe abbrechen.
ResizeObserver-Ersatz reagiert auf Viewportänderungen ohne dauernde Polling-Schleife.

Prüfung: Typecheck, 25 Unit-Tests und 38 Oberflächentests bestanden; ein vorhandener
Codec-Test übersprungen. Bei den älteren Browserprüfungen werden die ersetzten APIs
vor dem Start entfernt. Dies ist eine Simulation in aktuellem Chromium, kein Nachweis
für jede Besonderheit des Samsung-Browsers.

Der erste Signierungsversuch mit dem bisherigen Geräteprofil wurde durch den Fernseher
abgelehnt. Nach der vom Nutzer abgeschlossenen Erstellung seines zusätzlichen Samsung-
Zertifikatsprofils wurde das getrennte WGT neu signiert, tatsächlich installiert und
erfolgreich gestartet. Bestehende Pakete und das ursprüngliche Zertifikatsprofil erhalten.

Artefakt lokal: Releases/Tests/Samsung-Tizen5/Dado-Player-Tizen-0.7.2.wgt.
Keine Zertifikate, Passwörter, DUIDs oder private Netzwerkdaten in diesem Bericht.
Physische Wiedergabe und Bedienung sind anschließend auf dem TV zu bestätigen.
