*This project has been created as part of the 42 curriculum by nbilyj, nbarbosa.*

# TAP - The Answer Protocol

A multiplayer MUD (Multi-User Dungeon) game server and clients built over TCP, implementing the RFC 42TAP protocol. Players connect to a shared fantasy world where they can explore rooms, fight NPCs, collect items, complete quests, chat with other players, and form groups. The game world is persistent and shared: every player sees the same rooms, items, and NPCs in real time. When a player picks up an item, it disappears for everyone; when a player enters a room, all other players in that room are notified.

## Instructions

TAP is composed of three binaries:

- **server** — the game server that manages the world state, handles player connections, and enforces the TAP protocol over TCP (port 7534). It accepts any number of simultaneous clients. The world data (rooms, NPCs, items, quests) is loaded from a JSON file at startup.
- **client_cli** — a terminal client with autocompletion and syntax highlighting powered by rustyline. It connects to a TAP server via TCP and provides an interactive prompt where the user types commands. Commands are highlighted green if valid, red if unknown. Tab-completion suggests all available commands including subcommands like `CHAT GLOBAL` or `GROUP INVITE`.
- **client_gui** — a web-based GUI client using Axum as the HTTP server and WebSocket as the bridge to the TCP server. The browser connects via WebSocket to the Axum server, which then opens a TCP connection to the TAP server and relays messages in both directions. The GUI is served as static files (HTML/CSS/JS) on `http://127.0.0.1:3000`. It features a visual room display, clickable direction buttons, a chat panel, inventory display, and an interactive map.

All communication follows the line-oriented TAP protocol: UTF-8 text, one command per line, terminated by `LF` (0x0A). Responses start with `OK` on success or `ERR <code> <CONSTANT>` on failure.

## Resources

### External Libraries

| Crate | Purpose |
|---|---|
| tokio | Async runtime for the server, providing the TCP listener, async I/O, timers (used for item respawn delays, boss spawn interval, NPC regeneration), and task spawning for each client connection |
| serde / serde_json | JSON serialization and deserialization for the world data file, as well as formatting command responses (LOOK, STATUS, INVENTORY, EXAMINE, QUEST, GROUP INFO all return JSON) |
| tracing / tracing-subscriber | Structured JSON logging with severity levels (INFO, WARN, ERROR) and environment-based filtering via `RUST_LOG` |
| rustyline | Readline library for the CLI client, providing line editing, command history, tab-completion, and syntax highlighting |
| axum | HTTP and WebSocket server framework for the GUI client, handling the WebSocket upgrade and static file fallback routing |
| tower-http | Middleware for serving the `static/` directory as static files (HTML, CSS, JS) for the web GUI |
| futures-util | Stream and sink utilities used for splitting the WebSocket connection into sender/receiver halves |

### AI Usage

Claude was used as an assistant during writing this README. All code logic, architecture decisions, and protocol design were made by the team.

## Architecture

```
project/
├── src/
│   ├── lib.rs                  # Core data structures (Room, Player, Npc, Item, Quest, Group, World, TapError)
│   ├── utils.rs                # Utility functions (username/group validation, arg parsing)
│   ├── bin/
│   │   ├── server.rs           # TCP server main loop, command dispatcher, flood detection
│   │   ├── client_cli.rs       # Terminal client with rustyline
│   │   └── client_gui.rs       # Axum WebSocket-to-TCP bridge for the GUI
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── connect.rs          # CONNECT — player authentication
│   │   ├── disconnect.rs       # Cleanup on disconnect
│   │   ├── look.rs             # LOOK — room description
│   │   ├── movement.rs         # MOVE — room navigation
│   │   ├── who.rs              # WHO — online player list
│   │   ├── chat.rs             # CHAT GLOBAL/ROOM/GROUP
│   │   ├── talk.rs             # TALK — NPC dialogue
│   │   ├── take.rs             # TAKE — pick up items
│   │   ├── drop.rs             # DROP — drop items
│   │   ├── inventory.rs        # INVENTORY — player inventory
│   │   ├── examine.rs          # EXAMINE — item details
│   │   ├── attack.rs           # ATTACK — combat with hostile NPCs
│   │   ├── status.rs           # STATUS — player HP and state
│   │   ├── quest.rs            # QUEST/QUESTS — quest system
│   │   ├── sleep.rs            # SLEEP — HP recovery
│   │   ├── shop.rs             # SHOP/SHOP BUY — game shop
│   │   ├── market.rs           # MARKET/MARKET BUY/SELL — player market
│   │   ├── group_dispatcher.rs # GROUP subcommand router
│   │   └── group/
│   │       ├── mod.rs
│   │       ├── create.rs       # GROUP CREATE
│   │       ├── invite.rs       # GROUP INVITE
│   │       ├── join.rs         # GROUP JOIN
│   │       ├── leave.rs        # GROUP LEAVE
│   │       ├── disband.rs      # GROUP DISBAND
│   │       ├── kick.rs         # GROUP KICK
│   │       └── info.rs         # GROUP INFO
│   ├── events/
│   │   ├── mod.rs
│   │   ├── room.rs             # Room-scoped event broadcasting
│   │   ├── global.rs           # Global event broadcasting
│   │   ├── group.rs            # Group event broadcasting
│   │   ├── user.rs             # User-targeted events
│   │   ├── boss.rs             # Data-driven boss spawner (reads from world JSON)
│   │   └── regen.rs            # NPC HP regeneration (60s after last hit)
│   └── assets/
│       └── default_world.json  # World data (rooms, NPCs, items, quests)
├── static/
│   ├── index.html              # Web GUI HTML
│   ├── script.js               # Web GUI logic
│   └── style.css               # Web GUI styles
├── Cargo.toml
└── Makefile
```

### Server Design

The server follows a **single-process, multi-task async** architecture using tokio:

- **Shared world state**: The entire game world (`World` struct containing rooms, players, NPCs, items, quests, groups) is stored in a single `Arc<Mutex<World>>`. Every command handler locks this mutex, performs its operation, and releases it. This guarantees consistency: two players cannot take the same item simultaneously.
- **Client connection handling**: Each TCP connection spawns a dedicated tokio task. The task reads lines from the socket in a loop and dispatches each command to the appropriate handler function. A command dispatcher in `server.rs` pattern-matches the first word of each line (case-insensitive) to route to the correct handler.
- **Message delivery**: Each client has its own `mpsc::unbounded_channel`. The sender half is stored in a global `registry` (`HashMap<String, UnboundedSender<String>>`), keyed by username. A separate tokio task per client reads from the receiver half and writes messages to the TCP socket. This allows any handler or event to send messages to any connected player by looking up their sender in the registry.
- **Event broadcasting**: Events (room presence, chat, combat, item changes) are broadcast by iterating over relevant players in the world state, looking up each player's sender in the registry, and sending the event string. Room events go to all players in the same room (optionally excluding the triggering player). Global events go to all registered players. Group events go to all group members.
- **Background tasks**: The server spawns long-running tokio tasks for the boss spawner (every 60 seconds) and NPC HP regeneration (checks every 15 seconds, heals NPCs that haven't been hit for 60 seconds). Short-lived tasks are spawned for item respawn (30-second delay) and NPC respawn (30-second delay after being killed).
- **Startup validation**: At startup, the server verifies that the spawn room and sleep room exist in the world data, and that all quests reference valid NPCs and items. If any validation fails, the server exits immediately with an error message.

## Protocol Implementation

The server implements RFC 42TAP v1. Below is a detailed description of every command, its behavior, its response format, and its error cases.

### Connection Flow

1. Client connects via TCP to port 7534
2. Server immediately sends the greeting: `OK hello proto=1`
3. Client must send `CONNECT <username>` to authenticate before any other command
4. Any command sent before `CONNECT` (except `QUIT`) is silently ignored
5. After successful `CONNECT`, the player enters the AUTHENTICATED state and can use all commands
6. `QUIT` terminates the session — the server sends `OK bye` and closes the connection
7. If the TCP connection drops without `QUIT`, the server performs the same cleanup (remove player from world, notify room)

### Detailed Command Reference

#### `CONNECT <username>`

Authenticates the player and enters the game world. The username must be 1-20 characters long and contain only alphanumeric characters, underscores, or hyphens. The player spawns in the spawn room (default: `room.gate` — South Gate) with 100 HP and an empty inventory. All players currently in the spawn room receive an `EVT ROOM PRESENCE ENTER <username>` notification. If the player is already connected (sent CONNECT twice), the server responds with `ERR already connected`. Cannot be used while already authenticated.

- **Success**: `OK Connected`
- **Errors**: `ERR 201 NAME_IN_USE` (username taken), `ERR 202 INVALID_USER_NAME` (invalid format)

#### `LOOK`

Returns a JSON description of the current room, including its name, description, exits, list of players present, items on the ground, and NPCs. If the player is in the designated sleep room (`room.guest_room`), the response includes an additional `can_sleep: true` field to indicate the SLEEP command is available.

- **Success**: `OK {"id":"room.square","name":"Village Square","description":"A bustling square...","exits":{"north":"room.tavern","east":"room.market",...},"players":["alice"],"items":[],"npcs":["npc.guard"]}`
- **Success (sleep room)**: same format with `"can_sleep":true` added

#### `MOVE <direction>`

Moves the player to an adjacent room in the specified direction (north, south, east, west). The direction is case-insensitive. The player is removed from the current room's player list and added to the destination room's player list. Two events are broadcast: `EVT ROOM PRESENCE LEAVE <username>` to all other players in the old room, and `EVT ROOM PRESENCE ENTER <username>` to all other players in the new room.

- **Success**: `OK room=<new_room_id>`
- **Errors**: `ERR 301 NO_EXIT` (no exit in that direction)

#### `WHO`

Returns the total number of connected players on the server.

- **Success**: `OK players=<count>`

#### `CHAT GLOBAL <message>`

Sends a message to every connected player on the server. The message is broadcast as `(GLOBAL) <username>: <message>` to all players including the sender.

- **Success**: `OK`

#### `CHAT ROOM <message>`

Sends a message to all players in the same room. The message is broadcast as `(ROOM) <username>: <message>` to every player in the room (excluding the sender from the event, but the sender receives the `OK` confirmation).

- **Success**: `OK`

#### `CHAT GROUP <message>`

Sends a message to all members of the player's group. The message is broadcast as `(GROUP) <username>: <message>` to every group member. The player must be in a group.

- **Success**: `OK`
- **Errors**: `ERR 401 NOT_IN_GROUP` (player is not in any group)

#### `TALK <npc>`

Initiates dialogue with an NPC present in the current room. The NPC is matched by partial ID (e.g., `TALK guard` matches `npc.guard`). The server returns the NPC's name and one line of dialogue selected from their dialogue list (the first entry). NPCs can be friendly or hostile — both can be talked to.

- **Success**: `OK {"npc": "<NPC name>", "dialogue": "<dialogue line>"}`
- **Errors**: `ERR 404 NPC_NOT_FOUND` (NPC not in room or doesn't exist)

#### `TAKE <item>`

Picks up an item from the current room and adds it to the player's inventory. The item is matched by partial ID (e.g., `TAKE sword` matches `item.sword`). The item is removed from the room's item list. All other players in the room receive `EVT ROOM ITEM TAKEN <item_id> <username>`. After 30 seconds, the item automatically respawns in its original room (if not already present), and all players in that room receive `EVT ROOM ITEM RESPAWN <item_id>`.

- **Success**: `OK taken=<item_id>`
- **Errors**: `ERR 404 ITEM_NOT_FOUND` (item not in room)

#### `DROP <item>`

Drops an item from the player's inventory onto the ground in the current room. The item is matched by partial ID. Only one copy of the item is removed if the player has duplicates. The item is added to the room's item list. All other players in the room receive `EVT ROOM ITEM DROPPED <item_id> <username>`.

- **Success**: `OK dropped=<item_id>`
- **Errors**: `ERR 404 ITEM_NOT_IN_INVENTORY` (item not in player's inventory)

#### `INVENTORY`

Returns the player's full inventory as a JSON array of item IDs.

- **Success**: `OK ["item.sword","item.herbs","item.ale"]`
- **Success (empty)**: `OK []`

#### `EXAMINE <item>`

Inspects an item to see its properties. The server first looks for the item in the current room, then in the player's inventory. The item is matched by partial ID. Returns a JSON object with the item's full details including name, damage, armor, and heal values.

- **Success**: `OK {"id":"item.sword","name":"Iron Sword","damage":15,"armor":null,"heal":null}`
- **Errors**: `ERR 404 ITEM_NOT_FOUND` (item not in room or inventory)

#### `ATTACK <npc>`

Attacks a hostile NPC in the current room. The NPC is matched by partial ID. The player deals base 10 damage plus the highest weapon damage bonus from their inventory (the best weapon is automatically selected). The NPC counter-attacks immediately, dealing its own damage to the player. Both HP values are updated simultaneously. The result depends on whether either combatant reaches 0 HP:

- **Normal hit**: Both player and NPC survive. Returns damage dealt and remaining HP.
- **NPC defeated** (`target_hp` reaches 0): The NPC is removed from the room. It respawns in the same room after 30 seconds with full HP restored. The room receives `EVT ROOM COMBAT <npc_name> defeated by <username> npc=<npc_id>`.
- **Player killed** (`attacker_hp` reaches 0): The player is moved to the spawn room (`room.gate`) and revived with 50 HP. Their status resets to Alive. The old room receives `EVT ROOM COMBAT <username> was killed by <npc_name>` and `EVT ROOM PRESENCE LEAVE <username>`. The spawn room receives `EVT ROOM PRESENCE ENTER <username>`.

All attacks broadcast `EVT ROOM COMBAT <username> attacks <npc_name> for <damage> damage npc=<npc_id> hp=<remaining_hp>` to other players in the room, allowing their clients to display real-time NPC health updates.

- **Success (combat continues)**: `OK {"attacker_hp": <player_hp>, "target_hp": <npc_hp>, "damage": <player_damage>, "status": "combat"}`
- **Success (NPC dies)**: `OK {"attacker_hp": <player_hp>, "target_hp": 0, "damage": <player_damage>, "status": "victory"}`
- **Success (player dies)**: `OK {"attacker_hp": 0, "target_hp": <npc_hp>, "damage": <player_damage>, "status": "death", "respawn_room": "<room_id>", "respawn_hp": 50}`
- **Errors**: `ERR 404 NPC_NOT_FOUND`, `ERR 405 NPC_NOT_HOSTILE` (NPC is friendly), `ERR 409 PLAYER_DEAD`

#### `STATUS`

Returns the player's current HP, maximum HP, and alive/dead status as JSON.

- **Success**: `OK {"hp": 85, "max_hp": 100, "gold": 50, "status": "alive"}`

#### `QUEST <npc>`

Interacts with a quest-giving NPC in the current room. This command has two behaviors depending on the player's quest state:

1. **Accepting a quest**: If the player hasn't accepted the NPC's quest yet, the quest is added to their active quest list. The server returns the full quest details as JSON (description, target item, target count, reward).
2. **Turning in a quest**: If the player has the quest active and has collected enough of the required items, the quest items are consumed from the inventory, the reward items are added, and the quest is marked as completed. The server returns a completion confirmation with the reward details.

Each quest can only be completed once per player. If the player has already completed the NPC's quest, the server returns an error.

- **Success (accept)**: `OK {"id":"quest.herbs","description":"Bring 1 healing herbs...","giver":"npc.hermit","type":"fetch","target_item":"item.herbs","target_count":1,"reward":"item.crystal","reward_count":1}`
- **Success (turn in)**: `OK {"quest_id": "quest.herbs", "status": "completed", "reward": "item.crystal", "reward_count": 1}`
- **Errors**: `ERR 404 NPC_NOT_FOUND`, `ERR 406 NO_QUEST_AVAILABLE` (NPC has no quest, or quest already done), `ERR 408 QUEST_NOT_COMPLETE` (not enough items)

#### `QUESTS`

Lists all the player's active and completed quests as a JSON array. Active quests include progress tracking showing how many target items the player currently has versus the required count.

- **Success**: `OK [{"quest_id":"quest.herbs","status":"active","progress":"0/1"},{"quest_id":"quest.iron","status":"completed"}]`
- **Success (no quests)**: `OK []`

#### `SLEEP`

Restores the player's HP to maximum. This command only works in the designated sleep room (`room.guest_room` — the Guest Room upstairs in the tavern). All other players in the room receive `EVT SLEEP <username>`. Dead players cannot sleep.

- **Success**: `OK hp=100/100 You feel really good now !`
- **Errors**: `ERR 410 CANNOT_SLEEP_HERE` (not in the sleep room), `ERR 409 PLAYER_DEAD`

#### `SHOP`

Lists the game shop catalog. The shop has an infinite stock — items are always available for purchase. Each entry includes the item's id, name, price, and combat stats.

- **Success**: `OK [{"id":"item.sword","name":"Iron Sword","price":50,"damage":15,"armor":null,"heal":null}, ...]`

#### `SHOP BUY <item_id>`

Purchases an item from the game shop. The item is matched by partial ID. The item's `value` field determines the price. The player must have enough gold.

- **Success**: `OK bought=<item_id> price=<amount> gold=<remaining_gold>`
- **Errors**: `ERR 404 ITEM_NOT_FOUND`, `ERR 412 NOT_ENOUGH_GOLD`

#### `SELL <item_id>`

Puts an item from the player's inventory up for sale on the player market. The item is removed from the inventory and listed at its base value. Other players (and the seller) can then buy it with `MARKET BUY`.

- **Success**: `OK listed=<item_id> price=<amount>`
- **Errors**: `ERR 404 ITEM_NOT_IN_INVENTORY`

#### `MARKET`

Lists all items currently for sale on the player market. Each entry includes the item details, seller name, price, and a numeric index used for purchasing.

- **Success**: `OK [{"index":0,"item_id":"item.sword","name":"Iron Sword","seller":"bob","price":50,"damage":15,"armor":null,"heal":null}, ...]`
- **Success (empty)**: `OK []`

#### `MARKET BUY <index>`

Purchases an item from the player market by its listing index (from the `MARKET` response). The buyer's gold is deducted and the seller's gold is credited. The item is added to the buyer's inventory and removed from the market.

- **Success**: `OK bought=<item_id> price=<amount> seller=<name> gold=<remaining_gold>`
- **Errors**: `ERR 404 ITEM_NOT_FOUND` (invalid index), `ERR 412 NOT_ENOUGH_GOLD`

#### `GROUP CREATE <name>`

Creates a new group with the specified name. The player becomes the group leader and the only member. The name must be 1-25 characters and contain only alphanumeric characters, underscores, or hyphens. If no name is provided, the group is named after the player. The player must not already be in a group.

- **Success**: `OK group=<group_name>`
- **Errors**: `ERR 211 GROUP_NAME_IN_USE`, `ERR 212 INVALID_GROUP_NAME`, `ERR 402 ALREADY_IN_GROUP`

#### `GROUP INVITE <player>`

Invites another player to join the group. Only the group leader can invite. The target player must exist, must not already be in the group, and must not already have a pending invite. The invited player receives `EVT GROUP INVITE <inviter> id=<group_id>`. A player cannot invite themselves.

- **Success**: `OK`
- **Errors**: `ERR 403 NOT_GROUP_LEADER`, `ERR 404 PLAYER_NOT_FOUND`, `ERR 402 PLAYER_ALREADY_IN_GROUP` (already a member), `ERR 402 ALREADY_IN_GROUP` (already invited), `ERR 407 CANNOT_INVITE_SELF`

#### `GROUP JOIN <group_id>`

Joins a group the player was invited to. The player must have received an invite (via `EVT GROUP INVITE`) and must not already be in a group. Upon joining, all group members receive `EVT GROUP JOIN <username>`.

- **Success**: `OK group=<group_id>`
- **Errors**: `ERR 402 ALREADY_IN_GROUP` (already in another group), `ERR 404 GROUP_NOT_FOUND`, `ERR 407 NOT_INVITED` (no pending invite)

#### `GROUP LEAVE`

Leaves the current group. If the leaving player is the group leader, leadership transfers to the first remaining member and the group receives `EVT GROUP LEADER <new_leader>`. If the group is now empty after the player leaves, the group is automatically disbanded. Otherwise, remaining members receive `EVT GROUP LEAVE <username>`.

- **Success**: `OK`
- **Errors**: `ERR 401 NOT_IN_GROUP`

#### `GROUP DISBAND`

Disbands the group entirely. Only the group leader can disband. All members receive `EVT GROUP DISBAND` and are removed from the group. The group is deleted from the world.

- **Success**: `OK`
- **Errors**: `ERR 403 NOT_GROUP_LEADER`, `ERR 404 GROUP_NOT_FOUND`

#### `GROUP KICK <player>`

Removes a player from the group. Only the group leader can kick. The kicked player receives `EVT GROUP KICK <leader_name>`, and all remaining members receive `EVT GROUP LEAVE <kicked_player>`. The leader cannot kick themselves.

- **Success**: `OK`
- **Errors**: `ERR 403 NOT_GROUP_LEADER`, `ERR 404 PLAYER_NOT_IN_GROUP`, `ERR 407 CANNOT_KICK_SELF`

#### `GROUP INFO`

Returns the group's full details as JSON, including the group ID, leader name, list of members, and list of pending invites. The player must be in a group.

- **Success**: `OK {"id":"raiders","leader":"alice","players":["alice","bob"],"invited":["charlie"]}`
- **Errors**: `ERR 401 NOT_IN_GROUP`

#### `QUIT`

Disconnects from the server. The player is removed from the world, removed from their room's player list, and unregistered from the message registry. All players in the same room receive `EVT ROOM PRESENCE LEAVE <username>`. The server responds with `OK bye` before closing the connection.

- **Success**: `OK bye`

### Deviations from RFC 42TAP

- **SLEEP**: Extension command not in the RFC. Restores HP to max when used in the designated sleep room (`room.guest_room`). Returns `ERR 410 CANNOT_SLEEP_HERE` if used elsewhere.
- **EXAMINE**: Extension command not in the RFC. Inspects an item in the current room or the player's inventory. Returns item properties as JSON.
- **GROUP KICK, GROUP DISBAND, GROUP INFO**: Extension subcommands not in the RFC. KICK removes a member (leader only), DISBAND dissolves the group (leader only), INFO shows group composition.
- **LOOK response format**: Returns a flat JSON object with room fields at the top level (`id`, `name`, `description`, `exits`, `players`, `items`, `npcs`) plus an optional `can_sleep` boolean, rather than nesting under a `room` key.
- **TALK response**: Returns JSON (`{"npc": "<name>", "dialogue": "<dialogue>"}`) instead of plain text.
- **Chat broadcast format**: Chat messages are broadcast as `(SCOPE) username: message` (e.g., `(GLOBAL) alice: hello`), not prefixed with `EVT`.
- **EVT SLEEP**: Notifies other players in the room when someone rests, not defined in the RFC.
- **EVT GROUP LEADER, EVT GROUP DISBAND, EVT GROUP KICK**: Additional group events for leadership transfer, group dissolution, and member kicking.
- **ERR 409 PLAYER_DEAD, ERR 410 CANNOT_SLEEP_HERE**: Additional error codes not in the RFC.

### Events

| Event | Trigger |
|---|---|
| `EVT ROOM PRESENCE ENTER <player>` | A player enters the room (sent to all other players in the room) |
| `EVT ROOM PRESENCE LEAVE <player>` | A player leaves the room (sent to all other players in the room) |
| `EVT ROOM COMBAT <player> attacks <npc> for <dmg> damage npc=<id> hp=<remaining>` | A player attacks an NPC; includes NPC id and remaining HP for real-time updates |
| `EVT ROOM COMBAT <npc> defeated by <player> npc=<id>` | An NPC is killed; includes NPC id so clients can remove it |
| `EVT ROOM COMBAT <player> was killed by <npc>` | A player is killed by an NPC (sent to other players in the room) |
| `EVT ROOM ITEM TAKEN <item> <player>` | A player picks up an item (sent to other players in the room) |
| `EVT ROOM ITEM DROPPED <item> <player>` | A player drops an item (sent to other players in the room) |
| `EVT ROOM ITEM RESPAWN <item>` | An item reappears in the room 30 seconds after being taken |
| `EVT SLEEP <player>` | A player rests in the sleep room (sent to other players in the room) |
| `EVT GLOBAL [ALERT] ...` | Boss spawn announcement (sent to all connected players) |
| `EVT GROUP INVITE <inviter> id=<group_id>` | You received a group invitation |
| `EVT GROUP JOIN <player>` | A player joined your group (sent to all group members) |
| `EVT GROUP LEAVE <player>` | A player left or was kicked from your group |
| `EVT GROUP LEADER <player>` | A new leader was appointed after the old leader left |
| `EVT GROUP DISBAND` | Your group has been disbanded by the leader |
| `EVT GROUP KICK <leader>` | You were kicked from your group (sent only to the kicked player) |
| `(GLOBAL) user: msg` | Global chat message (sent to all connected players) |
| `(ROOM) user: msg` | Room chat message (sent to all players in the room) |
| `(GROUP) user: msg` | Group chat message (sent to all group members) |

### Error Codes

| Code | Constant | Meaning |
|---|---|---|
| 000 | NOT_AUTHENTICATED | Command sent before CONNECT |
| 201 | NAME_IN_USE | Username already taken by another connected player |
| 202 | INVALID_USER_NAME | Username is empty, too long (>20 chars), or contains invalid characters |
| 211 | GROUP_NAME_IN_USE | Group name already taken |
| 212 | INVALID_GROUP_NAME | Group name is empty, too long (>25 chars), or contains invalid characters |
| 301 | NO_EXIT | No exit in that direction from the current room |
| 401 | NOT_IN_GROUP | Player tried a group command but is not in any group |
| 402 | ALREADY_IN_GROUP | Player is already in a group (cannot join/create another) or already invited |
| 403 | NOT_GROUP_LEADER | Only the group leader can invite, kick, or disband |
| 404 | PLAYER_NOT_FOUND | Target player does not exist or is not connected |
| 404 | GROUP_NOT_FOUND | Group does not exist |
| 404 | ITEM_NOT_FOUND | Item not found in room |
| 404 | ITEM_NOT_IN_INVENTORY | Item not found in player's inventory |
| 404 | NPC_NOT_FOUND | NPC not found in current room |
| 404 | PLAYER_NOT_IN_GROUP | Target player is not a member of the group (kick) |
| 405 | NPC_NOT_HOSTILE | Cannot attack a friendly NPC |
| 406 | NO_QUEST_AVAILABLE | NPC has no quest, or the player already completed it |
| 407 | CANNOT_INVITE_SELF | Player tried to invite themselves |
| 407 | CANNOT_KICK_SELF | Leader tried to kick themselves |
| 407 | NOT_INVITED | Player tried to join a group without a pending invite |
| 408 | QUEST_NOT_COMPLETE | Player does not have enough quest items to turn in |
| 409 | PLAYER_DEAD | Player is dead and cannot perform this action |
| 410 | CANNOT_SLEEP_HERE | SLEEP was used outside the designated sleep room |
| 900 | CONNECTION_FAILED | TCP connection error |
| 901 | SEND_FAILED | Failed to serialize or send a response |
| 902 | FLOODING | Client exceeded the rate limit and was kicked |

### Flood Detection

The server tracks commands per second per client using a sliding window:
- The counter resets every second
- More than 10 commands/second: a warning is logged (`abuse_flood` event)
- More than 20 commands/second: the client is immediately kicked with `ERR 902 FLOODING` and the connection is closed. An `abuse_kick` event is logged with the player's IP and username.

## Combat System

Combat in TAP is a simple exchange-based system where each `ATTACK` command results in one round of mutual damage:

1. **Player attacks**: The player deals base damage of 10 plus the highest weapon damage bonus from their inventory. The server automatically scans the player's inventory and selects the weapon with the highest damage value. For example, if the player carries an Iron Sword (15 dmg) and a Pickaxe (8 dmg), the total damage is 10 + 15 = 25.
2. **NPC counter-attacks**: The NPC immediately deals its own damage to the player. Each NPC has a fixed damage value defined in the world data.
3. **HP update**: Both HP values are reduced simultaneously. HP cannot go below 0 (saturating subtraction).
4. **Outcome check**:
   - If the NPC's HP reaches 0: it is defeated, removed from the room, and will respawn in 30 seconds with full HP restored to its maximum.
   - If the player's HP reaches 0: the player dies, is moved to the spawn room (`room.gate`), and revived with 50 HP. Their status resets to Alive. All room events (combat result, presence leave/enter) are broadcast.
   - If both survive: the round ends, and the player can attack again.

### NPC Stats

| NPC | Location | HP | Damage | Difficulty |
|---|---|---|---|---|
| Giant Crab | Shipwreck Beach | 15 | 5 | Easy |
| Wild Boar | Sunlit Clearing | 20 | 8 | Easy |
| Forest Wolf | Whispering Forest | 25 | 10 | Medium |
| Cave Goblin | Crystal Cavern | 30 | 8 | Medium |
| Swamp Serpent | Murky Swamp | 30 | 10 | Medium |
| Cave Spider | Abandoned Mine | 35 | 12 | Medium |
| Forest Bandit | Deep Forest | 35 | 14 | Hard |
| Skeleton Warrior | Forgotten Crypt | 40 | 12 | Hard |
| Ancient Wraith | Ancient Ruins | 55 | 18 | Hard |

### Boss Stats

| Boss | Location | HP | Damage |
|---|---|---|---|
| Lich King | Dark Catacombs | 300 | 25 |
| The Kraken | Shipwreck Beach | 400 | 30 |
| Ancestral Dragon | Dragon's Lair | 500 | 35 |

### Weapon Damage Bonuses

| Weapon | Location | Damage bonus |
|---|---|---|
| Enchanted Staff | Ancient Ruins (or quest reward) | +25 |
| Battle Axe | Frozen Peak (or quest reward) | +20 |
| Sea Trident | Quest reward | +18 |
| Iron Sword | Blacksmith Forge | +15 |
| Shadow Dagger | Deep Forest | +12 |
| Pickaxe | Abandoned Mine | +8 |
| Bone Club | Forgotten Crypt | +6 |
| Wooden Branch | Whispering Forest | +3 |
| Fresh Fish | Harbor Docks | +1 |
| No weapon (base) | — | +0 |

### NPC Regeneration

A background task checks every 15 seconds whether any NPC needs HP restoration. An NPC's HP is restored to maximum only if **60 seconds have elapsed since the last hit** it received. This cooldown-based approach means bosses and other NPCs remain killable during active combat, but will fully heal if left alone for a minute.

### Boss System

The server supports **world bosses** defined in the world data (NPCs with `"boss": true`, `"boss_room"`, and `"boss_alert"` fields). Every 60 seconds, the boss spawner checks if any boss is active — if none is present, it spawns the next one in rotation. When a boss spawns, a global alert (`EVT GLOBAL [ALERT] ...`) is broadcast to all connected players. Bosses are designed as group challenges due to their high HP and damage.

| Boss | Room | HP | Dmg | Gold | Alert |
|---|---|---|---|---|---|
| Ancestral Dragon | Dragon's Lair | 500 | 35 | 100 | A thunderous roar echoes... |
| Lich King | Dark Catacombs | 300 | 25 | 75 | A chilling darkness spreads... |
| The Kraken | Shipwreck Beach | 400 | 30 | 80 | The sea churns violently... |

Boss configuration is data-driven: adding a new boss only requires adding an NPC entry with `boss`, `boss_room`, and `boss_alert` fields in the world JSON file. Unlike regular hostile NPCs, bosses do **not** respawn automatically after being killed — they only reappear via the boss spawner cycle when no boss is active.

## Economy System

Players start with **50 gold**. Gold is earned by defeating hostile NPCs (each NPC has a `gold_drop` value based on its HP). **All players who contributed damage** to an NPC receive the full gold reward when it dies — not just the player who lands the killing blow. Gold can be spent in two ways:

### Game Shop (`SHOP` / `SHOP BUY`)

A fixed catalog of items available at any time with infinite stock. Items are priced at their base `value`. The shop sells consumables (food, herbs), tools, and weapons.

### Player Market (`MARKET` / `SELL` / `MARKET BUY`)

A player-to-player marketplace. Any player can list an item from their inventory with `SELL` — the item is removed from their inventory and listed at its base value. Other players (and the seller) can purchase listed items with `MARKET BUY <index>`. When purchased, the buyer pays and the seller receives the gold. Listings are global and persist until bought (or until server restart).

## Quest System

Quests are fetch-type: the player must collect specific items and return them to the quest-giving NPC.

### Available Quests

| Quest | Giver | Location | Objective | Reward |
|---|---|---|---|---|
| quest.herbs | Old Hermit | Sacred Grove | Bring 1 Healing Herbs | 2 Health Potions |
| quest.sword | Village Blacksmith | Blacksmith Forge | Bring 1 Iron Sword | 1 Battle Axe |
| quest.pearl | Old Fisher | Harbor Docks | Bring 2 Sea Pearls | 1 Sea Trident |
| quest.mushroom | Swamp Witch | Murky Swamp | Bring 2 Glowing Mushrooms | 1 Enchanted Staff |

### Quest Flow

1. **Find the NPC**: Go to the room where the quest-giving NPC is located.
2. **Accept the quest**: Send `QUEST <npc>` (e.g., `QUEST hermit`). The server adds the quest to your active quest list and returns the full quest details as JSON, describing what items to collect and what you will receive.
3. **Collect items**: Explore the world and use `TAKE` to pick up the required items. Use `QUESTS` at any time to check your progress (e.g., `"progress": "0/1"` or `"progress": "1/1"`).
4. **Turn in the quest**: Return to the quest NPC and send `QUEST <npc>` again. If you have enough items, the quest items are consumed from your inventory, the reward items are added, and the quest is marked as completed. The server returns a confirmation with the reward details.
5. **Quest completion**: Once completed, a quest cannot be accepted again from the same NPC. The NPC returns `ERR 406 NO_QUEST_AVAILABLE` for that player.

### Quest Validation

- A player can only have each quest active once
- A completed quest cannot be re-accepted
- The required items must be in the player's inventory at turn-in time
- Quest items are consumed (removed from inventory) upon successful turn-in
- Reward items are added to inventory upon successful turn-in

## World Design

The default world contains **21 rooms** organized across five distinct areas. Difficulty increases as players move further from the village.

```
                        Frozen Peak
                            |
                      Crystal Cavern
                            |
              Abandoned Mine --- Blacksmith --- Village Square --- Marketplace --- Harbor Docks
                                                     |                                 |
                                   Inn --- Tavern    |                           Shipwreck Beach
                                                     |
                                                 South Gate --- Old Lighthouse
                                                     |
                                              Whispering Forest --- Sacred Grove
                                               |           |
                                          Deep Forest  Sunlit Clearing
                                               |
                                          Murky Swamp
                                               |
                                        Forgotten Crypt
                                               |
                                       Dark Catacombs
                                               |
                                         Ancient Ruins
                                               |
                                         Dragon's Lair
```

### Areas

**Village (safe zone)**: Village Square, Tavern, Traveler's Inn, Blacksmith Forge, Marketplace. No hostile NPCs. Shops, quests, and rest.

**Coastal**: Harbor Docks, Shipwreck Beach. Light combat (Giant Crab). Boss: The Kraken (400 HP).

**Mountains**: Abandoned Mine, Crystal Cavern, Frozen Peak. Medium combat (Cave Spider, Cave Goblin). Rare items at the peak.

**Forest**: Whispering Forest, Sacred Grove, Deep Forest, Sunlit Clearing, Murky Swamp. Increasing difficulty (Wolf → Boar → Bandit → Serpent).

**Dark Depths**: Forgotten Crypt, Dark Catacombs, Ancient Ruins, Dragon's Lair. Hardest area (Skeleton → Lich King → Wraith → Dragon). The best weapons are found here.

### Key Locations

- **South Gate** (`room.gate`): Spawn point for all new players. The boundary between village safety and the dangerous wilderness.
- **Traveler's Inn** (`room.inn`): The only room where SLEEP works. Located upstairs above the tavern. Players come here to restore HP to full.
- **Village Square** (`room.square`): Central hub connecting all village areas.
- **Dragon's Lair** (`room.lair`): Deepest room in the game. The Ancestral Dragon (500 HP) spawns here.
- **Dark Catacombs** (`room.catacombs`): The Lich King (300 HP) spawns here.
- **Shipwreck Beach** (`room.beach`): The Kraken (400 HP) spawns here.
- **Sacred Grove** (`room.grove`): The Old Hermit gives the herbs quest here.
- **Frozen Peak** (`room.peak`): End of the mountain branch. The Battle Axe (20 dmg) can be found here.

### NPCs

The world contains 21 NPCs with three roles:

- **Friendly NPCs** (9): Village Guard, Tavern Bartender, Innkeeper, Village Blacksmith, Market Merchant, Old Fisher, Lighthouse Keeper, Old Hermit, Swamp Witch. These NPCs can be talked to (`TALK`) and four of them give quests (`QUEST`). They cannot be attacked.
- **Hostile NPCs** (9): Giant Crab (15 HP), Wild Boar (20 HP), Forest Wolf (25 HP), Cave Goblin (30 HP), Swamp Serpent (30 HP), Cave Spider (35 HP), Forest Bandit (35 HP), Skeleton Warrior (40 HP), Ancient Wraith (55 HP). They counter-attack and respawn 30 seconds after being killed.
- **World Bosses** (3): Lich King (300 HP, 25 dmg), The Kraken (400 HP, 30 dmg), Ancestral Dragon (500 HP, 35 dmg). Spawn every 60 seconds in their designated rooms. Designed as group challenges.

### Items

The world contains 26 items distributed across rooms, with stronger equipment found in more dangerous areas:

- **Weapons** (9): Enchanted Staff (25 dmg, Ancient Ruins), Battle Axe (20 dmg, Frozen Peak), Sea Trident (18 dmg, quest reward), Iron Sword (15 dmg, Blacksmith), Shadow Dagger (12 dmg, Deep Forest), Pickaxe (8 dmg, Mine), Bone Club (6 dmg, Crypt), Wooden Branch (3 dmg, Forest), Fresh Fish (1 dmg, Docks)
- **Healing** (7): Health Potion (35 hp, Murky Swamp), Phoenix Feather (30 hp, Frozen Peak), Healing Herbs (20 hp, Sacred Grove), Glowing Mushroom (15 hp, Crystal Cavern), Frothy Ale (10 hp, Tavern), Loaf of Bread (8 hp, Market), Fresh Apple (5 hp, Market)
- **Quest Items**: Sea Pearl (Shipwreck Beach), Glowing Mushroom (Crystal Cavern), Healing Herbs (Sacred Grove), Iron Sword (Blacksmith)
- **Miscellaneous**: Old Key, Blue Crystal, Ancient Treasure, Hemp Rope, Oil Lantern, Broken Compass, Ancient Scroll, Dark Gem, Dragon Scale, Phoenix Feather

All items respawn in their original room 30 seconds after being picked up. Multiple players can each pick up the same item once it respawns.

## Server Logging

The server uses the `tracing` crate with JSON-formatted structured output. Each log entry is a JSON object containing a timestamp, severity level, event type, and contextual fields (player name, IP address, room, item, NPC, etc.).

### Log Levels and Events

- **INFO**: `server_start` (server bind address), `connection` (client IP), `disconnection` (client IP), `command` (full command text, player, IP), `response` (response text), `player_connect` (player name, spawn room), `player_disconnect` (player name, room), `item_take` / `item_drop` / `item_respawn` (item, player, room), `combat` (attack details), `combat_victory` / `combat_death` (outcome), `quest_accept` / `quest_complete` (quest, player), `boss_spawn` (room), `npc_regen` (NPC, HP), `npc_respawn` (NPC, room)
- **WARN**: `abuse_flood` (IP, player, rate — triggered at >10 cmd/s), `send_failed` (failed to deliver message to a client)
- **ERROR**: `bind_failed` (port in use), `read_failed` (TCP read error), `accept_failed` (failed to accept connection), `disconnect_failed` (cleanup error), `abuse_kick` (IP, player, rate — triggered at >20 cmd/s), `send_failed` (greeting failed)

### Usage

Control log level and filtering with the `RUST_LOG` environment variable:

```bash
RUST_LOG=info cargo run --bin server -- src/assets/default_world.json
RUST_LOG=warn cargo run --bin server -- src/assets/default_world.json
RUST_LOG=tap=debug cargo run --bin server -- src/assets/default_world.json
```

Log output goes to stderr in JSON format. Each line is a self-contained JSON object that can be piped to tools like `jq` for filtering:

```bash
cargo run --bin server -- src/assets/default_world.json 2>&1 | jq 'select(.fields.event == "combat")'
```

## Group Contributions

| Member | Contributions |
|---|---|
| **nbilyj** | Server architecture, TCP handling, command dispatcher, event system, combat system, quest system, CLI client, world design, protocol implementation |
| **nbarbosa** | Web GUI client (Axum + WebSocket bridge), HTML/CSS/JS frontend, group system, sleep system, flood detection |

## Building and Running

### Prerequisites

- Rust (edition 2021) and Cargo

### Build

```bash
make build
# or
cargo build --release
```

### Install Dependencies

```bash
make install
# or
cargo fetch
```

### Run the Server

```bash
make run-server
# or
cargo run --bin server -- src/assets/default_world.json
```

The server listens on `0.0.0.0:7534`. You can optionally pass a custom world JSON file as an argument. If no argument is given, it defaults to `src/assets/default_world.json`.

### Run the CLI Client

```bash
make run-client
# or
cargo run --bin client_cli -- 127.0.0.1 7534
```

The CLI client takes two arguments: the host and port to connect to. Once connected, type commands at the `>` prompt. Use Tab for autocompletion and Up/Down arrows for command history.

### Run the GUI Client

```bash
make run-client-gui
# or
cargo run --bin client_gui
```

Then open `http://127.0.0.1:3000` in a browser. The GUI port can be changed with the `TAP_GUI_PORT` environment variable:

```bash
TAP_GUI_PORT=8080 cargo run --bin client_gui
```

The GUI provides a visual interface with:
- Room description and exit buttons for navigation
- Player list and NPC list for the current room
- Chat panel for global, room, and group messages
- Inventory display
- Action buttons (attack, talk, take, sleep/rest)

### Quick Test with netcat

```bash
make nc
# or
nc 127.0.0.1 7534
```

Then type TAP commands manually. Example session:

```
OK hello proto=1
CONNECT alice
OK Connected
LOOK
OK {"id":"room.gate","name":"South Gate",...}
MOVE north
OK room=room.square
WHO
OK players=1
QUIT
OK bye
```

### Lint

```bash
make lint
# or
cargo check
```

### Clean

```bash
make clean
# or
cargo clean
```

## Testing

### Manual Testing

Connect multiple clients to verify multiplayer features:

```bash
# Terminal 1: start the server
make run-server

# Terminal 2: CLI client
cargo run --bin client_cli -- 127.0.0.1 7534

# Terminal 3: another CLI client or netcat
nc 127.0.0.1 7534
```

### Flood Detection Test

```bash
(echo "CONNECT flooder"; for i in $(seq 1 25); do echo "WHO"; done; echo "QUIT") | nc localhost 7534
```

Expected: after ~10 WHO commands, warnings appear in server logs. After ~20, the client receives `ERR 902 FLOODING` and is disconnected.

### Multiplayer Test Scenarios

- **Room presence**: Connect two players. Move one into the same room as the other. Verify the second player receives `EVT ROOM PRESENCE ENTER`.
- **Chat**: Two players in the same room: test `CHAT ROOM`. Two players in different rooms: test `CHAT GLOBAL`. Two players in a group: test `CHAT GROUP`.
- **Item persistence**: Player A takes an item. Player B does `LOOK` and confirms the item is gone. Wait 30 seconds, both players do `LOOK` and confirm the item respawned.
- **Combat**: Attack a hostile NPC until it dies. Verify the NPC disappears from `LOOK`. Wait 30 seconds, verify it reappears. Attack until the player dies, verify respawn at South Gate with 50 HP.
- **Quest flow**: `QUEST hermit` to accept, `TAKE herbs` in the grove, `QUEST hermit` to turn in. Verify herbs are consumed and potions are rewarded. `QUESTS` shows the quest as completed. Try all four quests (hermit, blacksmith, fisher, witch).
- **Group flow**: Player A: `GROUP CREATE team`. Player A: `GROUP INVITE playerB`. Player B receives the invite event. Player B: `GROUP JOIN team`. Both do `GROUP INFO` to confirm. Test `CHAT GROUP`. Player A: `GROUP KICK playerB` or `GROUP DISBAND`.
- **Boss**: Wait 60 seconds after server start. Verify all three bosses spawn (dragon in lair, lich in catacombs, kraken at beach). All players receive global alerts.
- **Sleep**: Go to the Traveler's Inn (`room.inn`). `LOOK` shows `can_sleep: true`. Take some damage from combat first, then `SLEEP`. Verify HP is restored to max.

### Feature Checklist

- [ ] Connect two players, verify `WHO` shows both
- [ ] Move between rooms, verify `EVT ROOM PRESENCE` events
- [ ] `CHAT GLOBAL`, `CHAT ROOM`, `CHAT GROUP` messaging
- [ ] `TAKE` / `DROP` items, verify room events and item respawn (30s)
- [ ] `ATTACK` hostile NPCs, verify combat, NPC death, and NPC respawn (30s)
- [ ] `ATTACK` until player dies, verify respawn at South Gate with 50 HP
- [ ] `QUEST` accept and turn-in flow with item consumption and reward
- [ ] `QUESTS` shows active progress and completed quests
- [ ] `GROUP CREATE` / `INVITE` / `JOIN` / `LEAVE` / `DISBAND` / `KICK` / `INFO`
- [ ] `SLEEP` in the Traveler's Inn, verify HP restoration to max
- [ ] `SLEEP` outside the inn returns `ERR 410`
- [ ] `EXAMINE` items in room and inventory
- [ ] `INVENTORY` shows current items as JSON array
- [ ] `STATUS` shows HP and alive/dead status
- [ ] `TALK` to friendly and hostile NPCs
- [ ] Flood detection: rapid commands trigger warning then kick
- [ ] Boss spawn: wait 60s, verify 3 bosses appear (dragon/lich/kraken) with global alerts
- [ ] NPC HP regen: damage an NPC without killing it, wait 60s after last hit, verify HP is back to max
- [ ] Disconnect cleanup: player leaves, verify room event and `WHO` count decreases
- [ ] Username validation: empty, too long, special characters all rejected
