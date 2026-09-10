"use strict";


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
const roomLockedEl = $("#room-locked");
const combatBarEl = $("#combat-bar");
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
const sellPopover = $("#sell-popover");
const sellOptionsEl = $("#sell-options");

const goldTextEl = $("#gold-text");
const bossIndicatorEl = $("#boss-indicator");
const bossLabelEl = $("#boss-label");
const shopListEl = $("#shop-list");
const marketListEl = $("#market-list");

const state = {
  ws: null,
  connected: false,
  me: { username: null, hp: null, maxHp: null, gold: null, status: "alive" },
  room: null,
  inventory: [],
  quests: [],
  shop: [],
  market: [],
  group: null,
  serverCount: 0,
  npcCache: {},
  itemCache: {},
  activeTab: "global",
  activeNpc: null,
  activeBoss: null,
  mapData: null,
  mapLayout: null,
  sellItem: null,
  lockedRoom: null,
  combatNpc: null,
};

const pending = [];

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
  "413": "There's no merchant here to trade with.",
  "414": "Something here blocks your way — defeat it first.",
  "415": "You are not in a fight right now.",
};

function friendlyError(line) {
  const m = line.match(/^ERR\s+(\d{3})\s*(.*)$/);
  if (!m) return line;
  const [, code, name] = m;
  return ERROR_MESSAGES[code] || name || `Error ${code}`;
}

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

  ws.addEventListener("error", () => {  });
}

function returnToLogin() {
  state.ws = null;
  state.me = { username: null, hp: null, maxHp: null, gold: null, status: "alive" };
  state.room = null;
  state.inventory = [];
  state.quests = [];
  state.shop = [];
  state.market = [];
  state.group = null;
  state.npcCache = {};
  state.itemCache = {};
  state.activeBoss = null;
  state.activeNpc = null;
  state.mapData = null;
  state.mapLayout = null;
  state.serverCount = 0;
  pending.length = 0;
  npcPopover.hidden = true;
  itemPopover.hidden = true;
  sellPopover.hidden = true;
  state.sellItem = null;
  state.lockedRoom = null;
  state.combatNpc = null;
  roomLockedEl.hidden = true;
  document.getElementById("minimap-inline").hidden = true;
  Object.values(panes).forEach((p) => { if (p) p.innerHTML = ""; });
  bossIndicatorEl.hidden = true;
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
  if (state.ws) { try { state.ws.close(); } catch {  } }
}

function onLine(line) {
  if (line == null) return;
  line = String(line);
  if (line.length === 0) return;

  const chatMatch = line.match(/^\((GLOBAL|ROOM|GROUP)\)\s+(\S+):\s?(.*)$/);
  if (chatMatch) {
    const [, scope, who, text] = chatMatch;
    logChat(scope.toLowerCase(), who, text);
    return;
  }

  if (line.startsWith("EVT ")) {
    handleEvent(line.slice(4));
    return;
  }

  if (line.startsWith("ERR connection failed")) {
    if (pending.length && pending[0].type === "GREETING") pending.shift();
    if (pending.length && pending[0].type === "CONNECT") pending.shift();
    loginFailed(line.replace(/^ERR\s+/, ""));
    return;
  }

  const ctx = pending.shift();
  if (!ctx) { logRaw(line); return; }
  handleResponse(ctx, line);
}

function handleResponse(ctx, line) {
  const isErr = line.startsWith("ERR");

  switch (ctx.type) {
    case "GREETING":
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
      if (isErr) {
        if (ctx.meta.quiet) return;
        showToast({ text: friendlyError(line), type: "error" });
        closeItemPopover();
        return;
      }
      const data = safeJson(line.slice(3));
      if (!data) return;
      state.itemCache[data.id] = data;
      if (ctx.meta.quiet) { renderSellPopover(); return; }
      showItemPopover(data);
      return;
    }

    case "TAKE":
      if (isErr) { showToast({ text: friendlyError(line), type: "error" }); return; }
      sendCommand("LOOK", "LOOK");
      sendCommand("INVENTORY", "INVENTORY");
      sendCommand("QUESTS", "QUESTS");
      return;

    case "DROP":
      if (isErr) { showToast({ text: friendlyError(line), type: "error" }); return; }
      sendCommand("LOOK", "LOOK");
      sendCommand("INVENTORY", "INVENTORY");
      sendCommand("QUESTS", "QUESTS");
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
        if (data.status === "victory") {
          delete state.npcCache[ctx.meta.npcId];
        } else {
          state.npcCache[ctx.meta.npcId] = {
            ...(state.npcCache[ctx.meta.npcId] || {}),
            hostile: true,
            hp: data.target_hp,
          };
        }
      }
      const npcLabel = state.npcCache[ctx.meta.npcId]?.name || humanize(ctx.meta.npcId);
      const armorMsg = data.absorbed > 0 ? ` (${data.absorbed} absorbed)` : "";
      if (data.status === "victory") {
        const goldMsg = data.gold_earned ? ` +${data.gold_earned} gold` : "";
        logCombat(`You defeated ${npcLabel}! (-${data.damage} HP dealt)${goldMsg}`);
      }
      else if (data.status === "death") logCombat(`${npcLabel} struck you down.${armorMsg} You wake up back at a safe place.`);
      else logCombat(`You hit ${npcLabel} for ${data.damage}. They hit you for ${data.npc_damage}${armorMsg}. [${npcLabel}: ${data.target_hp} HP | You: ${data.attacker_hp} HP]`);
      state.combatNpc = (data.status === "victory" || data.status === "death") ? null : ctx.meta.npcId;
      renderCombat();
      sendCommand("STATUS", "STATUS");
      if (data.status === "victory") sendCommand("QUESTS", "QUESTS");
      if (data.status === "victory" || data.status === "death") sendCommand("LOOK", "LOOK");
      else renderRoom();
      return;
    }

    case "DEFEND": {
      if (isErr) { showToast({ text: friendlyError(line), type: "error" }); return; }
      const data = safeJson(line.slice(3));
      if (!data) return;
      const label = state.npcCache[state.combatNpc]?.name || humanize(state.combatNpc);
      if (data.status === "death") {
        state.combatNpc = null;
        logCombat(`${label} broke through your guard. You wake up back at a safe place.`);
        sendCommand("LOOK", "LOOK");
      } else {
        logCombat(`You brace against ${label}: ${data.blocked} blocked, ${data.npc_damage} through. [You: ${data.attacker_hp} HP]`);
      }
      sendCommand("STATUS", "STATUS");
      renderCombat();
      return;
    }

    case "FLEE": {
      if (isErr) { showToast({ text: friendlyError(line), type: "error" }); return; }
      const data = safeJson(line.slice(3));
      if (!data) return;
      const label = state.npcCache[state.combatNpc]?.name || humanize(state.combatNpc);
      if (data.fled) {
        state.combatNpc = null;
        logCombat(`You break away from ${label} and fall back.`);
        showToast({ text: "You got away." });
        sendCommand("LOOK", "LOOK");
      } else if (data.status === "death") {
        state.combatNpc = null;
        logCombat(`${label} cut you down as you turned to run.`);
        sendCommand("LOOK", "LOOK");
      } else {
        logCombat(`You fail to break away — ${label} hits you for ${data.npc_damage}. [You: ${data.attacker_hp} HP]`);
        showToast({ text: "You couldn't get away.", type: "error" });
      }
      sendCommand("STATUS", "STATUS");
      renderCombat();
      return;
    }

    case "ABANDON_QUEST": {
      if (isErr) { showToast({ text: friendlyError(line), type: "error" }); return; }
      const data = safeJson(line.slice(3));
      showToast({ text: `Abandoned ${humanize(data?.quest_id || "")}.` });
      sendCommand("QUESTS", "QUESTS");
      sendCommand("INVENTORY", "INVENTORY");
      return;
    }

    case "STATUS": {
      if (isErr) return;
      const data = safeJson(line.slice(3));
      if (!data) return;
      state.me.hp = data.hp;
      state.me.maxHp = data.max_hp;
      state.me.gold = data.gold;
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
      } else if (data.type === "deliver") {
        showToast({ text: `New quest: ${data.description} — ${humanize(data.target_item)} added to your pack.` });
      } else {
        showToast({ text: `New quest: ${data.description}` });
      }
      // Accepting a delivery hands over the parcel, so the inventory moves either way.
      sendCommand("INVENTORY", "INVENTORY");
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

    case "SHOP": {
      if (isErr) return;
      const arr = safeJson(line.slice(3));
      state.shop = Array.isArray(arr) ? arr : [];
      renderShop();
      return;
    }

    case "SHOP_BUY": {
      if (isErr) { showToast({ text: friendlyError(line), type: "error" }); return; }
      const m = line.match(/bought=(\S+) price=(\d+) gold=(\d+)/);
      if (m) {
        state.me.gold = Number(m[3]);
        renderHp();
        showToast({ text: `Bought ${humanize(m[1])} for ${m[2]} gold.` });
      }
      sendCommand("INVENTORY", "INVENTORY");
      return;
    }

    case "SHOP_SELL": {
      if (isErr) { showToast({ text: friendlyError(line), type: "error" }); return; }
      const m = line.match(/sold=(\S+) price=(\d+) gold=(\d+)/);
      if (m) {
        state.me.gold = Number(m[3]);
        renderHp();
        showToast({ text: `Sold ${humanize(m[1])} to the merchant for ${m[2]} gold.` });
      }
      sendCommand("INVENTORY", "INVENTORY");
      return;
    }

    case "MARKET": {
      if (isErr) return;
      const arr = safeJson(line.slice(3));
      state.market = Array.isArray(arr) ? arr : [];
      renderMarket();
      return;
    }

    case "MARKET_BUY": {
      if (isErr) { showToast({ text: friendlyError(line), type: "error" }); return; }
      const m = line.match(/bought=(\S+) price=(\d+) seller=(\S+) gold=(\d+)/);
      if (m) {
        state.me.gold = Number(m[4]);
        renderHp();
        showToast({ text: `Bought ${humanize(m[1])} from ${m[3]} for ${m[2]} gold.` });
      }
      sendCommand("INVENTORY", "INVENTORY");
      sendCommand("MARKET", "MARKET");
      return;
    }

    case "MARKET_CANCEL": {
      if (isErr) { showToast({ text: friendlyError(line), type: "error" }); return; }
      const m = line.match(/cancelled=(\S+)/);
      if (m) {
        showToast({ text: `Cancelled listing for ${humanize(m[1])}.` });
      }
      sendCommand("INVENTORY", "INVENTORY");
      sendCommand("MARKET", "MARKET");
      return;
    }

    case "USE": {
      if (isErr) { showToast({ text: friendlyError(line), type: "error" }); return; }
      const data = safeJson(line.slice(3));
      if (data) {
        state.me.hp = data.hp;
        state.me.maxHp = data.max_hp;
        renderHp();
        showToast({ text: `Used ${humanize(data.used)}: +${data.heal} HP` });
      }
      sendCommand("INVENTORY", "INVENTORY");
      return;
    }

    case "SELL": {
      if (isErr) { showToast({ text: friendlyError(line), type: "error" }); return; }
      const m = line.match(/listed=(\S+) price=(\d+)/);
      if (m) {
        showToast({ text: `Listed ${humanize(m[1])} for ${m[2]} gold on the market.` });
      }
      sendCommand("INVENTORY", "INVENTORY");
      sendCommand("MARKET", "MARKET");
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

    case "MAP": {
      if (isErr) return;
      const data = safeJson(line.slice(3));
      if (data) {
        state.mapData = data;
        state.mapLayout = computeMapLayout(data);
      }
      return;
    }

    default:
      logRaw(line);
  }
}

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
    const combatText = rest.slice("ROOM COMBAT ".length);
    const defeatedMatch = combatText.match(/^(.+) defeated by (\S+) npc=(\S+)$/);
    if (defeatedMatch) {
      const [, , , npcId] = defeatedMatch;
      delete state.npcCache[npcId];
      if (state.room) state.room.npcs = (state.room.npcs || []).filter((n) => n !== npcId);
      renderRoom();
    } else {
      const hpMatch = combatText.match(/npc=(\S+) hp=(\d+)$/);
      if (hpMatch) {
        const [, npcId, hp] = hpMatch;
        state.npcCache[npcId] = { ...(state.npcCache[npcId] || {}), hostile: true, hp: Number(hp) };
        renderRoom();
      }
    }
    logCombat(combatText.replace(/ npc=\S+( hp=\d+)?$/, ""));
    return;
  }
  if (rest.startsWith("COMBAT REWARD ")) {
    const m = rest.match(/^COMBAT REWARD npc=(\S+) gold=(\d+) total=(\d+)$/);
    if (m) {
      const [, npcId, gold, total] = m;
      const npcLabel = state.npcCache[npcId]?.name || humanize(npcId);
      state.me.gold = Number(total);
      renderHp();
      const earned = Number(gold);
      logCombat(earned > 0
        ? `${npcLabel} was defeated — you earn ${gold} gold for taking part.`
        : `${npcLabel} was defeated — your part in it counts.`);
      if (earned > 0) showToast({ text: `+${gold} gold for helping defeat ${npcLabel}.` });
      // The kill was credited to every attacker, so a kill quest may have moved.
      sendCommand("QUESTS", "QUESTS");
    }
    return;
  }
  if (rest.startsWith("ROOM ITEM TAKEN ")) {
    const m = rest.match(/^ROOM ITEM TAKEN (\S+) (\S+)$/);
    if (m) {
      const [, itemId, who] = m;
      if (state.room) state.room.items = (state.room.items || []).filter((i) => i !== itemId);
      logEvent(`${who} picks up ${state.itemCache[itemId]?.name || humanize(itemId)}.`);
      renderRoom();
    }
    return;
  }
  if (rest.startsWith("ROOM ITEM DROPPED ")) {
    const m = rest.match(/^ROOM ITEM DROPPED (\S+) (\S+)$/);
    if (m) {
      const [, itemId, who] = m;
      if (state.room && !(state.room.items || []).includes(itemId)) {
        state.room.items = [...(state.room.items || []), itemId];
      }
      logEvent(`${who} drops ${state.itemCache[itemId]?.name || humanize(itemId)}.`);
      renderRoom();
    }
    return;
  }
  if (rest.startsWith("ROOM ITEM RESPAWN ")) {
    const itemId = rest.slice("ROOM ITEM RESPAWN ".length).trim();
    if (state.room && !(state.room.items || []).includes(itemId)) {
      state.room.items = [...(state.room.items || []), itemId];
    }
    logEvent(`${state.itemCache[itemId]?.name || humanize(itemId)} reappears.`);
    renderRoom();
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
  if (rest.startsWith("MARKET LISTED ")) {
    const m = rest.match(/^MARKET LISTED (\S+) (\S+) (\d+)$/);
    if (m) {
      const [, seller, itemId, price] = m;
      logEvent(`${seller} lists ${state.itemCache[itemId]?.name || humanize(itemId)} for ${price} gold.`);
      sendCommand("MARKET", "MARKET");
    }
    return;
  }
  if (rest.startsWith("MARKET CANCELLED ")) {
    const m = rest.match(/^MARKET CANCELLED (\S+) (\S+)$/);
    if (m) {
      const [, seller, itemId] = m;
      logEvent(`${seller} withdraws ${state.itemCache[itemId]?.name || humanize(itemId)} from the market.`);
      sendCommand("MARKET", "MARKET");
    }
    return;
  }
  if (rest.startsWith("MARKET BOUGHT ")) {
    const m = rest.match(/^MARKET BOUGHT (\S+) (\S+)$/);
    if (m) {
      const [, buyer, itemId] = m;
      logEvent(`${buyer} buys ${state.itemCache[itemId]?.name || humanize(itemId)} off the market.`);
      sendCommand("MARKET", "MARKET");
    }
    return;
  }
  if (rest.startsWith("MARKET SOLD ")) {
    const msg = rest.slice("MARKET SOLD ".length).trim();
    logEvent(msg);
    showToast({ text: msg });
    sendCommand("STATUS", "STATUS");
    sendCommand("MARKET", "MARKET");
    return;
  }
  if (rest.startsWith("GLOBAL JOIN ")) {
    const who = rest.slice("GLOBAL JOIN ".length).trim();
    logEvent(`${who} joined the world.`);
    return;
  }
  if (rest.startsWith("GLOBAL LEAVE ")) {
    const who = rest.slice("GLOBAL LEAVE ".length).trim();
    logEvent(`${who} left the world.`);
    return;
  }
  if (rest.startsWith("GLOBAL ")) {
    const body = rest.slice("GLOBAL ".length);
    if (body.startsWith("[ALERT] ")) {
      const bossMatch = body.match(/boss=(\S+)\s+room=(\S+)$/);
      const text = body.replace(/^\[ALERT\]\s*/, "").replace(/\s*boss=\S+\s+room=\S+$/, "");
      logCombat(text);
      showToast({ text, type: "error", timeout: 8000 });
      if (bossMatch) {
        state.activeBoss = { id: bossMatch[1], room: bossMatch[2], name: text.split("...")[0].trim() };
        renderBossIndicator();
      }
    } else if (body.startsWith("[DEFEAT] ")) {
      const text = body.replace(/^\[DEFEAT\]\s*/, "").replace(/\s*boss=\S+$/, "");
      logCombat(text);
      showToast({ text, timeout: 6000 });
      state.activeBoss = null;
      renderBossIndicator();
    } else {
      logCombat(body);
    }
    return;
  }
  if (rest.startsWith("STATS ")) {
    const m = rest.match(/players=(\d+)/);
    if (m) {
      state.serverCount = Number(m[1]);
      serverCountEl.textContent = state.serverCount;
    }
    return;
  }
  if (rest.startsWith("DISCONNECTED")) {
    logSystem(rest);
    showToast({ text: "The server closed the connection.", type: "error", timeout: 6000 });
    setTimeout(returnToLogin, 800);
    return;
  }
  logRaw(`EVT ${rest}`);
}

function renderHp() {
  const { hp, maxHp, gold, status } = state.me;
  const pct = maxHp ? Math.max(0, Math.min(100, (hp / maxHp) * 100)) : 0;
  [hpFillTop, hpFillMain].forEach((el) => {
    el.style.width = `${pct}%`;
    el.classList.toggle("low", pct <= 30);
  });
  const text = hp != null ? `${hp}/${maxHp}` : "—/—";
  hpTextTop.textContent = text;
  hpTextMain.textContent = text;
  goldTextEl.textContent = gold != null ? gold : "—";
  statusPill.textContent = status || "alive";
  statusPill.classList.toggle("dead", status === "dead");
}

function renderCombat() {
  const foe = state.combatNpc;
  combatBarEl.hidden = !foe;
  if (foe) $("#combat-foe").textContent = state.npcCache[foe]?.name || humanize(foe);
}

function renderBossIndicator() {
  if (state.activeBoss) {
    bossLabelEl.textContent = `${humanize(state.activeBoss.id)} @ ${humanize(state.activeBoss.room)}`;
    bossIndicatorEl.hidden = false;
  } else {
    bossIndicatorEl.hidden = true;
  }
}

const sleepBtn = $("#sleep-btn");

function applyRoom(room) {
  if (!room) return;
  if (state.room && room.id !== state.room.id) state.combatNpc = null;
  state.room = room;
  renderRoom();
  renderInventory();
  renderCombat();
  if (!document.getElementById("minimap-inline").hidden) renderMinimap();
}

const DIRECTION_ORDER = ["north", "east", "south", "west", "up", "down", "in", "out"];

function renderRoom() {
  const room = state.room;
  if (!room) return;

  roomNameEl.textContent = room.name || "—";
  roomIdEl.textContent = room.id || "";
  roomDescEl.textContent = room.description || "";

  exitRowEl.innerHTML = "";
  const exits = room.exits || {};
  [["north", "n"], ["west", "w"], ["east", "e"], ["south", "s"]].forEach(([dir, slot]) => {
    const btn = document.createElement("button");
    btn.className = `exit-btn compass-${slot}`;
    btn.type = "button";
    btn.textContent = humanize(dir);
    const barred = room.locked && exits[dir] && exits[dir] !== room.locked_exit;
    if (exits[dir] && !barred) {
      btn.title = `Move ${dir} → ${humanize(exits[dir])}`;
      btn.onclick = () => sendCommand("MOVE", `MOVE ${dir}`);
    } else if (barred) {
      btn.disabled = true;
      btn.title = "Blocked — defeat what guards this room first";
      btn.classList.add("barred");
    } else {
      btn.disabled = true;
      btn.title = "No exit";
    }
    exitRowEl.appendChild(btn);
  });
  const center = document.createElement("button");
  center.className = "exit-btn compass-c";
  center.type = "button";
  center.disabled = true;
  center.textContent = "◈";
  exitRowEl.appendChild(center);

  if (room.locked) {
    const back = room.locked_exit ? humanize(room.locked_exit) : "the way you came";
    const msg = `Barred. Something here still guards this room — until it falls, the only way out is back to ${back}.`;
    roomLockedEl.textContent = msg;
    roomLockedEl.hidden = false;
    if (state.lockedRoom !== room.id) { state.lockedRoom = room.id; logCombat(msg); }
  } else {
    roomLockedEl.hidden = true;
    if (state.lockedRoom === room.id) { state.lockedRoom = null; logEvent("The way is clear — this room no longer holds you."); }
  }

  const others = (room.players || []).filter((p) => p !== state.me.username);
  playersListEl.innerHTML = others.length
    ? others.map((p) => `<li class="chip">${escapeHtml(p)}</li>`).join("")
    : '<li class="empty-note">you\'re alone</li>';
  roomCountEl.textContent = (room.players || []).length;

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

  sleepBtn.hidden = !room.can_sleep;
}

function renderInventory() {
  inventoryListEl.innerHTML = state.inventory.length
    ? state.inventory.map((id) => `
        <li class="inventory-item">
          <span>${escapeHtml(state.itemCache[id]?.name || humanize(id))}</span>
          <span class="inventory-item-actions">
            <button class="btn btn-ghost btn-xs" data-info="${escapeHtml(id)}" type="button" title="Item info" aria-label="Item info">ⓘ</button>
            <button class="btn btn-ghost btn-xs" data-use="${escapeHtml(id)}" type="button">use</button>
            <button class="btn btn-ghost btn-xs" data-sell="${escapeHtml(id)}" type="button">sell</button>
            <button class="btn btn-ghost btn-xs" data-drop="${escapeHtml(id)}" type="button">drop</button>
          </span>
        </li>`).join("")
    : '<li class="empty-note">empty-handed</li>';
  $$('[data-drop]', inventoryListEl).forEach((btn) => {
    btn.onclick = () => sendCommand("DROP", `DROP ${btn.dataset.drop}`);
  });
  $$('[data-use]', inventoryListEl).forEach((btn) => {
    btn.onclick = () => sendCommand("USE", `USE ${btn.dataset.use}`);
  });
  $$('[data-sell]', inventoryListEl).forEach((btn) => {
    btn.onclick = (ev) => openSellPopover(btn.dataset.sell, ev.currentTarget);
  });
  $$('[data-info]', inventoryListEl).forEach((btn) => {
    btn.onclick = (ev) => requestItemInfo(btn.dataset.info, ev.currentTarget);
  });
}

function renderShop() {
  shopListEl.innerHTML = state.shop.length
    ? state.shop.map((item) => {
        const stats = [
          item.damage ? `${item.damage} dmg` : "",
          item.armor ? `${item.armor} def` : "",
          item.heal ? `+${item.heal} hp` : "",
        ].filter(Boolean).join(", ");
        return `<li class="inventory-item">
          <span>${escapeHtml(item.name)} <span class="entity-mark">${item.price}g${stats ? " · " + stats : ""}</span></span>
          <button class="btn btn-secondary btn-xs" data-shopbuy="${escapeHtml(item.id)}" type="button">Buy</button>
        </li>`;
      }).join("")
    : '<li class="empty-note">shop is empty</li>';
  $$('[data-shopbuy]', shopListEl).forEach((btn) => {
    btn.onclick = () => sendCommand("SHOP_BUY", `SHOP BUY ${btn.dataset.shopbuy}`);
  });
}

function renderMarket() {
  marketListEl.innerHTML = state.market.length
    ? state.market.map((listing) => {
        const isMine = listing.seller === state.me.username;
        return `<li class="inventory-item">
          <span>${escapeHtml(listing.name)} <span class="entity-mark">${listing.price}g · by ${escapeHtml(listing.seller)}</span></span>
          ${!isMine
            ? `<button class="btn btn-secondary btn-xs" data-marketbuy="${listing.index}" type="button">Buy</button>`
            : `<button class="btn btn-danger btn-xs" data-marketcancel="${listing.index}" type="button">Cancel</button>`}
        </li>`;
      }).join("")
    : '<li class="empty-note">nothing for sale</li>';
  $$('[data-marketbuy]', marketListEl).forEach((btn) => {
    btn.onclick = () => sendCommand("MARKET_BUY", `MARKET BUY ${btn.dataset.marketbuy}`);
  });
  $$('[data-marketcancel]', marketListEl).forEach((btn) => {
    btn.onclick = () => sendCommand("MARKET_CANCEL", `MARKET CANCEL ${btn.dataset.marketcancel}`);
  });
}

function renderQuests() {
  if (!state.quests.length) {
    questListEl.innerHTML = '<li class="empty-note">no quests yet — talk to someone</li>';
    return;
  }
  questListEl.innerHTML = state.quests.map((q, i) => {
    const done = q.status === "completed";
    const prog = done ? "completed" : escapeHtml(q.progress || "");
    return `<li class="quest-item${done ? " completed" : ""}">
      <div class="quest-head">
        <span class="quest-name">${escapeHtml(humanize(q.quest_id))}</span>
        <span class="quest-progress">${prog}</span>
      </div>
      <div class="quest-desc" id="quest-desc-${i}" hidden>${escapeHtml(q.description || "")}</div>
      <button class="btn btn-ghost btn-xs quest-info-btn" data-quest-idx="${i}" type="button">details</button>
      ${done ? "" : `<button class="btn btn-ghost btn-xs quest-info-btn" data-abandon="${escapeHtml(q.quest_id)}" type="button">abandon</button>`}
    </li>`;
  }).join("");
  $$("[data-abandon]", questListEl).forEach((btn) => {
    btn.onclick = () => sendCommand("ABANDON_QUEST", `ABANDON_QUEST ${btn.dataset.abandon}`);
  });
  $$("[data-quest-idx]", questListEl).forEach((btn) => {
    btn.onclick = () => {
      const desc = $(`#quest-desc-${btn.dataset.questIdx}`);
      if (desc) { desc.hidden = !desc.hidden; btn.textContent = desc.hidden ? "details" : "hide"; }
    };
  });
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

// Mirrors MERCHANT_RATE in src/commands/shop.rs — the merchant pays 80%.
const MERCHANT_RATE = 0.8;

function openSellPopover(itemId, anchorEl) {
  state.sellItem = itemId;
  renderSellPopover();

  sellPopover.hidden = false;
  const rect = anchorEl.getBoundingClientRect();
  const top = Math.min(window.innerHeight - 200, rect.bottom + 8 + window.scrollY);
  const left = Math.min(window.innerWidth - 280, rect.left + window.scrollX);
  sellPopover.style.top = `${Math.max(8, top)}px`;
  sellPopover.style.left = `${Math.max(8, left)}px`;

  // The price only becomes known once the item has been examined.
  if (state.itemCache[itemId]?.value == null) {
    sendCommand("EXAMINE", `EXAMINE ${itemId}`, { itemId, quiet: true });
  }
}

function closeSellPopover() {
  sellPopover.hidden = true;
  state.sellItem = null;
}

function renderSellPopover() {
  const itemId = state.sellItem;
  if (!itemId) return;

  const cached = state.itemCache[itemId];
  const value = cached?.value;
  const canTrade = !!state.room?.can_trade;

  $("#sell-popover-name").textContent = `Sell ${cached?.name || humanize(itemId)}`;

  const merchantPrice = value != null ? Math.max(1, Math.floor(value * MERCHANT_RATE)) : null;
  const merchantNote = canTrade
    ? (merchantPrice != null ? `${merchantPrice} gold, paid now` : "Paid immediately, at a reduced price")
    : "Only at the Marketplace";
  const marketNote = value != null
    ? `${value} gold, once a buyer takes it`
    : "Full price, whenever a buyer takes it";

  sellOptionsEl.innerHTML = `
    <button class="sell-option" type="button" data-choice="merchant"${canTrade ? "" : " disabled"}>
      <span class="sell-option-label">Merchant</span>
      <span class="sell-option-note">${escapeHtml(merchantNote)}</span>
    </button>
    <button class="sell-option" type="button" data-choice="market">
      <span class="sell-option-label">Player market</span>
      <span class="sell-option-note">${escapeHtml(marketNote)}</span>
    </button>`;

  $$("[data-choice]", sellOptionsEl).forEach((btn) => {
    btn.onclick = () => {
      if (btn.dataset.choice === "merchant") sendCommand("SHOP_SELL", `SHOP SELL ${itemId}`);
      else sendCommand("SELL", `SELL ${itemId}`);
      closeSellPopover();
    };
  });
}

function refreshAll() {
  sendCommand("LOOK", "LOOK");
  sendCommand("STATUS", "STATUS");
  sendCommand("INVENTORY", "INVENTORY");
  sendCommand("QUESTS", "QUESTS");
  sendCommand("WHO", "WHO");
  sendCommand("SHOP", "SHOP");
  sendCommand("MARKET", "MARKET");
  sendCommand("MAP", "MAP");
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
$("#look-refresh").addEventListener("click", () => refreshAll());
sleepBtn.addEventListener("click", () => sendCommand("SLEEP", "SLEEP"));
$("#defend-btn").addEventListener("click", () => sendCommand("DEFEND", "DEFEND"));
$("#flee-btn").addEventListener("click", () => sendCommand("FLEE", "FLEE"));
$("#inventory-refresh").addEventListener("click", () => sendCommand("INVENTORY", "INVENTORY"));
$("#shop-refresh").addEventListener("click", () => sendCommand("SHOP", "SHOP"));
$("#market-refresh").addEventListener("click", () => sendCommand("MARKET", "MARKET"));
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

$$(".tab-btn[data-tab]").forEach((btn) => {
  btn.addEventListener("click", () => {
    state.activeTab = btn.dataset.tab;
    $$(".tab-btn[data-tab]").forEach((b) => b.classList.toggle("active", b === btn));
    ["global", "room", "group"].forEach(name => {
      panes[name].classList.toggle("active", name === state.activeTab);
    });
    $("#chat-scope-label").textContent = `${state.activeTab}>`;
  });
});

$("#chat-form").addEventListener("submit", (e) => {
  e.preventDefault();
  const input = $("#chat-input");
  const text = input.value.trim();
  if (!text) return;
  const scope = state.activeTab;
  sendCommand("CHAT", `CHAT ${scope.toUpperCase()} ${text}`);
  input.value = "";
});


$("#npc-popover-close").addEventListener("click", closeNpcPopover);
$("#item-popover-close").addEventListener("click", closeItemPopover);
$("#sell-popover-close").addEventListener("click", closeSellPopover);
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

const ARROW_KEYS = {
  ArrowUp: "north",
  ArrowDown: "south",
  ArrowLeft: "west",
  ArrowRight: "east",
};

document.addEventListener("keydown", (e) => {
  const dir = ARROW_KEYS[e.key];
  if (!dir || e.ctrlKey || e.metaKey || e.altKey) return;
  if (screenGame.hidden || !state.room) return;
  const el = document.activeElement;
  if (el && (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.isContentEditable)) return;

  e.preventDefault();
  if (!state.room.exits?.[dir]) {
    showToast({ text: "You can't go that way.", type: "error", timeout: 1800 });
    return;
  }
  sendCommand("MOVE", `MOVE ${dir}`);
});

document.addEventListener("click", (e) => {
  if (!npcPopover.hidden && !npcPopover.contains(e.target) && !e.target.closest('[data-action="npc"]')) {
    closeNpcPopover();
  }
  if (!itemPopover.hidden && !itemPopover.contains(e.target)
    && !e.target.closest('[data-action="item-info"]') && !e.target.closest('[data-info]')) {
    closeItemPopover();
  }
  if (!sellPopover.hidden && !sellPopover.contains(e.target) && !e.target.closest('[data-sell]')) {
    closeSellPopover();
  }
});

function computeMapLayout(mapData) {
  const hasCoords = Object.values(mapData.rooms).some((r) => r.map_x != null && r.map_y != null);

  const positions = {};
  if (hasCoords) {
    for (const [id, room] of Object.entries(mapData.rooms)) {
      if (room.map_x != null && room.map_y != null) {
        positions[id] = { col: room.map_x, row: room.map_y };
      }
    }
  } else {
    const DIR_OFF = { north: [0, -1], south: [0, 1], east: [1, 0], west: [-1, 0] };
    const grid = {};
    const visited = new Set();
    const queue = [{ id: mapData.spawn, col: 0, row: 0 }];
    while (queue.length > 0) {
      const { id, col, row } = queue.shift();
      if (visited.has(id)) continue;
      let fc = col, fr = row, key = `${fc},${fr}`;
      if (grid[key]) {
        for (let r = 1; r < 20; r++) {
          let placed = false;
          for (const [dc, dr] of [[r,0],[-r,0],[0,r],[0,-r]]) {
            const k = `${col+dc},${row+dr}`;
            if (!grid[k]) { fc = col+dc; fr = row+dr; placed = true; break; }
          }
          if (placed) break;
        }
      }
      key = `${fc},${fr}`;
      grid[key] = id;
      positions[id] = { col: fc, row: fr };
      visited.add(id);
      const room = mapData.rooms[id];
      if (!room) continue;
      for (const [dir, targetId] of Object.entries(room.exits)) {
        if (visited.has(targetId)) continue;
        const [dc, dr] = DIR_OFF[dir] || [0, 0];
        queue.push({ id: targetId, col: fc + dc, row: fr + dr });
      }
    }
  }

  let minC = Infinity, maxC = -Infinity, minR = Infinity, maxR = -Infinity;
  for (const { col, row } of Object.values(positions)) {
    minC = Math.min(minC, col); maxC = Math.max(maxC, col);
    minR = Math.min(minR, row); maxR = Math.max(maxR, row);
  }

  const cellW = 80, cellH = 70, padX = 60, padY = 40;
  const pixels = {};
  for (const [id, { col, row }] of Object.entries(positions)) {
    pixels[id] = {
      x: padX + (col - minC) * cellW,
      y: padY + (row - minR) * cellH,
    };
  }

  const edges = [];
  const seen = new Set();
  for (const [id, room] of Object.entries(mapData.rooms)) {
    for (const [dir, targetId] of Object.entries(room.exits)) {
      const ek = [id, targetId].sort().join("|");
      if (seen.has(ek)) continue;
      seen.add(ek);
      const reverseDir = Object.entries(mapData.rooms[targetId]?.exits || {})
        .find(([, t]) => t === id);
      edges.push({
        a: id,
        b: targetId,
        dirA: dir,
        dirB: reverseDir ? reverseDir[0] : null,
      });
    }
  }

  return {
    pixels,
    edges,
    width: padX * 2 + (maxC - minC) * cellW,
    height: padY * 2 + (maxR - minR) * cellH,
  };
}

function renderMinimap() {
  const layout = state.mapLayout;
  if (!layout) return;
  const svg = document.getElementById("minimap-svg");
  if (!svg) return;

  const currentRoom = state.room?.id;
  svg.setAttribute("viewBox", `0 0 ${layout.width} ${layout.height}`);

  const nodes = Object.entries(layout.pixels);
  let html = "";

  for (const { a, b, dirA, dirB } of layout.edges) {
    const pa = layout.pixels[a], pb = layout.pixels[b];
    if (!pa || !pb) continue;

    const dx = pb.x - pa.x, dy = pb.y - pa.y;
    const len = Math.hypot(dx, dy);
    if (len === 0) continue;
    const nx = dx / len, ny = dy / len;
    const px = -ny, py = nx;

    let side = 0;
    for (const [id, p] of nodes) {
      if (id === a || id === b) continue;
      const t = ((p.x - pa.x) * nx + (p.y - pa.y) * ny) / len;
      if (t <= 0.05 || t >= 0.95) continue;
      const perp = (p.x - pa.x) * px + (p.y - pa.y) * py;
      if (Math.abs(perp) < 20) { side = perp >= 0 ? -1 : 1; break; }
    }

    const mx = (pa.x + pb.x) / 2, my = (pa.y + pb.y) / 2;
    const cx = mx + px * side * 56, cy = my + py * side * 56;
    const at = (t) => ({
      x: (1 - t) ** 2 * pa.x + 2 * (1 - t) * t * cx + t * t * pb.x,
      y: (1 - t) ** 2 * pa.y + 2 * (1 - t) * t * cy + t * t * pb.y,
    });

    html += side
      ? `<path d="M${pa.x} ${pa.y} Q${cx.toFixed(1)} ${cy.toFixed(1)} ${pb.x} ${pb.y}" class="minimap-edge" fill="none"/>`
      : `<line x1="${pa.x}" y1="${pa.y}" x2="${pb.x}" y2="${pb.y}" class="minimap-edge"/>`;

    const label = (p, dir, from) => {
      if (!dir) return "";
      const live = from === currentRoom;
      const letter = dir[0].toUpperCase();
      const text = `<text x="${p.x.toFixed(1)}" y="${(p.y + 2.5).toFixed(1)}" class="minimap-dir${live ? " live" : ""}">${letter}</text>`;
      return live
        ? `<g class="minimap-move" data-move="${dir}"><circle cx="${p.x.toFixed(1)}" cy="${p.y.toFixed(1)}" r="9" class="minimap-hit"/>${text}</g>`
        : text;
    };

    const tOff = Math.min(0.4, 24 / len);
    html += label(at(tOff), dirA, a);
    html += label(at(1 - tOff), dirB, b);
  }

  for (const [id, p] of Object.entries(layout.pixels)) {
    const cur = id === currentRoom;
    const label = state.mapData.rooms[id]?.name || id;
    html += `<circle cx="${p.x}" cy="${p.y}" r="${cur ? 8 : 5}" class="minimap-node${cur ? " current" : ""}"/>`;
    html += `<text x="${p.x}" y="${p.y - 11}" class="minimap-label${cur ? " current" : ""}">${escapeHtml(label)}</text>`;
  }

  svg.innerHTML = html;

  $$("[data-move]", svg).forEach((el) => {
    el.addEventListener("click", () => sendCommand("MOVE", `MOVE ${el.dataset.move}`));
  });
}

function toggleMinimap() {
  const panel = document.getElementById("minimap-inline");
  if (panel.hidden) {
    renderMinimap();
    panel.hidden = false;
    panel.scrollIntoView({ block: "nearest", behavior: "smooth" });
  } else {
    panel.hidden = true;
  }
}

$("#minimap-btn").addEventListener("click", toggleMinimap);
$("#minimap-close").addEventListener("click", () => {
  document.getElementById("minimap-inline").hidden = true;
});

window.addEventListener("beforeunload", () => {
  if (state.ws && state.ws.readyState === WebSocket.OPEN) {
    try { state.ws.send("QUIT"); } catch {  }
  }
});