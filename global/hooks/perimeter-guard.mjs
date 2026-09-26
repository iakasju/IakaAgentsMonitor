#!/usr/bin/env node
// perimeter-guard.mjs — Garde iakaframe du canal des GESTES DIRECTS (Edit/Write/Bash/NotebookEdit).
// Cable sur PreToolUse, matcher "Edit|Write|Bash|NotebookEdit".
//
// Pourquoi : delegation-guard.mjs ne garde QUE l'outil Task (delegation). Les gestes mutateurs
// DIRECTS (Edit/Write/Bash/NotebookEdit) ne sont gardes par aucun hook -> faille de perimetre
// (un Edit/commit hors du projet courant passe non controle). Ce garde detecte le(s) chemin(s)
// touche(s), journalise, et selon le mode signale (WARN, exit 0) ou bloque (DENY, exit 2) un
// geste qui SORT du perimetre autorise.
//
// PRINCIPE : garde de CHEMINS, jamais de personas (la persona iakaframe est absente du payload).
// ANCRAGE : $CLAUDE_PROJECT_DIR (stable), PAS le cwd du payload (qui derive). Variable absente
// -> SKIP (fail-open, exit 0).
// FAIL-OPEN partout : tout bug interne => exit 0. Journal : ~/.claude/iakaframe-perimeter.log
//
// ARCHITECTURE (Lot 0, parite multirunner) : ce fichier est l'ADAPTATEUR CLAUDE de la garde de
// perimetre. La LOGIQUE DE DECISION pure (classement d'un chemin absolu contre le perimetre) vit
// dans ./guard-core.mjs (verdictPerimeter/isPerimeterBlocking), partagee avec les autres runners.
// Ici on ne garde que le specifique-Claude : ancrage $CLAUDE_PROJECT_DIR, resolution des chemins
// (cwd/tilde), reperes du foyer ~/.claude, journal, exit code. Comportement STRICTEMENT inchange.

import { appendFileSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { isAbsolute, join, relative, resolve } from "node:path";
import { verdictPerimeter, isPerimeterBlocking } from "./guard-core.mjs";

const LOG = join(homedir(), ".claude", "iakaframe-perimeter.log");
const CLAUDE_DIR = join(homedir(), ".claude");
const HARNESS_SETTINGS = join(CLAUDE_DIR, "settings.json");

const allow = () => process.exit(0);
const ts = () => new Date().toISOString();
const write = (rec) => {
  try { appendFileSync(LOG, JSON.stringify(rec) + "\n", "utf8"); } catch { /* fail-open */ }
};

// Resout le mode EFFECTIF pour un outil donne, a partir de la valeur brute de la variable.
// Renvoie "deny" ou "warn".
const effectiveMode = (modeEnv, tool) => {
  const v = String(modeEnv || "").trim().toLowerCase();
  if (v === "deny") return "deny";
  if (v === "warn") return "warn";
  // default | "" | inconnu => panachage par outil
  return tool === "Bash" ? "warn" : "deny";
};

// Adaptateur Claude : classe un chemin absolu contre le perimetre, en injectant les reperes du
// foyer ~/.claude et l'implementation de path. Delegue le verdict pur a guard-core.
const classifyPath = (absPath, projectDir) => verdictPerimeter(absPath, projectDir, {
  portfolioDir: CLAUDE_DIR,
  harnessSettings: HARNESS_SETTINGS,
  relativeFn: relative,
  isAbsoluteFn: isAbsolute,
});

const isBlocking = (verdict) => isPerimeterBlocking(verdict);

try {
  const raw = readAll();
  if (!raw.trim()) allow();
  const p = JSON.parse(raw);
  const event = p.hook_event_name || "";
  if (event !== "PreToolUse") allow();

  const session = p.session_id || null;
  const tool = p.tool_name || "";
  const ti = p.tool_input || {};
  const payloadCwd = p.cwd || process.cwd();
  const modeEnvRaw = process.env.IAKAFRAME_PERIMETER_MODE;
  const modeEnv = (modeEnvRaw == null || modeEnvRaw === "") ? "default" : modeEnvRaw;

  // Ancrage stable : $CLAUDE_PROJECT_DIR. Absent -> SKIP (fail-open).
  const projectDirRaw = process.env.CLAUDE_PROJECT_DIR;
  if (!projectDirRaw || !projectDirRaw.trim()) {
    write({
      at: ts(), event: "SKIP", session, tool: tool || null, path: null,
      project_dir: null, verdict: "SKIP", reason: "no_project_dir",
      mode: effectiveMode(modeEnv, tool), mode_env: modeEnv,
    });
    allow();
  }
  const projectDir = resolve(projectDirRaw);

  // Resout un chemin (absolu garde tel quel ; relatif resolu contre le cwd du payload).
  const toAbs = (pth) => isAbsolute(pth) ? resolve(pth) : resolve(payloadCwd, pth);

  // ---- Cas Edit / Write / NotebookEdit : chemin explicite FIABLE ----
  if (tool === "Edit" || tool === "Write" || tool === "NotebookEdit") {
    const rawPath = ti.file_path || ti.notebook_path || null;
    const mode = effectiveMode(modeEnv, tool);
    if (!rawPath) {
      // Pas de chemin -> on ne devine pas (fail-open).
      write({
        at: ts(), event: "GESTE", session, tool, path: null,
        project_dir: projectDir, verdict: "SKIP", reason: "no_path",
        mode, mode_env: modeEnv,
      });
      allow();
    }
    const abs = toAbs(rawPath);
    const verdict = classifyPath(abs, projectDir);
    write({
      at: ts(), event: "GESTE", session, tool, path: abs,
      project_dir: projectDir, verdict, mode, mode_env: modeEnv,
    });
    if (isBlocking(verdict)) {
      const why = verdict === "DENY_HARNESS"
        ? "auto-modification du harnais (~/.claude/settings.json) reservee a l'humain"
        : "chemin HORS perimetre projet";
      const msg =
        "[perimeter-guard] Geste " + tool + " " + (mode === "deny" ? "REFUSE" : "HORS PERIMETRE") +
        " : " + why + ".\n  Cible    : " + abs +
        "\n  Perimetre: " + projectDir +
        "\n  Verdict  : " + verdict + " (mode=" + mode + ").\n" +
        (mode === "deny"
          ? "  Passe par la delegation (Aragorn/royaume) plutot qu'un geste direct hors perimetre,\n  ou exporte IAKAFRAME_PERIMETER_MODE=warn pour desamorcer.\n"
          : "  (WARN : geste laisse passer mais journalise.)\n");
      process.stderr.write(msg);
      if (mode === "deny") process.exit(2);
    }
    allow();
  }

  // ---- Cas Bash : heuristique sur la chaine shell (MVP honnete) ----
  if (tool === "Bash") {
    const command = String(ti.command || "");
    const mode = effectiveMode(modeEnv, tool);
    const cmdShort = command.length > 500 ? command.slice(0, 500) + "…" : command;

    const absPaths = extractAbsPaths(command);
    if (absPaths.length === 0) {
      // Aucun chemin absolu identifiable -> on ne devine pas. BASH_UNRESOLVED, toujours exit 0.
      write({
        at: ts(), event: "GESTE", session, tool, path: null, command: cmdShort,
        project_dir: projectDir, verdict: "BASH_UNRESOLVED", mode, mode_env: modeEnv,
      });
      allow();
    }

    // Classer chaque chemin absolu ; un HORS ou DENY_HARNESS suffit a declencher.
    let worst = "ALLOW_PROJECT"; // pire verdict rencontre (priorite DENY_HARNESS > HORS > autres)
    const classified = [];
    for (const ap of absPaths) {
      const v = classifyPath(ap, projectDir);
      classified.push({ path: ap, verdict: v });
      if (v === "DENY_HARNESS") worst = "DENY_HARNESS";
      else if (v === "HORS" && worst !== "DENY_HARNESS") worst = "HORS";
    }

    write({
      at: ts(), event: "GESTE", session, tool, path: classified, command: cmdShort,
      project_dir: projectDir, verdict: worst, mode, mode_env: modeEnv,
    });

    if (isBlocking(worst)) {
      const offenders = classified.filter((c) => isBlocking(c.verdict)).map((c) => c.path);
      const why = worst === "DENY_HARNESS"
        ? "auto-modification du harnais (~/.claude/settings.json) reservee a l'humain"
        : "chemin(s) absolu(s) HORS perimetre projet";
      const msg =
        "[perimeter-guard] Commande Bash " + (mode === "deny" ? "REFUSEE" : "HORS PERIMETRE") +
        " : " + why + ".\n  Chemin(s): " + offenders.join(", ") +
        "\n  Perimetre: " + projectDir +
        "\n  Verdict  : " + worst + " (mode=" + mode + ").\n" +
        (mode === "deny"
          ? "  Passe par la delegation plutot qu'un geste direct hors perimetre,\n  ou exporte IAKAFRAME_PERIMETER_MODE=warn pour desamorcer.\n"
          : "  (WARN : commande laissee passer mais journalisee. Heuristique non exhaustive.)\n");
      process.stderr.write(msg);
      if (mode === "deny") process.exit(2);
    }
    allow();
  }

  // Outil non concerne (matcher large) -> laisser passer.
  allow();
} catch {
  allow(); // fail-open : un bug du garde ne fige jamais une session
}

function readAll() {
  try { return readFileSync(0, "utf8"); }
  catch { return ""; }
}

// Extrait les chemins absolus "interessants" d'une chaine shell.
// MVP honnete : on capte les chemins absolus du foyer ($HOME/..., p.ex. /Users/<user>/...) et ~/.claude/...
// On NE pretend PAS parser le shell ; au moindre doute on n'attrape rien (-> BASH_UNRESOLVED).
function extractAbsPaths(command) {
  const out = new Set();
  // 1) chemins absolus POSIX explicites sous /Users/<user>/...
  //    (on borne au foyer pour eviter le bruit ; suffit a la faille demontree).
  const home = homedir();
  const homeEsc = home.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const reHome = new RegExp(homeEsc + "(?:/[^\\s'\"`;|&><]*)?", "g");
  let m;
  while ((m = reHome.exec(command)) !== null) {
    if (m[0]) out.add(resolve(m[0]));
  }
  // 2) ~/.claude/... (tilde) -> resolu vers le foyer reel.
  const reTilde = /~\/\.claude(?:\/[^\s'"`;|&><]*)?/g;
  while ((m = reTilde.exec(command)) !== null) {
    const tail = m[0].slice(1); // enleve le ~
    out.add(resolve(join(home, tail)));
  }
  return [...out];
}
