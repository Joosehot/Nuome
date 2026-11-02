"use strict";

const MOST_ATTEMPTS = 3;
const STEP_MS = 450;
const $ = id => document.getElementById(id);

function load(key, otherwise) {
  try {
    const raw = localStorage.getItem(key);
    return raw === null ? otherwise : JSON.parse(raw);
  } catch (e) {
    return otherwise;
  }
}

function save(key, value) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch (e) {
    // the game goes on without being remembered
  }
}

function utcDay(d) {
  return d.toISOString().slice(0, 10);
}

function dayBefore(day) {
  return utcDay(new Date(Date.parse(day + "T00:00:00Z") - 86400000));
}

/** the n's a guess names, or null when it is not a list of whole numbers ("4, 6, 7", "n = 4 or n = 6") */
function readGuess(text) {
  const words = text.toLowerCase().replace(/n\s*=/g, " ").replace(/\band\b|\bor\b|,|;/g, " ").trim().split(/\s+/);
  if (words.length === 0 || words[0] === "" || !words.every(w => /^\d+$/.test(w))) return null;
  return [...new Set(words.map(Number))].sort((a, b) => a - b);
}

function sameSet(a, b) {
  return a.length === b.length && a.every((x, i) => x === b[i]);
}

function shareText(p, state) {
  const n = state.attempts.length;
  const line = state.won ? `Nuome Daily #${p.number} ✅ ${n}/${MOST_ATTEMPTS}` : `Nuome Daily #${p.number} ❌ X/${MOST_ATTEMPTS}`;
  return `${line}\n${location.origin}${location.pathname}`;
}

function showStats() {
  const s = load("nuome-daily-stats", null);
  if (!s) return;
  $("stats").textContent = `Played ${s.played} · Solved ${s.won} · Streak ${s.streak} · Best ${s.best}`;
}

function recordFinish(day, won) {
  const s = load("nuome-daily-stats", { played: 0, won: 0, streak: 0, best: 0, lastWin: null });
  s.played += 1;
  if (won) {
    s.won += 1;
    s.streak = s.lastWin === dayBefore(day) ? s.streak + 1 : 1;
    s.best = Math.max(s.best, s.streak);
    s.lastWin = day;
  } else {
    s.streak = 0;
  }
  save("nuome-daily-stats", s);
}

function addStep(text) {
  const box = $("solution");
  const m = text.match(/^(.*?)\s+\[(\w+)\]$/);
  const el = document.createElement("div");
  el.className = "step";
  el.textContent = m ? m[1] + "   " : text;
  if (m) {
    const rule = document.createElement("span");
    rule.className = "rule";
    rule.textContent = `[${m[2]}]`;
    el.append(rule);
  }
  box.append(el);
}

function addLine(text, cls) {
  const el = document.createElement("div");
  el.className = cls;
  el.textContent = text;
  $("solution").append(el);
}

function reveal(p) {
  $("reveal").hidden = true;
  const box = $("solution");
  box.hidden = false;
  box.textContent = "";
  const parts = [...p.steps.map(s => () => addStep(s)), () => addLine(p.answer_line, "final"), ...p.checks.map(c => () => addLine(c, "check"))];
  const still = matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (still) {
    parts.forEach(f => f());
    return;
  }
  let i = 0;
  const next = () => {
    parts[i++]();
    if (i < parts.length) setTimeout(next, STEP_MS);
  };
  next();
}

async function copy(text) {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch (e) {
    const area = document.createElement("textarea");
    area.value = text;
    document.body.append(area);
    area.select();
    let done = false;
    try {
      done = document.execCommand("copy");
    } catch (e2) {
      done = false;
    }
    area.remove();
    return done;
  }
}

function play(p, day) {
  const key = `nuome-daily-${day}`;
  const state = load(key, { attempts: [], won: false, over: false });

  $("meta").textContent = `#${p.number} · ${day}`;
  $("question").textContent = p.question;
  for (const d of p.definitions) {
    const li = document.createElement("li");
    li.textContent = d;
    $("definitions").append(li);
  }
  $("puzzle").hidden = false;

  function draw() {
    $("attempts").textContent = "";
    state.attempts.forEach((a, i) => {
      const li = document.createElement("li");
      li.textContent = `${i + 1}. n = ${a.join(", ")} ${state.won && i === state.attempts.length - 1 ? "✓" : "✗"}`;
      $("attempts").append(li);
    });
    const left = MOST_ATTEMPTS - state.attempts.length;
    $("guess").disabled = state.over;
    $("form").querySelector("button").disabled = state.over;
    $("giveup").hidden = state.over;
    $("reveal").hidden = !state.over || !$("solution").hidden;
    $("share").hidden = !state.over;
    const m = $("message");
    m.className = "";
    if (state.won) {
      m.textContent = "Correct.";
      m.className = "right";
    } else if (state.over) {
      m.textContent = `The answer: n = ${p.answer.join(", ")}`;
      m.className = "wrong";
    } else if (state.attempts.length) {
      m.textContent = `Not that. ${left} attempt${left === 1 ? "" : "s"} left.`;
      m.className = "wrong";
    } else {
      m.textContent = "";
    }
  }

  function finish(won) {
    state.over = true;
    state.won = won;
    recordFinish(day, won);
    showStats();
  }

  $("form").addEventListener("submit", e => {
    e.preventDefault();
    if (state.over) return;
    const guess = readGuess($("guess").value);
    if (guess === null) {
      $("message").textContent = "Write whole numbers, like 4, 6, 7.";
      $("message").className = "wrong";
      return;
    }
    if (guess.some(n => n < 3 || n > p.bound)) {
      $("message").textContent = `Only n from 3 to ${p.bound} are asked.`;
      $("message").className = "wrong";
      return;
    }
    if (state.attempts.some(a => sameSet(a, guess))) {
      $("message").textContent = "You already tried that.";
      $("message").className = "wrong";
      return;
    }
    state.attempts.push(guess);
    const right = sameSet(guess, p.answer);
    if (right || state.attempts.length >= MOST_ATTEMPTS) finish(right);
    save(key, state);
    $("guess").value = "";
    draw();
  });

  $("giveup").addEventListener("click", () => {
    if (state.over) return;
    finish(false);
    save(key, state);
    draw();
  });

  $("reveal").addEventListener("click", () => reveal(p));

  $("share").addEventListener("click", async () => {
    const text = shareText(p, state);
    $("shared").textContent = (await copy(text)) ? "Copied to clipboard." : text;
  });

  draw();
}

async function main() {
  showStats();
  const day = utcDay(new Date());
  try {
    const r = await fetch(`puzzles/${day}.json`);
    if (!r.ok) throw new Error(String(r.status));
    play(await r.json(), day);
  } catch (e) {
    $("none").hidden = false;
  }
}

main();
