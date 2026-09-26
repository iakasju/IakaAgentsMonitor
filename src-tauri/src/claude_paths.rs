//! claude_paths — utilitaires génériques de résolution des chemins Claude Code.
//!
//! ┌────────────────────────────────────────────────────────────────────────────────────────┐
//! │ PROVENANCE : recopié depuis iakaTokenCounter, `iakatc-core/src/measure/claude.rs`,      │
//! │ branche `worktree-agent-a73ae73650ed5e328`, commit `1c9fd0e` (cf.                       │
//! │ specs/instructions/liste-des-agents.md, Q3-A). Fonctions génériques, aucun calcul de    │
//! │ jetons : `claude_projects_dir`, `claude_transcript_files`, `project_of`,                │
//! │ `bucket_project`, `PORTFOLIO_ROOTS`, `OUT_OF_PROJECT_BUCKET`. Comportement et tests      │
//! │ repris à l'identique.                                                                    │
//! └────────────────────────────────────────────────────────────────────────────────────────┘

use std::path::{Path, PathBuf};

/// Racines de portefeuille connues : quand Claude Code tourne à la racine d'un de ces dossiers
/// plutôt que dans un sous-projet, `project_of` renvoie ce nom de racine — pas un vrai projet.
pub const PORTFOLIO_ROOTS: &[&str] = &["work", "Desktop"];

/// Seau où tombent les projets issus d'une racine de portefeuille.
pub const OUT_OF_PROJECT_BUCKET: &str = "hors projet";

/// Dernier segment d'un cwd = nom de projet. Coupe sur `/` ET `\` pour gérer les vieux
/// transcripts Windows. Défensif : chaîne vide -> `None`, séparateurs de fin ignorés.
pub fn project_of(cwd: &str) -> Option<String> {
    cwd.trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .find(|s| !s.is_empty())
        .map(str::to_string)
}

/// Réduit un nom de projet BRUT (sortie de [`project_of`]) à sa forme affichée : identique, sauf
/// s'il s'agit d'une racine de portefeuille connue, auquel cas il tombe dans
/// [`OUT_OF_PROJECT_BUCKET`].
pub fn bucket_project(raw: String) -> String {
    if PORTFOLIO_ROOTS.contains(&raw.as_str()) {
        OUT_OF_PROJECT_BUCKET.to_string()
    } else {
        raw
    }
}

/// Liste RÉCURSIVEMENT tous les `*.jsonl` sous `projects_dir` : transcripts de session
/// principale (`<sid>.jsonl`) ET transcripts de sous-agents délégués
/// (`<sid>/subagents/agent-<id>.jsonl`), sans hypothèse sur la profondeur. Répertoire illisible
/// sauté sans erreur. Ordre de retour NON garanti.
pub fn claude_transcript_files(projects_dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![projects_dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                out.push(path);
            }
        }
    }
    out
}

/// Répertoire des transcripts Claude Code : `<home>/.claude/projects`.
pub fn claude_projects_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(Path::new(&home).join(".claude").join("projects"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn project_of_prend_le_dernier_segment() {
        assert_eq!(
            project_of("/Users/x/work/iaka-demo"),
            Some("iaka-demo".into())
        );
        assert_eq!(project_of("/a/b/"), Some("b".into()));
        assert_eq!(project_of(""), None);
    }

    #[test]
    fn project_of_gere_les_cwd_windows() {
        assert_eq!(project_of(r"C:\iakaVODdash"), Some("iakaVODdash".into()));
        assert_eq!(
            project_of(r"C:\Users\x\work\iaka-demo"),
            Some("iaka-demo".into())
        );
        assert_eq!(project_of(r"/c/work\iaka-demo"), Some("iaka-demo".into()));
        assert_eq!(project_of(r"C:\iakaVODdash\"), Some("iakaVODdash".into()));
        assert_eq!(project_of(r"\a\b\"), Some("b".into()));
    }

    #[test]
    fn bucket_project_regroupe_les_racines_de_portefeuille() {
        assert_eq!(bucket_project("work".into()), OUT_OF_PROJECT_BUCKET);
        assert_eq!(bucket_project("Desktop".into()), OUT_OF_PROJECT_BUCKET);
        assert_eq!(bucket_project("iaka-demo".into()), "iaka-demo");
    }

    fn tmp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("iaka-am-paths-{name}-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn claude_transcript_files_trouve_session_principale_et_sous_agent() {
        let dir = tmp_dir("marche");
        let escaped = dir.join("-w-proj");
        fs::create_dir_all(escaped.join("sid/subagents")).unwrap();
        fs::write(escaped.join("sid.jsonl"), "{}").unwrap();
        fs::write(escaped.join("sid/subagents/agent-1.jsonl"), "{}").unwrap();

        let files = claude_transcript_files(&dir);
        let names: Vec<String> = files
            .iter()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .collect();
        assert!(names.iter().any(|n| n.ends_with("-w-proj/sid.jsonl")));
        assert!(names
            .iter()
            .any(|n| n.ends_with("sid/subagents/agent-1.jsonl")));
    }

    #[test]
    fn claude_transcript_files_dossier_absent_liste_vide() {
        assert!(claude_transcript_files(Path::new("/dossier/inexistant/xyz")).is_empty());
    }

    #[test]
    fn claude_projects_dir_depend_de_home() {
        // Défensif : la fonction ne panique jamais, HOME est quasi toujours présent en CI/dev.
        let _ = claude_projects_dir();
    }
}
