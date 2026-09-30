//! Micro-banc de la durée d'un « tick » du watcher (mesure M1 de ce-poste-vue-locale-lisibilite.md) —
//! sans lancer l'app, en lecture SEULE de `~/.claude` et `~/.cache/iaka-agents` :
//!  - ANCIEN tick : `cards_from_dir` (parcours complet de `~/.claude/projects`, cause F4 des pics CPU) ;
//!  - NOUVEAU tick : sessions vivantes (`kill(pid,0)`) + fiches du disque + assemblage D2 + repli
//!    par session (O(sessions vivantes)).
//!
//! Usage : `cargo run --release --example tick_bench` (la release est la mesure représentative ;
//! la durée d'un tick n'est qu'une part du budget CPU, dont la mesure réelle se fait sur le `.app`
//! selon `scripts/measure-cpu.sh`).

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use iakaagentsmonitor_lib::labels::Labels;
use iakaagentsmonitor_lib::roster::Roster;
use iakaagentsmonitor_lib::sources::claude_code_files;
use iakaagentsmonitor_lib::sources::local_sessions::{
    build_local_snapshot, cards_dir, pid_alive, read_disk_cards, read_sessions, sessions_dir, LocalInput, LocalSession,
};

fn percentile(sorted: &[f64], p: f64) -> f64 {
    let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[idx]
}

fn summary(ms: &mut [f64]) -> String {
    ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
    format!(
        "min {:.2} ms · p50 {:.2} ms · p95 {:.2} ms · max {:.2} ms",
        ms[0],
        percentile(ms, 0.5),
        percentile(ms, 0.95),
        ms[ms.len() - 1]
    )
}

fn main() {
    let Some(projects) = claude_code_files::claude_projects_dir() else {
        eprintln!("HOME introuvable");
        return;
    };
    let runs = 12;

    // ---- ANCIEN tick ----
    let mut old = Vec::new();
    let mut old_cards = 0;
    for _ in 0..runs {
        let t = Instant::now();
        old_cards = claude_code_files::cards_from_dir(&projects).len();
        old.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    println!("ANCIEN tick (parcours complet de {}) : {old_cards} fiches ; {runs} passes ; {}", projects.display(), summary(&mut old));

    // ---- NOUVEAU tick ----
    let (Some(sdir), Some(cdir)) = (sessions_dir(), cards_dir()) else {
        eprintln!("répertoires de sessions/fiches indéterminés");
        return;
    };
    let labels = Labels::default();
    let roster = Roster::new();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
    let mut new = Vec::new();
    let mut shown = String::new();
    for _ in 0..(runs * 10) {
        let t = Instant::now();
        let sessions = read_sessions(&sdir, &pid_alive).unwrap_or_default();
        let disk_cards = read_disk_cards(&cdir);
        let mut fallback = |s: &LocalSession| match (s.cwd.as_deref(), s.session_id.as_deref()) {
            (Some(cwd), Some(sid)) => claude_code_files::cards_for_session(&projects, cwd, sid),
            _ => Vec::new(),
        };
        let out = build_local_snapshot(
            LocalInput { sessions: &sessions, disk_cards, bus_cards: Vec::new() },
            &mut fallback,
            now,
            90,
            &labels,
            &roster,
        );
        new.push(t.elapsed().as_secs_f64() * 1000.0);
        if shown.is_empty() {
            shown = format!(
                "{} session(s) vivante(s) [pids {:?}], {} agent(s) affiches, {} non rattache(s), repli par session pour {:?}\n",
                sessions.len(),
                sessions.iter().map(|s| s.pid).collect::<Vec<_>>(),
                out.snapshot.count,
                out.snapshot.unattached.len(),
                out.fallback_for
            );
            for s in &out.snapshot.sessions {
                shown += &format!(
                    "  - {} : coordinateur « {} » ({}), {} sous-agent(s) : {:?}\n",
                    s.project,
                    s.coordinator.task,
                    s.coordinator.state,
                    s.coordinator.children.len(),
                    s.coordinator.children.iter().map(|c| c.persona.as_str()).collect::<Vec<_>>()
                );
            }
        }
    }
    println!("NOUVEAU tick (sessions + fiches + assemblage D2) : {} passes ; {}", runs * 10, summary(&mut new));
    print!("{shown}");
}
