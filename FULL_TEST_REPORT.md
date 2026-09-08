# TAP — Full Project Audit Report

Audit réalisé contre le sujet v1.4 et le RFC 42TAP.

---

## 1. Compilation & Build

| Check | Status |
|---|---|
| `cargo build` compile sans erreurs | ✅ PASS |
| `cargo build --release` compile | ✅ PASS |
| Aucun warning de compilation | ✅ PASS |
| Pas de Python dans le projet | ✅ PASS |
| Langage autorisé (Rust) | ✅ PASS |

---

## 2. Makefile / Build System

Le sujet exige des targets pour : install dependencies, run-server, run-client, run-client-gui, lint, clean.

| Target | Commande | Status |
|---|---|---|
| `build` | `cargo build --release` | ✅ PASS |
| `install` | `cargo fetch` | ✅ PASS |
| `run-server` | `cargo run --bin server -- src/assets/default_world.json` | ✅ PASS |
| `run-client` | `cargo run --bin client_cli` | ✅ PASS |
| `run-client-gui` | `cargo run --bin client_gui` | ✅ PASS |
| `lint` | `cargo check` | ✅ PASS |
| `clean` | `cargo clean` | ✅ PASS |

---

## 3. Protocole RFC 42TAP — Conformité Commandes

### 3.1 Connection Management

| Requirement | Status | Détails |
|---|---|---|
| Greeting `OK hello proto=1` envoyé à la connexion TCP | ✅ PASS | `server.rs:142` |
| `CONNECT <username>` → `OK connected` | ⚠️ WARN | Renvoie `OK Connected` (C majuscule) au lieu de `OK connected`. Minuscule dans le RFC. |
| `CONNECT` username déjà pris → `ERR 201 NAME_IN_USE` | ✅ PASS | |
| `QUIT` → `OK bye` | ✅ PASS | `server.rs:224` |
| Déconnexion gracieuse (cleanup player state, broadcast leave) | ✅ PASS | `disconnect.rs` retire le joueur de la room et du world |
| TCP stream buffering (messages fragmentés/coalescés) | ✅ PASS | Utilise `BufReader::read_line` qui bufferise jusqu'au `\n` |
| UTF-8 encoding | ✅ PASS | Rust String est UTF-8 natif |
| Commandes case-insensitive | ✅ PASS | `line.to_uppercase()` avant le dispatch |

### 3.2 Core Commands

| Commande | Format RFC | Implémentation | Status |
|---|---|---|---|
| `LOOK` | `OK {"room":{...}, "players":[], "items":[], "npcs":[]}` | `OK {"id":..., "name":..., "exits":..., "players":[], "items":[], "npcs":[]}` | ⚠️ WARN — Format plat au lieu de `room` imbriqué. **Documenté dans README comme déviation.** |
| `MOVE <dir>` | `OK room=<id>` / `ERR 301 NO_EXIT` | Conforme | ✅ PASS |
| `WHO` | `OK players=<count>` | Conforme | ✅ PASS |
| `QUIT` | `OK bye` | Conforme | ✅ PASS |

### 3.3 Communication Commands

| Commande | Format RFC | Implémentation | Status |
|---|---|---|---|
| `CHAT GLOBAL <msg>` | Réponse `OK`, event `EVT GLOBAL CHAT <user> <msg>` | Réponse `OK` ✅, event `(GLOBAL) user: msg` | ⚠️ WARN — Format chat non-EVT. **Documenté dans README.** |
| `CHAT ROOM <msg>` | Réponse `OK`, event `EVT ROOM CHAT <user> <msg>` | Réponse `OK` ✅, event `(ROOM) user: msg` | ⚠️ WARN — Même déviation. **Documenté.** |
| `CHAT GROUP <msg>` | Réponse `OK`, event `EVT GROUP CHAT <user> <msg>` | Réponse `OK` ✅, event `(GROUP) user: msg` | ⚠️ WARN — Même déviation. **Documenté.** |

### 3.4 Group Commands

| Commande | Format RFC | Status |
|---|---|---|
| `GROUP CREATE` | `OK group=<id>` | ✅ PASS |
| `GROUP INVITE <user>` | `OK` + event `EVT GROUP INVITE <leader>` | ✅ PASS — Event envoie aussi `id=<group_id>` (extension) |
| `GROUP JOIN <id>` | `OK group=<id>` + event `EVT GROUP JOIN <user>` | ✅ PASS |
| `GROUP LEAVE` | `OK` + event `EVT GROUP LEAVE <user>` | ✅ PASS |
| `GROUP DISBAND` (extension) | `OK` | ✅ PASS — Documenté |
| `GROUP KICK` (extension) | `OK` | ✅ PASS — Documenté |
| `GROUP INFO` (extension) | `OK <json>` | ✅ PASS — Documenté |

### 3.5 Resource Interaction Commands

| Commande | Format RFC | Status |
|---|---|---|
| `TAKE <item>` → `OK taken=<id>` | Conforme | ✅ PASS |
| `TAKE` item inexistant → `ERR 404 ITEM_NOT_FOUND` | Conforme | ✅ PASS |
| `DROP <item>` → `OK dropped=<id>` | Conforme | ✅ PASS |
| `DROP` pas en inventaire → `ERR 404 ITEM_NOT_IN_INVENTORY` | Conforme | ✅ PASS |
| `INVENTORY` → `OK [json array]` | Conforme | ✅ PASS |
| `TALK <npc>` → `OK <dialogue>` | ⚠️ WARN — Renvoie JSON `{"npc":"...","dialogue":"..."}` au lieu de texte brut. **Documenté.** |
| `TALK` NPC absent → `ERR 404 NPC_NOT_FOUND` | Conforme | ✅ PASS |
| `ATTACK <npc>` → `OK <combat-json>` | ✅ PASS — JSON avec `attacker_hp`, `target_hp`, `damage`, `status` |
| `ATTACK` NPC absent → `ERR 404 NPC_NOT_FOUND` | ✅ PASS |
| `ATTACK` NPC friendly → `ERR 405 NPC_NOT_HOSTILE` | ✅ PASS |
| `STATUS` → `OK {"hp":..., "max_hp":..., "status":"..."}` | ✅ PASS — Inclut aussi `gold` (extension) |
| `QUEST <npc>` → quest data / `ERR 404` / `ERR 406` | ✅ PASS |
| `QUESTS` → liste JSON des quêtes | ✅ PASS |

### 3.6 Extension Commands (hors RFC, documentés)

| Commande | Status |
|---|---|
| `SLEEP` — restaure HP dans la sleep room | ✅ PASS — Documenté |
| `EXAMINE <item>` — détails item | ✅ PASS — Documenté |
| `USE <item>` — utiliser item de soin | ✅ PASS — Documenté |
| `SHOP` / `SHOP BUY` — magasin | ✅ PASS — Documenté |
| `MARKET` / `MARKET BUY` / `SELL` / `MARKET CANCEL` — marché joueurs | ✅ PASS — Documenté |

---

## 4. Event System

### 4.1 Events RFC obligatoires

| Event | Format RFC | Status |
|---|---|---|
| `EVT ROOM PRESENCE ENTER <user>` | Conforme | ✅ PASS |
| `EVT ROOM PRESENCE LEAVE <user>` | Conforme | ✅ PASS |
| `EVT ROOM CHAT <user> <msg>` | `(ROOM) user: msg` | ⚠️ WARN — Déviation documentée |
| `EVT GLOBAL CHAT <user> <msg>` | `(GLOBAL) user: msg` | ⚠️ WARN — Déviation documentée |
| `EVT GROUP INVITE <leader>` | Conforme (+ `id=<group_id>`) | ✅ PASS |
| `EVT GROUP JOIN <user>` | Conforme | ✅ PASS |
| `EVT GROUP LEAVE <user>` | Conforme | ✅ PASS |
| `EVT GROUP CHAT <user> <msg>` | `(GROUP) user: msg` | ⚠️ WARN — Déviation documentée |
| `EVT STATS players=<count>` | **Non implémenté** | ❌ FAIL |

### 4.2 Events custom (extensions)

| Event | Status |
|---|---|
| `EVT ROOM COMBAT ...` (attaque, victoire, mort) | ✅ PASS |
| `EVT ROOM ITEM TAKEN/DROPPED/RESPAWN` | ✅ PASS |
| `EVT GLOBAL [ALERT] ...` (boss spawn) | ✅ PASS |
| `EVT GLOBAL [DEFEAT] ...` (boss kill) | ✅ PASS |
| `EVT GROUP LEADER/DISBAND/KICK` | ✅ PASS |
| `EVT MARKET SOLD ...` | ✅ PASS |
| `EVT SLEEP <user>` | ✅ PASS |

---

## 5. Error Codes

| Code | Message RFC | Implémenté | Status |
|---|---|---|---|
| 201 | NAME_IN_USE | ✅ | ✅ PASS |
| 301 | NO_EXIT | ✅ | ✅ PASS |
| 401 | NOT_IN_GROUP | ✅ | ✅ PASS |
| 402 | ALREADY_IN_GROUP | ✅ | ✅ PASS |
| 404 | ITEM_NOT_FOUND | ✅ | ✅ PASS |
| 404 | ITEM_NOT_IN_INVENTORY | ✅ | ✅ PASS |
| 404 | NPC_NOT_FOUND | ✅ | ✅ PASS |
| 405 | NPC_NOT_HOSTILE | ✅ | ✅ PASS |
| 406 | NO_QUEST_AVAILABLE | ✅ | ✅ PASS |
| 900 | CONNECTION_FAILED | ✅ (défini) | ✅ PASS |
| 901 | SEND_FAILED | ✅ | ✅ PASS |

Codes additionnels (extensions documentées) : 000, 202, 211, 212, 403, 407, 408, 409, 410, 411, 412, 902.

---

## 6. World Data

### 6.1 Requirements du sujet

| Requirement | Minimum | Actuel | Status |
|---|---|---|---|
| Rooms interconnectées | ≥ 8 | 18 | ✅ PASS |
| Loops dans la map | ≥ 1 | 3+ circuits | ✅ PASS |
| Branches | ≥ 1 | 4 dead-ends (guest_room, lair, catacombs, beach) | ✅ PASS |
| NPC roles distincts | ≥ 3 | 4 (friendly, quest-giver, hostile, boss) | ✅ PASS |
| Items distincts | ≥ 4 | 19 | ✅ PASS |
| Items obtenables in-world | ≥ 2 | 14 (dans les rooms) | ✅ PASS |
| Quêtes simples | ≥ 2 | 4 | ✅ PASS |
| Full circuit possible | oui | oui (square→market→docks→lighthouse→gate→square) | ✅ PASS |

### 6.2 Validation au démarrage

| Validation | Status |
|---|---|
| Spawn room existe | ✅ PASS |
| Sleep room existe | ✅ PASS |
| Quest target_item existe | ✅ PASS |
| Quest reward existe | ✅ PASS |
| Quest giver NPC existe | ✅ PASS |
| Shop items existent | ✅ PASS |
| Room items existent | ✅ PASS |
| Room NPCs existent | ✅ PASS |
| NPC quest references existent | ✅ PASS |
| NPC boss_room references existent | ✅ PASS |
| **Room exits pointent vers des rooms valides** | ❌ FAIL — Non validé |

---

## 7. Combat System

| Requirement | Status |
|---|---|
| Players start 100 HP | ✅ PASS (`lib.rs:60`) |
| Enemy NPCs ont des HP variables | ✅ PASS (25-500 HP) |
| ATTACK deals damage + counter-attack | ✅ PASS |
| STATUS montre HP et statut | ✅ PASS |
| Mort → respawn au spawn room avec HP réduit | ✅ PASS (50 HP à room.gate) |
| Combat results broadcast aux joueurs de la room | ✅ PASS |
| Armor system (flat reduction, min 1 damage) | ✅ PASS |
| Best weapon auto-selected | ✅ PASS |
| NPC respawn 30s après mort | ✅ PASS |
| Boss ne respawn pas automatiquement (via spawner) | ✅ PASS |
| Boss defeat broadcast global | ✅ PASS |

---

## 8. Server Logging

Le sujet exige un logging complet et structuré.

| Requirement | Status |
|---|---|
| Log connexions/déconnexions avec timestamp + IP | ✅ PASS |
| Log chaque commande reçue avec player name | ✅ PASS |
| Log toutes les réponses et error codes | ✅ PASS |
| Log changements d'état (items, NPCs, combat) | ✅ PASS |
| Log progression et complétion de quêtes | ✅ PASS |
| Format structuré (JSON recommandé) | ✅ PASS — `tracing_subscriber::fmt().json()` |
| Niveaux de log (INFO, WARN, ERROR) | ✅ PASS |
| Monitoring des patterns d'abus (flooding) | ✅ PASS |
| Timestamps précis | ✅ PASS — tracing les ajoute automatiquement |
| Ne pas impacter la performance | ✅ PASS — async, logging non-bloquant |

---

## 9. CLI Client

| Requirement | Status |
|---|---|
| Se connecte au serveur TCP | ✅ PASS |
| Affiche les messages en temps réel | ✅ PASS (tokio spawn pour la lecture) |
| Reçoit les events pendant l'attente d'input | ✅ PASS (reader et writer séparés) |
| Commandes en syntaxe RFC | ✅ PASS (envoi direct) |
| Autocomplétion des commandes | ✅ PASS (rustyline) |
| Syntax highlighting | ✅ PASS (vert=valide, rouge=invalide) |

---

## 10. GUI Client

| Requirement | Status |
|---|---|
| Affiche room details, items, NPCs, exits | ✅ PASS |
| Updates en temps réel | ✅ PASS (WebSocket) |
| Inventaire avec gestion (TAKE/DROP buttons) | ✅ PASS |
| Gère IDs et display names | ✅ PASS |
| Update room view après TAKE/DROP | ✅ PASS (refresh automatique) |
| Chat séparé (Global, Room, Group) / log view | ✅ PASS (tabs + log pane) |
| Boutons pour actions | ✅ PASS (move, talk, attack, quest, etc.) |
| Compteurs joueurs (room + server) | ✅ PASS (topbar) |
| Interaction NPC via TALK avec dialogue | ✅ PASS (popover NPC) |

---

## 11. README

| Section requise | Présente | Status |
|---|---|---|
| Première ligne italique `This project has been created...` | ✅ | ✅ PASS |
| Description | ✅ | ✅ PASS |
| Instructions | ✅ | ✅ PASS |
| Resources (+ AI usage) | ✅ | ✅ PASS |
| Architecture | ✅ | ✅ PASS |
| Protocol Implementation | ✅ | ✅ PASS |
| Combat System | ✅ | ✅ PASS |
| Quest System | ✅ | ✅ PASS |
| World Design | ✅ | ✅ PASS |
| Server Logging | ✅ | ✅ PASS |
| Group Contributions | ✅ | ✅ PASS |
| Building and Running | ✅ | ✅ PASS |
| Testing | ✅ | ✅ PASS |
| Écrit en anglais | ✅ | ✅ PASS |
| Déviations du protocole documentées | ✅ | ✅ PASS |

---

## 12. Bugs & Edge Cases

### 12.1 ❌ BUGS CONFIRMÉS

#### BUG-1 : `EVT STATS players=<count>` non implémenté
- **Sévérité** : Moyenne
- **RFC** : Section 6.2.4 définit `EVT STATS players=<count>` comme event obligatoire
- **Problème** : Aucun endroit dans le code ne broadcast cet event. Quand un joueur se connecte ou se déconnecte, le compteur n'est pas envoyé aux autres clients.
- **Fix** : Ajouter un broadcast `EVT STATS players=<count>` dans `handle_connect` (après ajout au world) et `handle_disconnect` (après suppression).

#### BUG-2 : Déconnexion ne nettoie pas le groupe
- **Sévérité** : Haute
- **Problème** : `handle_disconnect` (`disconnect.rs`) retire le joueur du world et de la room, mais ne le retire PAS de son groupe. Si un joueur est dans un groupe et se déconnecte (ou perd la connexion), il reste dans `group.players` comme un fantôme.
- **Impact** : Le groupe affiche un membre qui n'existe plus. `CHAT GROUP` essaiera de lui envoyer un message (échouera silencieusement). `GROUP INFO` le listera toujours.
- **Fix** : Dans `handle_disconnect`, si le joueur a un `group_id`, appeler la logique de `leave_group` pour le retirer proprement.

#### BUG-3 : Validation des exits manquante au démarrage
- **Sévérité** : Basse (pas de bug actuellement car le world.json est correct)
- **Problème** : Le serveur valide les items, NPCs, quêtes, boss_room... mais ne valide PAS que les `exits` des rooms pointent vers des rooms qui existent réellement.
- **Impact** : Si le world.json a un exit vers une room inexistante, le joueur recevra `ERR 301 NO_EXIT` en essayant d'aller dans une direction qui devrait exister visuellement (via LOOK).
- **Fix** : Ajouter une boucle de validation dans `server.rs` :
```rust
for (rid, room) in &world.rooms {
    for (dir, target) in &room.exits {
        if !world.rooms.contains_key(target) {
            fatal(&format!("Room {} exit {} points to unknown room: {}", rid, dir, target));
        }
    }
}
```

#### BUG-4 : `CONNECT` renvoie `OK Connected` au lieu de `OK connected`
- **Sévérité** : Basse
- **Problème** : Le RFC dit `OK connected` (minuscule). Le code renvoie `OK Connected` (majuscule).
- **Impact** : Un client RFC-strict d'un autre groupe pourrait ne pas reconnaître la réponse.
- **Fix** : `connect.rs:21` → changer `"OK Connected\n"` en `"OK connected\n"`.

### 12.2 ⚠️ WARNINGS (déviations documentées)

| # | Déviation | Documenté dans README |
|---|---|---|
| W-1 | Chat events en `(SCOPE) user: msg` au lieu de `EVT SCOPE CHAT user msg` | ✅ Oui |
| W-2 | LOOK format plat au lieu de `{"room":{...}}` imbriqué | ✅ Oui |
| W-3 | TALK renvoie du JSON au lieu de texte brut | ✅ Oui |
| W-4 | Extensions non-RFC (SLEEP, EXAMINE, USE, SHOP, MARKET, GROUP KICK/DISBAND/INFO) | ✅ Oui |

### 12.3 ⚠️ EDGE CASES POTENTIELS

#### EC-1 : NPC/Item matching trop permissif
- **Problème** : Le matching utilise `id.contains(npc_id)`. Si un joueur tape `ATTACK a`, cela matchera le premier NPC dont l'id contient "a".
- **Impact** : Ambiguïté si deux NPCs ont des IDs similaires. Ex: `npc.guard` et `npc.guardian` — taper `guard` matchera le premier trouvé.
- **Sévérité** : Basse (pas de collision dans le world actuel)

#### EC-2 : Pas de limite de taille de message
- **Problème** : Le RFC recommande 1024 bytes max par ligne. Le serveur ne vérifie pas.
- **Impact** : Un client malveillant pourrait envoyer des lignes très longues pour consommer de la mémoire.
- **Sévérité** : Basse

#### EC-3 : `market` indices instables après achat/annulation
- **Problème** : Les listings du market sont indexés par position dans le Vec. Quand un listing est retiré (achat/annulation), tous les indices suivants changent.
- **Impact** : Si deux joueurs regardent le market en même temps et l'un achète l'index 0, l'autre joueur a des indices décalés. Il pourrait acheter le mauvais item.
- **Sévérité** : Basse (le mutex empêche les races, mais l'UI du joueur aura des données périmées)

#### EC-4 : Flood protection dans le README ne correspond pas au code
- **Problème** : Le README dit "10 cmd/s warning, 20 cmd/s kick". Le code utilise une fenêtre de 2s avec seuils 25 (warning) et 40 (kick).
- **Sévérité** : Cosmétique — mettre à jour le README.

---

## 13. README vs World.json — Incohérences

Le README décrit un monde plus grand que ce qui existe dans `world.json` / `default_world.json` :

| README mentionne | world.json | Status |
|---|---|---|
| 21 rooms | 18 rooms | ❌ Incohérent |
| 21 NPCs | 13 NPCs | ❌ Incohérent |
| 26 items | 19 items | ❌ Incohérent |
| `room.inn` (Traveler's Inn) | N'existe pas — sleep room = `room.guest_room` | ❌ Incohérent |
| `room.peak` (Frozen Peak) | N'existe pas | ❌ Incohérent |
| `room.grove` (Sacred Grove) | N'existe pas | ❌ Incohérent |
| `room.swamp` (Murky Swamp) | N'existe pas | ❌ Incohérent |
| `room.deep_forest` (Deep Forest) | N'existe pas | ❌ Incohérent |
| Giant Crab, Wild Boar, Swamp Serpent, Cave Spider, Forest Bandit, Ancient Wraith NPCs | N'existent pas | ❌ Incohérent |
| Health Potion, Phoenix Feather, Glowing Mushroom, Sea Pearl, Bone Club, Shadow Dagger, etc. | N'existent pas | ❌ Incohérent |
| `quest.pearl`, `quest.mushroom` / `quest.sword` | N'existent pas (quêtes actuelles: herbs, iron, crystal, treasure) | ❌ Incohérent |

**Sévérité : Haute** — Un évaluateur qui lit le README puis teste le jeu verra que les rooms, NPCs et items décrits n'existent pas.

**Action requise** : Soit ajouter les rooms/NPCs/items manquants au world.json, soit réécrire les sections World Design, NPC Stats, Items, et Quest System du README pour correspondre au monde réel.

---

## 14. Résumé

### Statistiques

| Catégorie | Pass | Warn | Fail |
|---|---|---|---|
| Protocol commands | 14 | 4 | 0 |
| Events | 12 | 4 | 1 |
| Error codes | 11 | 0 | 0 |
| World data | 10 | 0 | 1 |
| Server features | 10 | 0 | 0 |
| CLI client | 6 | 0 | 0 |
| GUI client | 9 | 0 | 0 |
| README sections | 14 | 0 | 0 |
| **Total** | **86** | **8** | **2** |

### Actions prioritaires

1. **🔴 CRITIQUE — README vs World.json** : Le README décrit un monde de 21 rooms/21 NPCs/26 items qui n'existe pas. Synchroniser le README avec le monde réel OU compléter le world.json.
2. **🔴 BUG — Déconnexion ne nettoie pas le groupe** : Ajouter le cleanup de groupe dans `handle_disconnect`.
3. **🟡 MANQUANT — `EVT STATS players=<count>`** : Ajouter le broadcast lors des connect/disconnect.
4. **🟡 MINOR — `OK Connected` → `OK connected`** : Corriger la casse dans `connect.rs`.
5. **🟡 MINOR — Validation des exits** : Ajouter la vérification au démarrage.
6. **🟡 COSMÉTIQUE — Flood thresholds dans README** : Mettre à jour pour correspondre au code (2s/25warn/40kick).
