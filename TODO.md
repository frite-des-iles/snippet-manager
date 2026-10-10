# TODO — Snippet Manager

## En cours
- [ ] Recherche fonctionnelle (brancher la barre en haut)
    - filtre par titre, code, description
    - se combine avec le filtre par langage
    - afficher "X résultats" dans la barre du bas quand un filtre est actif

## Fonctionnalités à faire (par priorité)
- [ ] Suppression de snippet (bouton présent, à brancher) + confirmation
- [ ] Édition d'un snippet (réutiliser la modale d'ajout en mode "édition")
- [ ] Favoris (étoile ⭐ sur chaque snippet, tri par favori)
- [ ] Tri de la liste (date, alpha, use_count, favoris)
- [ ] Import/Export JSON (sauvegarde, partage, synchro manuelle via clé USB)
- [ ] Auto-fermeture des paires dans l'éditeur de code (`(`, `[`, `{`, `"`, `'`)
  → si le fermant est déjà juste après le curseur, avancer au lieu d'insérer
  → pistes : crate `egui_code_editor`, ou TextEditState + comparaison new_code / new_code_prev
- [ ] Sync P2P automatique sur LAN (mDNS + CRDT, ex. `libp2p` + `Loro`)
- [ ] Packaging binaire portable (`cargo build --release`, tester Win + Linux)

## Améliorations UI (détails)
- [ ] Masquer les flèches ◀ ▶ de la barre de langages quand tout tient (pas de débordement)
- [ ] Auto-focus sur le champ de recherche du combo langage à l'ouverture
- [ ] Repositionner le popup du combo si proche du bas de l'écran (remonter au lieu de descendre)
- [ ] Option "wrap layout" pour les langages (plusieurs lignes) au lieu du scroll horizontal
- [ ] Indicateur visuel de scrollbar discrète pour les onglets de langage (optionnel)

## Projets suivants (file d'attente personnelle)
1. Launcher de projets
    - recherche dans un répertoire racine unique
    - chaque projet a un fichier de config à sa racine (.launcher.toml ou équivalent)
    - champs : nom, type, IDE, commandes (build/run/test/git), icône, tags
2. Journal de dépenses complet
    - catégories, mois, stats, export CSV/PDF, multi-devises, sauvegarde
3. Traitement d'images en lot + images → PDF
    - redimensionner, compresser, convertir, renommer
    - X images → 1 PDF (A4, marge, orientation, qualité)
4. Détecteur de doublons avec GUI
    - scan, hash, prévisualisation, sélection fine, corbeille
5. OCR léger
    - image/PDF → texte, copie, export, détection de langue
6. (Déjà commencé) Snippet Manager ← ON EST ICI

## Idées non retenues (pour mémoire)
- Messagerie LAN portable, sans install, sans admin
- Notes markdown local
- Comparateur de fichiers/dossiers
- Extracteur d'archives portable
- Tableau de bord système léger
- Suivi d'habitudes, de santé, de factures, de recettes
- Générateur de mot de passe / coffre local