"use strict";

/* ==========================================================================
 * TAP GUI client
 *
 * Wire protocol reminder (see RFC 42TAP + this server's actual behaviour):
 *   - one command per line, server replies with exactly one "OK ..." or
 *     "ERR <code> <NAME>" line, in the SAME ORDER commands were sent
 *     (this connection's read loop awaits each handler before reading the
 *     next line, so responses never get reordered relative to requests).
 *   - "EVT ..." lines and raw "(GLOBAL)/(ROOM)/(GROUP) user: text" chat
 *     lines are UNSOLICITED and can arrive at any time, interleaved with
 *     command responses. They are never a reply to something we sent.
 *
 * That distinction is the whole trick to building a client without
 * request IDs: filter out anything that looks like an event/chat push
 * first, and treat everything else as "the answer to the oldest command
 * still waiting for one".
 * ========================================================================== */

// ---------------------------------------------------------------------------
// DOM references
// ---------------------------------------------------------------------------
const $ = (sel, root = document) => root.querySelector(sel);
const $$ = (sel, root = document) => Array.from(root.querySelectorAll(sel));

const screenLogin = $("#screen-login");
const screenGame = $("#screen-game");
const connectForm = $("#connect-form");
const connectSubmit = $("#connect-submit");
const loginError = $("#login-error");

const connDot = $("#conn-dot");
const connLabel = $("#conn-label");
const meNameEl = $("#me-name");
const roomCountEl = $("#room-count");
const serverCountEl = $("#server-count");

const hpFillTop = $("#hp-fill-top");
const hpTextTop = $("#hp-text-top");
const hpFillMain = $("#hp-fill-main");
const hpTextMain = $("#hp-text-main");
const statusPill = $("#status-pill");

const roomNameEl = $("#room-name");
const roomIdEl = $("#room-id");
const roomDescEl = $("#room-desc");
const exitRowEl = $("#exit-row");
const playersListEl = $("#players-list");
const itemsListEl = $("#items-list");
const npcsListEl = $("#npcs-list");

const inventoryListEl = $("#inventory-list");
const questListEl = $("#quest-list");

const groupNoneEl = $("#group-none");
const groupActiveEl = $("#group-active");
const groupIdLabel = $("#group-id-label");
const groupLeaderLabel = $("#group-leader-label");
const groupMembersList = $("#group-members-list");
const groupDisbandBtn = $("#group-disband-btn");

const toastStack = $("#toast-stack");
const npcPopover = $("#npc-popover");
const itemPopover = $("#item-popover");

// ---------------------------------------------------------------------------
// Client-side state (everything here mirrors what the server told us —
// nothing is invented except cosmetic humanized labels for raw IDs).
// ---------------------------------------------------------------------------
const state = {
  ws: null,
  connected: false,
  me: { username: null, hp: null, maxHp: null, status: "alive" },
  room: null,           // last LOOK payload
  inventory: [],         // array of item ids
  quests: [],            // array of {quest_id, status, progress?}
  group: null,           // {id, leader, players, invited} or null
  serverCount: 0,
  npcCache: {},          // id -> {name, hostile: true|false|null, hp: number|null}
  itemCache: {},         // id -> {name, damage, armor, heal} — filled in by EXAMINE
  activeTab: "global",
  activeNpc: null,        // id currently shown in the popover
};

// FIFO queue of {type, meta} describing which command we're waiting on.
const pending = [];

// ---------------------------------------------------------------------------
// Small utilities
// ---------------------------------------------------------------------------
function humanize(id) {
  if (!id) return "";
  const short = id.includes(".") ? id.slice(id.indexOf(".") + 1) : id;
  return short
    .split(/[_\s]+/)
    .map((w) => (w.length ? w[0].toUpperCase() + w.slice(1) : w))
    .join(" ");
}

function escapeHtml(str) {
  return String(str)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

function nowTime() {
  const d = new Date();
  return d.toTimeString().slice(0, 8);
}

function safeJson(str) {
  try { return JSON.parse(str); } catch { return undefined; }
}

const ERROR_MESSAGES = {
  "000": "Not authenticated yet.",
  "201": "That name is already taken.",
  "202": "That username isn't valid.",
  "211": "That group name is already in use.",
  "212": "That group name isn't valid.",
  "301": "You can't go that way.",
  "401": "You're not in a group.",
  "402": "Already in a group.",
  "403": "Only the group leader can do that.",
  "404": "Not found.",
  "405": "That can't be attacked.",
  "406": "No quest available there.",
  "407": "That's not possible.",
  "408": "Quest objective isn't complete yet.",
  "409": "You're down — can't do that right now.",
  "410": "You can't rest here.",
  "900": "Connection failed.",
  "901": "Message failed to send.",
  "902": "Slow down — you're sending commands too fast.",
};

function friendlyError(line) {
  const m = line.match(/^ERR\s+(\d{3})\s*(.*)$/);
  if (!m) return line;
  const [, code, name] = m;
  return ERROR_MESSAGES[code] || name || `Error ${code}`;
}

// ---------------------------------------------------------------------------
// Logging into the three terminal panes ("global" / "room" / "group") plus
// the catch-all protocol "log" pane.
// ---------------------------------------------------------------------------
const panes = {
  global: $("#pane-global"),
  room: $("#pane-room"),
  group: $("#pane-group"),
  log: $("#pane-log"),
};

function appendLine(pane, kind, html) {
  const el = panes[pane];
  if (!el) return;
  const wasAtBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 24;
  const line = document.createElement("div");
  line.className = `log-line ${kind}`;
  line.innerHTML = `<span class="log-time">${nowTime()}</span><span class="log-body">${html}</span>`;
  el.appendChild(line);
  if (wasAtBottom) el.scrollTop = el.scrollHeight;
}

function logChat(scope, who, text) {
  const isSelf = who === state.me.username;
  appendLine(scope, `chat${isSelf ? " self" : ""}`,
    `<span class="who">${escapeHtml(who)}</span>: ${escapeHtml(text)}`);
}

function logEvent(text) { appendLine("log", "event", escapeHtml(text)); }
function logCombat(text) { appendLine("log", "combat", escapeHtml(text)); }
function logSystem(text) { appendLine("log", "system", escapeHtml(text)); }
function logRaw(text) { appendLine("log", "system", escapeHtml(text)); }
function logErrorLine(text) { appendLine("log", "error", escapeHtml(friendlyError(text))); }

// ---------------------------------------------------------------------------
// Toasts
// ---------------------------------------------------------------------------
function showToast({ text, type = "info", actions = [], timeout = 4500 }) {
  const el = document.createElement("div");
  el.className = `toast${type === "error" ? " error" : ""}`;
  const body = document.createElement("div");
  body.textContent = text;
  el.appendChild(body);
  if (actions.length) {
    const row = document.createElement("div");
    row.className = "toast-actions";
    actions.forEach((a) => {
      const b = document.createElement("button");
      b.className = "btn btn-secondary btn-xs";
      b.textContent = a.label;
      b.onclick = () => { a.onClick(); el.remove(); };
      row.appendChild(b);
    });
    el.appendChild(row);
  }
  toastStack.appendChild(el);
  if (timeout) setTimeout(() => el.remove(), timeout);
}

// ---------------------------------------------------------------------------
// WebSocket / command queue
// ---------------------------------------------------------------------------
function sendCommand(type, raw, meta = {}) {
  if (!state.ws || state.ws.readyState !== WebSocket.OPEN) return;
  pending.push({ type, meta });
  state.ws.send(raw);
}

function connect(host, port, username) {
  const proto = location.protocol === "https:" ? "wss" : "ws";
  const ws = new WebSocket(`${proto}://${location.host}/ws`);
  state.ws = ws;

  ws.addEventListener("open", () => {
    pending.push({ type: "GREETING" });
    pending.push({ type: "CONNECT", meta: { username } });
    ws.send(JSON.stringify({ host, port: Number(port), username }));
  });

  ws.addEventListener("message", (event) => onLine(event.data));

  ws.addEventListener("close", () => {
    state.connected = false;
    connDot.classList.add("down");
    connLabel.textContent = "disconnected";
    if (screenGame.hidden === false) {
      showToast({ text: "Connection to the server was lost.", type: "error", timeout: 6000 });
      returnToLogin();
    } else {
      loginFailed("Could not reach the server.");
    }
  });

  ws.addEventListener("error", () => { /* close handler covers UX */ });
}

function returnToLogin() {
  state.ws = null;
  state.me = { username: null, hp: null, maxHp: null, status: "alive" };
  state.room = null;
  state.inventory = [];
  state.quests = [];
  state.group = null;
  state.npcCache = {};
  pending.length = 0;
  screenGame.hidden = true;
  screenLogin.hidden = false;
  connectSubmit.disabled = false;
  connectSubmit.textContent = "Enter the world";
}

function loginFailed(msg) {
  loginError.hidden = false;
  loginError.textContent = msg;
  connectSubmit.disabled = false;
  connectSubmit.textContent = "Enter the world";
  if (state.ws) { try { state.ws.close(); } catch { /* noop */ } }
}

// ---------------------------------------------------------------------------
// Incoming line dispatcher
// ---------------------------------------------------------------------------
function onLine(line) {
  if (line == null) return;
  line = String(line);
  if (line.length === 0) return;

  // --- unsolicited chat broadcasts ------------------------------------
  const chatMatch = line.match(/^\((GLOBAL|ROOM|GROUP)\)\s+(\S+):\s?(.*)$/);
  if (chatMatch) {
    const [, scope, who, text] = chatMatch;
    logChat(scope.toLowerCase(), who, text);
    return;
  }

  // --- unsolicited protocol events ------------------------------------
  if (line.startsWith("EVT ")) {
    handleEvent(line.slice(4));
    return;
  }

  // --- bridge-level failures (before any TAP handshake) ---------------
  if (line.startsWith("ERR connection failed")) {
    if (pending.length && pending[0].type === "GREETING") pending.shift();
    if (pending.length && pending[0].type === "CONNECT") pending.shift();
    loginFailed(line.replace(/^ERR\s+/, ""));
    return;
  }

  // --- everything else is the answer to the oldest pending command ----
  const ctx = pending.shift();
  if (!ctx) { logRaw(line); return; }
  handleResponse(ctx, line);
}

// ---------------------------------------------------------------------------
// Responses (OK / ERR) matched against the command that triggered them
// ---------------------------------------------------------------------------
function handleResponse(ctx, line) {
  const isErr = line.startsWith("ERR");

  switch (ctx.type) {
    case "GREETING":
      // "OK hello proto=1" — nothing to do, CONNECT is next in the queue.
      return;

    case "CONNECT":
      if (isErr) { loginFailed(friendlyError(line)); return; }
      state.me.username = ctx.meta.username;
      meNameEl.textContent = ctx.meta.username;
      enterGame();
      return;

    case "QUIT":
      logSystem("You left the world.");
      showToast({ text: "You quit. Come back any time.", timeout: 3000 });
      if (state.ws) state.ws.close();
      return;

    case "LOOK":
      if (isErr) { logErrorLine(line); return; }
      applyRoom(safeJson(line.slice(3)));
      return;

    case "MOVE":
      if (isErr) { showToast({ text: friendlyError(line), type: "error" }); logErrorLine(line); return; }
      sendCommand("LOOK", "LOOK");
      return;

    case "WHO": {
      if (isErr) return;
      const m = line.match(/players=(\d+)/);
      state.serverCount = m ? Number(m[1]) : state.serverCount;
      serverCountEl.textContent = state.serverCount;
      return;
    }

    case "CHAT":
      if (isErr) { showToast({ text: friendlyError(line), type: "error" }); }
      // success is silent: the message already arrived via the broadcast line
      return;

    case "TALK": {
      if (isErr) { showToast({ text: friendlyError(line), type: "error" }); closeNpcPopover(); return; }
      const data = safeJson(line.slice(3));
      if (!data) return;
      if (ctx.meta.npcId) {
        state.npcCache[ctx.meta.npcId] = { ...(state.npcCache[ctx.meta.npcId] || {}), name: data.npc };
      }
      appendLine("log", "event", `<span class="who">${escapeHtml(data.npc)}</span>: “${escapeHtml(data.dialogue)}”`);
      showNpcDialogue(data.dialogue);
      return;
    }

    case "EXAMINE": {
      if (isErr) { showToast({ text: friendlyError(line), type: "error" }); closeItemPopover(); return; }
      const data = safeJson(line.slice(3));
      if (!data) return;
      state.itemCache[data.id] = data;
      showItemPopover(data);
      return;
    }

    case "TAKE":
      if (isErr) { showToast({ text: friendlyError(line), type: "error" }); return; }
      sendCommand("LOOK", "LOOK");
      sendCommand("INVENTORY", "INVENTORY");
      return;

    case "DROP":
      if (isErr) { showToast({ text: friendlyError(line), type: "error" }); return; }
      sendCommand("LOOK", "LOOK");
      sendCommand("INVENTORY", "INVENTORY");
      return;

    case "INVENTORY": {
      if (isErr) return;
      const arr = safeJson(line.slice(3));
      state.inventory = Array.isArray(arr) ? arr : [];
      renderInventory();
      return;
    }

    case "ATTACK": {
      if (isErr) {
        if (ctx.meta.npcId) {
          state.npcCache[ctx.meta.npcId] = { ...(state.npcCache[ctx.meta.npcId] || {}), hostile: false };
        }
        showToast({ text: friendlyError(line), type: "error" });
        renderRoom();
        return;
      }
      const data = safeJson(line.slice(3));
      if (!data) return;
      if (ctx.meta.npcId) {
        state.npcCache[ctx.meta.npcId] = {
          ...(state.npcCache[ctx.meta.npcId] || {}),
          hostile: true,
          hp: data.status === "victory" ? 0 : data.target_hp,
        };
      }
      const npcLabel = state.npcCache[ctx.meta.npcId]?.name || humanize(ctx.meta.npcId);
      if (data.status === "victory") logCombat(`You defeated ${npcLabel}! (-${data.damage} HP dealt)`);
      else if (data.status === "death") logCombat(`${npcLabel} struck you down. You wake up back at a safe place.`);
      else logCombat(`You hit ${npcLabel} for ${data.damage}. Their HP: ${data.target_hp}. Yours: ${data.attacker_hp}.`);
      sendCommand("STATUS", "STATUS");
      if (data.status === "victory" || data.status === "death") sendCommand("LOOK", "LOOK");
      else renderRoom();
      return;
    }

    case "STATUS": {
      if (isErr) return;
      const data = safeJson(line.slice(3));
      if (!data) return;
      state.me.hp = data.hp;
      state.me.maxHp = data.max_hp;
      state.me.status = data.status;
      renderHp();
      return;
    }

    case "QUEST": {
      if (isErr) { showToast({ text: friendlyError(line), type: "error" }); return; }
      const data = safeJson(line.slice(3));
      if (!data) return;
      if (data.status === "completed") {
        showToast({ text: `Quest complete! +${data.reward_count} × ${humanize(data.reward)}` });
        sendCommand("INVENTORY", "INVENTORY");
      } else {
        showToast({ text: `New quest: ${data.description}` });
      }
      sendCommand("QUESTS", "QUESTS");
      return;
    }

    case "QUESTS": {
      if (isErr) return;
      const arr = safeJson(line.slice(3));
      state.quests = Array.isArray(arr) ? arr : [];
      renderQuests();
      return;
    }

    case "SLEEP": {
      if (isErr) { showToast({ text: friendlyError(line), type: "error" }); return; }
      const m = line.match(/hp=(\d+)\/(\d+)/);
      if (m) { state.me.hp = Number(m[1]); state.me.maxHp = Number(m[2]); renderHp(); }
      showToast({ text: "You feel rested." });
      return;
    }

    case "GROUP_CREATE":
    case "GROUP_JOIN": {
      if (isErr) { showToast({ text: friendlyError(line), type: "error" }); return; }
      const m = line.match(/group=(\S+)/);
      if (m) { sendCommand("GROUP_INFO", "GROUP INFO"); }
      return;
    }

    case "GROUP_INFO": {
      if (isErr) { state.group = null; renderGroup(); return; }
      const data = safeJson(line.slice(3));
      state.group = data || null;
      renderGroup();
      return;
    }

    case "GROUP_INVITE":
      if (isErr) { showToast({ text: friendlyError(line), type: "error" }); return; }
      showToast({ text: "Invite sent." });
      sendCommand("GROUP_INFO", "GROUP INFO");
      return;

    case "GROUP_LEAVE":
      if (isErr) { showToast({ text: friendlyError(line), type: "error" }); return; }
      state.group = null;
      renderGroup();
      return;

    case "GROUP_DISBAND":
      if (isErr) { showToast({ text: friendlyError(line), type: "error" }); return; }
      state.group = null;
      renderGroup();
      return;

    case "GROUP_KICK":
      if (isErr) { showToast({ text: friendlyError(line), type: "error" }); return; }
      sendCommand("GROUP_INFO", "GROUP INFO");
      return;

    case "RAW":
      logRaw(`tap:~$ ${ctx.meta.raw}`);
      logRaw(line);
      refreshAll();
      return;

    default:
      logRaw(line);
  }
}

// ---------------------------------------------------------------------------
// Unsolicited events (EVT ...)
// ---------------------------------------------------------------------------
function handleEvent(rest) {
  if (rest.startsWith("ROOM PRESENCE ENTER ")) {
    const who = rest.slice("ROOM PRESENCE ENTER ".length).trim();
    if (state.room && !state.room.players.includes(who)) state.room.players.push(who);
    logEvent(`${who} enters the room.`);
    renderRoom();
    return;
  }
  if (rest.startsWith("ROOM PRESENCE LEAVE ")) {
    const who = rest.slice("ROOM PRESENCE LEAVE ".length).trim();
    if (state.room) state.room.players = state.room.players.filter((p) => p !== who);
    logEvent(`${who} leaves the room.`);
    renderRoom();
    return;
  }
  if (rest.startsWith("ROOM COMBAT ")) {
    logCombat(rest.slice("ROOM COMBAT ".length));
    return;
  }
  if (rest.startsWith("SLEEP ")) {
    logEvent(`${rest.slice(6).trim()} settles in to rest.`);
    return;
  }
  if (rest.startsWith("GROUP INVITE ")) {
    const m = rest.match(/^GROUP INVITE (\S+) id=(\S+)$/);
    if (m) {
      const [, from, groupId] = m;
      logEvent(`${from} invited you to group "${groupId}".`);
      showToast({
        text: `${from} invited you to their group.`,
        actions: [
          { label: "Accept", onClick: () => sendCommand("GROUP_JOIN", `GROUP JOIN ${groupId}`) },
          { label: "Dismiss", onClick: () => {} },
        ],
        timeout: 0,
      });
    }
    return;
  }
  if (rest.startsWith("GROUP KICK ")) {
    const kicker = rest.slice("GROUP KICK ".length).trim();
    logEvent(`You were removed from the group by ${kicker}.`);
    showToast({ text: `${kicker} removed you from the group.`, type: "error" });
    state.group = null;
    renderGroup();
    return;
  }
  if (rest.startsWith("GROUP DISBAND")) {
    logEvent("The group was disbanded.");
    state.group = null;
    renderGroup();
    return;
  }
  if (rest.startsWith("GROUP LEADER ")) {
    logEvent(`${rest.slice("GROUP LEADER ".length).trim()} is now the group leader.`);
    if (state.group) sendCommand("GROUP_INFO", "GROUP INFO");
    return;
  }
  if (rest.startsWith("GROUP JOIN ")) {
    logEvent(`${rest.slice("GROUP JOIN ".length).trim()} joined the group.`);
    if (state.group) sendCommand("GROUP_INFO", "GROUP INFO");
    return;
  }
  if (rest.startsWith("GROUP LEAVE ")) {
    logEvent(`${rest.slice("GROUP LEAVE ".length).trim()} left the group.`);
    if (state.group) sendCommand("GROUP_INFO", "GROUP INFO");
    return;
  }
  if (rest.startsWith("DISCONNECTED")) {
    logSystem(rest);
    showToast({ text: "The server closed the connection.", type: "error", timeout: 6000 });
    setTimeout(returnToLogin, 800);
    return;
  }
  // Unknown event shape — still surface it, never drop silently.
  logRaw(`EVT ${rest}`);
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------
function renderHp() {
  const { hp, maxHp, status } = state.me;
  const pct = maxHp ? Math.max(0, Math.min(100, (hp / maxHp) * 100)) : 0;
  [hpFillTop, hpFillMain].forEach((el) => {
    el.style.width = `${pct}%`;
    el.classList.toggle("low", pct <= 30);
  });
  const text = hp != null ? `${hp}/${maxHp}` : "—/—";
  hpTextTop.textContent = text;
  hpTextMain.textContent = text;
  statusPill.textContent = status || "alive";
  statusPill.classList.toggle("dead", status === "dead");
}

function applyRoom(room) {
  if (!room) return;
  state.room = room;
  renderRoom();
}

const DIRECTION_ORDER = ["north", "east", "south", "west", "up", "down", "in", "out"];

function renderRoom() {
  const room = state.room;
  if (!room) return;

  roomNameEl.textContent = room.name || "—";
  roomIdEl.textContent = room.id || "";
  roomDescEl.textContent = room.description || "";

  // exits
  exitRowEl.innerHTML = "";
  const dirs = Object.keys(room.exits || {});
  if (!dirs.length) {
    exitRowEl.innerHTML = '<span class="empty-note">no visible exits</span>';
  } else {
    dirs
      .sort((a, b) => {
        const ia = DIRECTION_ORDER.indexOf(a), ib = DIRECTION_ORDER.indexOf(b);
        if (ia === -1 && ib === -1) return a.localeCompare(b);
        if (ia === -1) return 1;
        if (ib === -1) return -1;
        return ia - ib;
      })
      .forEach((dir) => {
        const btn = document.createElement("button");
        btn.className = "exit-btn";
        btn.type = "button";
        btn.textContent = humanize(dir);
        btn.title = `Move ${dir}`;
        btn.onclick = () => sendCommand("MOVE", `MOVE ${dir}`);
        exitRowEl.appendChild(btn);
      });
  }

  // players (excluding self for the list, but counting self for the tally)
  const others = (room.players || []).filter((p) => p !== state.me.username);
  playersListEl.innerHTML = others.length
    ? others.map((p) => `<li class="chip">${escapeHtml(p)}</li>`).join("")
    : '<li class="empty-note">you\'re alone</li>';
  roomCountEl.textContent = (room.players || []).length;

  // items
  const items = room.items || [];
  itemsListEl.innerHTML = items.length
    ? items.map((id) => `
        <li class="item-row">
          <button class="entity-btn" type="button" data-action="take" data-id="${escapeHtml(id)}" title="Take">
            ${escapeHtml(state.itemCache[id]?.name || humanize(id))} <span class="entity-mark">take</span>
          </button>
          <button class="btn btn-ghost btn-xs item-info-btn" type="button" data-action="item-info" data-id="${escapeHtml(id)}" title="Item info" aria-label="Item info">ⓘ</button>
        </li>`).join("")
    : '<li class="empty-note">nothing lying around</li>';

  // npcs
  const npcs = room.npcs || [];
  npcsListEl.innerHTML = npcs.length
    ? npcs.map((id) => {
        const cached = state.npcCache[id];
        const label = cached?.name || humanize(id);
        const hp = cached?.hp != null ? ` <span class="entity-mark">${cached.hp} hp</span>` : "";
        const hostileClass = cached?.hostile ? " hostile" : "";
        return `<li>
          <button class="entity-btn${hostileClass}" type="button" data-action="npc" data-id="${escapeHtml(id)}">
            ${escapeHtml(label)}${hp}
          </button>
        </li>`;
      }).join("")
    : '<li class="empty-note">no one else here</li>';

  $$('[data-action="take"]', itemsListEl).forEach((btn) => {
    btn.onclick = () => sendCommand("TAKE", `TAKE ${btn.dataset.id}`);
  });
  $$('[data-action="item-info"]', itemsListEl).forEach((btn) => {
    btn.onclick = (ev) => { ev.stopPropagation(); requestItemInfo(btn.dataset.id, ev.currentTarget); };
  });
  $$('[data-action="npc"]', npcsListEl).forEach((btn) => {
    btn.onclick = (ev) => openNpcPopover(btn.dataset.id, ev.currentTarget);
  });
}

function renderInventory() {
  inventoryListEl.innerHTML = state.inventory.length
    ? state.inventory.map((id) => `
        <li class="inventory-item">
          <span>${escapeHtml(state.itemCache[id]?.name || humanize(id))}</span>
          <span class="inventory-item-actions">
            <button class="btn btn-ghost btn-xs" data-info="${escapeHtml(id)}" type="button" title="Item info" aria-label="Item info">ⓘ</button>
            <button class="btn btn-ghost btn-xs" data-drop="${escapeHtml(id)}" type="button">drop</button>
          </span>
        </li>`).join("")
    : '<li class="empty-note">empty-handed</li>';
  $$('[data-drop]', inventoryListEl).forEach((btn) => {
    btn.onclick = () => sendCommand("DROP", `DROP ${btn.dataset.drop}`);
  });
  $$('[data-info]', inventoryListEl).forEach((btn) => {
    btn.onclick = (ev) => requestItemInfo(btn.dataset.info, ev.currentTarget);
  });
}

function renderQuests() {
  if (!state.quests.length) {
    questListEl.innerHTML = '<li class="empty-note">no quests yet — talk to someone</li>';
    return;
  }
  questListEl.innerHTML = state.quests.map((q) => {
    const done = q.status === "completed";
    return `<li class="quest-item${done ? " completed" : ""}">
      <span class="quest-name">${escapeHtml(humanize(q.quest_id))}</span>
      <span class="quest-meta">${done ? "completed" : `in progress — ${escapeHtml(q.progress || "")}`}</span>
    </li>`;
  }).join("");
}

function renderGroup() {
  if (!state.group) {
    groupNoneEl.hidden = false;
    groupActiveEl.hidden = true;
    return;
  }
  groupNoneEl.hidden = true;
  groupActiveEl.hidden = false;
  groupIdLabel.textContent = state.group.id;
  groupLeaderLabel.textContent = state.group.leader;
  const isLeader = state.group.leader === state.me.username;
  groupDisbandBtn.hidden = !isLeader;

  const members = state.group.players || [];
  const invited = state.group.invited || [];
  groupMembersList.innerHTML = members.map((p) => {
    const isMe = p === state.me.username;
    const leaderTag = p === state.group.leader ? ' <span class="tag">leader</span>' : "";
    const kickBtn = isLeader && !isMe
      ? `<button class="btn btn-ghost btn-xs" data-kick="${escapeHtml(p)}" type="button">kick</button>`
      : "";
    return `<li class="chip" style="justify-content:space-between; display:flex;">
      <span>${escapeHtml(p)}${leaderTag}</span>${kickBtn}
    </li>`;
  }).join("") + invited.map((p) =>
    `<li class="chip"><em>${escapeHtml(p)} (invited)</em></li>`
  ).join("");

  $$('[data-kick]', groupMembersList).forEach((btn) => {
    btn.onclick = () => sendCommand("GROUP_KICK", `GROUP KICK ${btn.dataset.kick}`);
  });
}

// ---------------------------------------------------------------------------
// NPC popover
// ---------------------------------------------------------------------------
function openNpcPopover(npcId, anchorEl) {
  state.activeNpc = npcId;
  const cached = state.npcCache[npcId] || {};
  $("#npc-popover-name").textContent = cached.name || humanize(npcId);
  $("#npc-popover-dialogue").hidden = true;

  const attackBtn = $("#npc-attack-btn");
  attackBtn.hidden = cached.hostile === false;
  attackBtn.disabled = false;

  npcPopover.hidden = false;
  const rect = anchorEl.getBoundingClientRect();
  const top = Math.min(window.innerHeight - 200, rect.bottom + 8 + window.scrollY);
  const left = Math.min(window.innerWidth - 280, rect.left + window.scrollX);
  npcPopover.style.top = `${Math.max(8, top)}px`;
  npcPopover.style.left = `${Math.max(8, left)}px`;
}

function closeNpcPopover() {
  npcPopover.hidden = true;
  state.activeNpc = null;
}

function showNpcDialogue(text) {
  const el = $("#npc-popover-dialogue");
  el.hidden = false;
  el.textContent = `“${text}”`;
}

// ---------------------------------------------------------------------------
// Item popover — "EXAMINE" is a TAP protocol extension (not in RFC 42TAP)
// added specifically so the GUI can show item stats; see README.
// ---------------------------------------------------------------------------
let pendingItemAnchor = null;

function requestItemInfo(itemId, anchorEl) {
  pendingItemAnchor = anchorEl;
  sendCommand("EXAMINE", `EXAMINE ${itemId}`, { itemId });
}

function showItemPopover(item) {
  $("#item-popover-name").textContent = item.name || humanize(item.id);
  $("#item-popover-id").textContent = item.id;

  const stats = [];
  if (item.damage != null) stats.push(["Damage", item.damage]);
  if (item.armor != null) stats.push(["Armor", item.armor]);
  if (item.heal != null) stats.push(["Heals", item.heal]);
  const statsEl = $("#item-popover-stats");
  statsEl.innerHTML = stats.length
    ? stats.map(([label, val]) => `<li><span>${escapeHtml(label)}</span><span>${escapeHtml(val)}</span></li>`).join("")
    : '<li class="empty-note">nothing special about it</li>';

  itemPopover.hidden = false;
  const anchor = pendingItemAnchor;
  if (anchor) {
    const rect = anchor.getBoundingClientRect();
    const top = Math.min(window.innerHeight - 200, rect.bottom + 8 + window.scrollY);
    const left = Math.min(window.innerWidth - 280, rect.left + window.scrollX);
    itemPopover.style.top = `${Math.max(8, top)}px`;
    itemPopover.style.left = `${Math.max(8, left)}px`;
  }
}

function closeItemPopover() {
  itemPopover.hidden = true;
  pendingItemAnchor = null;
}

// ---------------------------------------------------------------------------
// Helpers to (re)sync everything after connecting or after a raw command
// ---------------------------------------------------------------------------
function refreshAll() {
  sendCommand("LOOK", "LOOK");
  sendCommand("STATUS", "STATUS");
  sendCommand("INVENTORY", "INVENTORY");
  sendCommand("QUESTS", "QUESTS");
  sendCommand("WHO", "WHO");
  if (state.group) sendCommand("GROUP_INFO", "GROUP INFO");
}

function enterGame() {
  state.connected = true;
  connDot.classList.remove("down");
  connLabel.textContent = "connected";
  screenLogin.hidden = true;
  screenGame.hidden = false;
  connectSubmit.disabled = false;
  connectSubmit.textContent = "Enter the world";
  loginError.hidden = true;
  logSystem(`Connected as ${state.me.username}.`);
  refreshAll();
}

// ---------------------------------------------------------------------------
// UI wiring
// ---------------------------------------------------------------------------
connectForm.addEventListener("submit", (e) => {
  e.preventDefault();
  const data = new FormData(connectForm);
  const host = data.get("host").trim();
  const port = data.get("port");
  const username = data.get("username").trim();
  loginError.hidden = true;
  if (!host || !port || !username) {
    loginFailed("Fill in every field.");
    return;
  }
  connectSubmit.disabled = true;
  connectSubmit.textContent = "Connecting…";
  connect(host, port, username);
});

$("#quit-btn").addEventListener("click", () => sendCommand("QUIT", "QUIT"));
$("#look-refresh").addEventListener("click", () => sendCommand("LOOK", "LOOK"));
$("#inventory-refresh").addEventListener("click", () => sendCommand("INVENTORY", "INVENTORY"));
$("#quests-refresh").addEventListener("click", () => sendCommand("QUESTS", "QUESTS"));
$("#group-refresh").addEventListener("click", () => {
  if (state.group) sendCommand("GROUP_INFO", "GROUP INFO");
});

$("#group-create-form").addEventListener("submit", (e) => {
  e.preventDefault();
  const name = new FormData(e.target).get("name").trim();
  sendCommand("GROUP_CREATE", name ? `GROUP CREATE ${name}` : "GROUP CREATE");
  e.target.reset();
});
$("#group-join-form").addEventListener("submit", (e) => {
  e.preventDefault();
  const id = new FormData(e.target).get("id").trim();
  if (!id) return;
  sendCommand("GROUP_JOIN", `GROUP JOIN ${id}`);
  e.target.reset();
});
$("#group-invite-form").addEventListener("submit", (e) => {
  e.preventDefault();
  const user = new FormData(e.target).get("user").trim();
  if (!user) return;
  sendCommand("GROUP_INVITE", `GROUP INVITE ${user}`);
  e.target.reset();
});
$("#group-leave-btn").addEventListener("click", () => sendCommand("GROUP_LEAVE", "GROUP LEAVE"));
groupDisbandBtn.addEventListener("click", () => sendCommand("GROUP_DISBAND", "GROUP DISBAND"));

// terminal tabs
$$(".tab-btn[data-tab]").forEach((btn) => {
  btn.addEventListener("click", () => {
    state.activeTab = btn.dataset.tab;
    $$(".tab-btn[data-tab]").forEach((b) => b.classList.toggle("active", b === btn));
    Object.entries(panes).forEach(([name, el]) => el.classList.toggle("active", name === state.activeTab));
    const scopeLabel = $("#chat-scope-label");
    const chatForm = $("#chat-form");
    if (state.activeTab === "log") {
      chatForm.hidden = true;
    } else {
      chatForm.hidden = false;
      scopeLabel.textContent = `${state.activeTab}>`;
    }
  });
});

$("#chat-form").addEventListener("submit", (e) => {
  e.preventDefault();
  const input = $("#chat-input");
  const text = input.value.trim();
  if (!text) return;
  const scope = state.activeTab === "log" ? "global" : state.activeTab;
  sendCommand("CHAT", `CHAT ${scope.toUpperCase()} ${text}`);
  input.value = "";
});

// raw console (power users / protocol debugging)
$("#console-toggle").addEventListener("click", () => {
  const form = $("#console-form");
  form.hidden = !form.hidden;
  if (!form.hidden) $("#console-input").focus();
});
$("#console-form").addEventListener("submit", (e) => {
  e.preventDefault();
  const input = $("#console-input");
  const raw = input.value.trim();
  if (!raw) return;
  sendCommand("RAW", raw, { raw });
  input.value = "";
});

// npc popover actions
$("#npc-popover-close").addEventListener("click", closeNpcPopover);
$("#item-popover-close").addEventListener("click", closeItemPopover);
$("#npc-talk-btn").addEventListener("click", () => {
  if (!state.activeNpc) return;
  sendCommand("TALK", `TALK ${state.activeNpc}`, { npcId: state.activeNpc });
});
$("#npc-quest-btn").addEventListener("click", () => {
  if (!state.activeNpc) return;
  sendCommand("QUEST", `QUEST ${state.activeNpc}`, { npcId: state.activeNpc });
});
$("#npc-attack-btn").addEventListener("click", () => {
  if (!state.activeNpc) return;
  sendCommand("ATTACK", `ATTACK ${state.activeNpc}`, { npcId: state.activeNpc });
  closeNpcPopover();
});

document.addEventListener("click", (e) => {
  if (!npcPopover.hidden && !npcPopover.contains(e.target) && !e.target.closest('[data-action="npc"]')) {
    closeNpcPopover();
  }
  if (!itemPopover.hidden && !itemPopover.contains(e.target)
    && !e.target.closest('[data-action="item-info"]') && !e.target.closest('[data-info]')) {
    closeItemPopover();
  }
});

window.addEventListener("beforeunload", () => {
  if (state.ws && state.ws.readyState === WebSocket.OPEN) {
    try { state.ws.send("QUIT"); } catch { /* noop */ }
  }
});