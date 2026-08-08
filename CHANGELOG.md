# Journal de bord - TAP

## 2025-08-08

### feat: GROUP CREATE et GROUP INFO
- Création de groupe avec nom custom ou username par défaut
- Validation du nom (longueur, caractères autorisés)
- Système de leader — le créateur est automatiquement leader et membre
- `GROUP INFO` retourne les infos du groupe en JSON
- `has_group`, `get_group`, `add_group` ajoutés dans `impl World`

### feat: CHAT GROUP
- Broadcast au groupe via `notify_group`
- Gestion erreur `NOT_IN_GROUP` si le joueur est pas dans un groupe
- `notify_group` dans `src/events/group.rs` — même pattern que `notify_room`