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


## 2025-08-09

### refactor: group dispatcher
- `info_group` prend maintenant `group_id` directement au lieu de `username`
- Fix double lock dans `group_dispatcher` (deadlock potentiel)
- Ajout `GroupNotFound` (404) et `NotGroupLeader` (403) dans `TapError`
- Placeholder `invite_group` créé

### feat: GROUP INFO
- Retourne les infos du groupe en JSON depuis `group_id` du joueur
- Erreur `GROUP_NOT_FOUND` si le groupe n'existe plus


## 2025-08-12

### feat: GROUP INVITE
- Vérification que l'invitant est bien le leader du groupe
- Impossible de s'inviter soi-même (`CannotInviteSelf` 409)
- Vérification que le joueur cible existe (`PlayerNotFound` 404)
- Vérification que le joueur est pas déjà dans le groupe (`PlayerAlreadyInGroup` 409)
- Ajout du joueur dans `group.invited` après vérifications
- Envoi de `EVT GROUP INVITE <username> id=<group_id>` au joueur invité via `notify_user`
- `notify_user` ajouté dans `src/events/user.rs`

### feat: GROUP JOIN
- Vérification que le joueur est invité avant de rejoindre
- Vérification que le joueur est pas déjà dans un groupe
- Ajout dans `group.players`, retrait de `group.invited`
- Mise à jour de `player.group_id`
- Broadcast `EVT GROUP JOIN <username>` à tous les membres
- Erreur `NOT_INVITED` (407) si pas invité

### feat: GROUP LEAVE
- Retrait du joueur du groupe
- Si leader quitte et membres restants → premier membre devient leader + EVT GROUP LEADER
- Si dernier membre quitte → appel disband automatique
- Mise à jour de `player.group_id` à None
- Broadcast EVT GROUP LEAVE aux membres restants

### feat: GROUP DISBAND
- Vérification que c'est le leader qui disband
- Broadcast `EVT GROUP DISBAND` avant suppression du groupe
- Reset `group_id` à None pour tous les membres
- Suppression du groupe du world
- Appelé automatiquement par `leave_group` si dernier membre quitte