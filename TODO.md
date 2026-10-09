# TODO — Snippet Manager

## Améliorations UI
- [ ] Auto-fermeture des paires dans l'éditeur de code : quand on tape `(`, `[`, `{`, `"`, `'`, insérer automatiquement le fermant correspondant et placer le curseur au milieu.
- [ ] Si le fermant est déjà juste après le curseur et qu'on tape le même fermant, avancer au lieu d'insérer un doublon (comportement IDE).
- [ ] Pistes : crate `egui_code_editor`, ou implémentation maison via `egui::text_edit::TextEditState` + comparaison `new_code` / `new_code_prev`.

## Fonctionnalités à venir
- [ ] Onglets de langage dynamiques dans la barre du haut (remplacer les boutons en dur).
- [ ] Barre de recherche fonctionnelle (globale + scopée par langage).
- [ ] Tags : affichage, filtre, édition.
- [ ] Édition d'un snippet existant (bouton "Éditer" à brancher).
- [ ] Suppression (bouton "Supprimer" à brancher), avec confirmation.
- [ ] Favoris (étoile).
- [ ] Tri de la liste (date, alpha, use_count, favoris).
- [ ] Recherche floue (`rdfl` → `read_file`).

## Synchronisation
- [ ] Import / Export JSON via clé USB.
- [ ] Synchronisation P2P automatique sur LAN (mDNS + CRDT).

## Packaging
- [ ] Build `--release` en binaire unique portable.
- [ ] Test sur Windows et Linux.