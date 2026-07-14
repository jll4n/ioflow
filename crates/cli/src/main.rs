use std::path::PathBuf;

use chrono::Utc;
use clap::{Parser, Subcommand};
use stu_vcs::{diff_trees, file_label, short, Commit, FileChange, Repo, StuArchive, Tree};

#[derive(Parser)]
#[command(
    name = "ioflow",
    about = "VCS local pour projets Control Expert (.stu)",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialise un dépôt .ioflow/ dans le répertoire courant
    Init,

    /// Crée un snapshot d'un fichier .stu (équivalent de git commit)
    Snapshot {
        /// Fichier .stu à capturer
        stu: PathBuf,

        /// Message décrivant les changements
        #[arg(short, long, default_value = "snapshot")]
        message: String,

        /// Fichier PLCopenXML exporté manuellement depuis Control Expert
        #[arg(long, value_name = "FICHIER.xml")]
        export: Option<PathBuf>,
    },

    /// Affiche l'historique des commits
    Log,

    /// Affiche le détail d'un commit et ses fichiers
    Show {
        /// Hash du commit (les 7 premiers caractères suffisent)
        hash: String,
    },

    /// Affiche les différences entre deux commits
    Diff {
        /// Hash du commit de référence (ancien)
        hash1: String,
        /// Hash du commit à comparer (récent)
        hash2: String,
    },

    /// Recrée un fichier .stu depuis un snapshot
    Restore {
        /// Hash du commit à restaurer
        hash: String,
        /// Fichier .stu de sortie
        #[arg(short, long, value_name = "SORTIE.stu")]
        output: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("erreur : {e}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Init => cmd_init(),
        Commands::Snapshot {
            stu,
            message,
            export,
        } => cmd_snapshot(stu, message, export),
        Commands::Log => cmd_log(),
        Commands::Show { hash } => cmd_show(hash),
        Commands::Diff { hash1, hash2 } => cmd_diff(hash1, hash2),
        Commands::Restore { hash, output } => cmd_restore(hash, output),
    }
}

// ─── init ─────────────────────────────────────────────────────────────────────

fn cmd_init() -> Result<(), Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir()?;
    Repo::init(&cwd)?;
    println!("Dépôt initialisé dans {}", cwd.join(".ioflow").display());
    Ok(())
}

// ─── snapshot ─────────────────────────────────────────────────────────────────

fn cmd_snapshot(
    stu_path: PathBuf,
    message: String,
    export: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir()?;
    let repo = Repo::open(&cwd)?;

    // 1. Extraire et hasher tous les fichiers du STU
    println!("Lecture de {} …", stu_path.display());
    let archive = StuArchive::open(&stu_path)?;
    let mut tree = Tree::new();

    for (name, data) in &archive.files {
        let hash = repo.objects.write(data)?;
        tree.insert(name.clone(), hash);
    }

    // 2. Si un export PLCopenXML est fourni, l'inclure dans le tree
    if let Some(xml_path) = export {
        let data = std::fs::read(&xml_path)?;
        let hash = repo.objects.write(&data)?;
        tree.insert("program.xml".to_string(), hash);
        println!("Export PLCopenXML inclus : {}", xml_path.display());
    }

    // 3. Stocker le tree
    let tree_hash = repo.objects.write(&tree.to_bytes())?;

    // 4. Créer le commit
    let parent = repo.head()?;
    let commit = Commit {
        parent,
        tree: tree_hash,
        message: message.clone(),
        author: repo.author(),
        timestamp: Utc::now(),
    };
    let commit_hash = repo.objects.write(&commit.to_bytes())?;

    // 5. Mettre à jour HEAD
    repo.set_head(&commit_hash)?;

    let n = archive.files.len();
    println!(
        "Snapshot créé : {} — \"{}\" ({} fichier{})",
        short(&commit_hash),
        message,
        n,
        if n > 1 { "s" } else { "" }
    );
    Ok(())
}

// ─── log ──────────────────────────────────────────────────────────────────────

fn cmd_log() -> Result<(), Box<dyn std::error::Error>> {
    let repo = Repo::open(&std::env::current_dir()?)?;
    let mut current = repo.head()?.ok_or(stu_vcs::VcsError::NoCommits)?;

    loop {
        let commit = repo.read_commit(&current)?;
        println!(
            "{} — {} — {} — {}",
            short(&current),
            commit.timestamp.format("%Y-%m-%d %H:%M"),
            commit.author,
            commit.message,
        );
        match commit.parent {
            Some(parent) => current = parent,
            None => break,
        }
    }
    Ok(())
}

// ─── show ─────────────────────────────────────────────────────────────────────

fn cmd_show(prefix: String) -> Result<(), Box<dyn std::error::Error>> {
    let repo = Repo::open(&std::env::current_dir()?)?;
    let hash = resolve_prefix(&repo, &prefix)?;
    let commit = repo.read_commit(&hash)?;
    let tree = repo.read_tree(&commit.tree)?;

    println!("commit  {hash}");
    println!("auteur  {}", commit.author);
    println!(
        "date    {}",
        commit.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
    );
    println!("message {}", commit.message);
    if let Some(p) = &commit.parent {
        println!("parent  {}", short(p));
    }
    println!();
    println!("{} fichier(s) :", tree.files.len());
    for (path, hash) in &tree.files {
        println!("  {} [{}]", path, file_label(path));
        println!("    {}", short(hash));
    }
    Ok(())
}

// ─── diff ─────────────────────────────────────────────────────────────────────

fn cmd_diff(prefix1: String, prefix2: String) -> Result<(), Box<dyn std::error::Error>> {
    let repo = Repo::open(&std::env::current_dir()?)?;
    let hash1 = resolve_prefix(&repo, &prefix1)?;
    let hash2 = resolve_prefix(&repo, &prefix2)?;

    let commit1 = repo.read_commit(&hash1)?;
    let commit2 = repo.read_commit(&hash2)?;
    let tree1 = repo.read_tree(&commit1.tree)?;
    let tree2 = repo.read_tree(&commit2.tree)?;

    println!("Diff {} → {}", short(&hash1), short(&hash2));
    println!(
        "     {} → {}",
        commit1.timestamp.format("%Y-%m-%d %H:%M"),
        commit2.timestamp.format("%Y-%m-%d %H:%M"),
    );
    println!();

    let changes = diff_trees(&tree1, &tree2);

    if !changes
        .iter()
        .any(|c| !matches!(c, FileChange::Unchanged { .. }))
    {
        println!("Aucun changement.");
        return Ok(());
    }

    for change in &changes {
        match change {
            FileChange::Unchanged { .. } => {}
            FileChange::Added { path, .. } => {
                println!("  + {} [{}]", path, file_label(path));
            }
            FileChange::Removed { path, .. } => {
                println!("  - {} [{}]", path, file_label(path));
            }
            FileChange::Modified {
                path,
                old_hash,
                new_hash,
            } => {
                let old_size = repo.objects.read(old_hash).map(|d| d.len()).unwrap_or(0);
                let new_size = repo.objects.read(new_hash).map(|d| d.len()).unwrap_or(0);
                println!(
                    "  ~ {} [{}]  {} → {}",
                    path,
                    file_label(path),
                    human_size(old_size),
                    human_size(new_size),
                );
            }
        }
    }

    let (added, removed, modified_count) = changes.iter().fold((0, 0, 0), |acc, c| match c {
        FileChange::Added { .. } => (acc.0 + 1, acc.1, acc.2),
        FileChange::Removed { .. } => (acc.0, acc.1 + 1, acc.2),
        FileChange::Modified { .. } => (acc.0, acc.1, acc.2 + 1),
        FileChange::Unchanged { .. } => acc,
    });
    println!();
    println!(
        "{} modifié(s), {} ajouté(s), {} supprimé(s)",
        modified_count, added, removed
    );
    Ok(())
}

// ─── restore ──────────────────────────────────────────────────────────────────

fn cmd_restore(prefix: String, output: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let repo = Repo::open(&std::env::current_dir()?)?;
    let hash = resolve_prefix(&repo, &prefix)?;
    let commit = repo.read_commit(&hash)?;
    let tree = repo.read_tree(&commit.tree)?;

    let mut files = std::collections::HashMap::new();
    for (path, blob_hash) in &tree.files {
        if path == "program.xml" {
            continue; // fichier ajouté par ioflow, pas dans le STU d'origine
        }
        let data = repo.objects.read(blob_hash)?;
        files.insert(path.clone(), data);
    }

    StuArchive::write(&files, &output)?;
    println!("Restauré depuis {} → {}", short(&hash), output.display());
    Ok(())
}

// ─── helpers ──────────────────────────────────────────────────────────────────

/// Résout un préfixe de hash (≥ 4 chars) en hash complet.
fn resolve_prefix(repo: &Repo, prefix: &str) -> Result<String, Box<dyn std::error::Error>> {
    if prefix.len() == 64 {
        return Ok(prefix.to_string());
    }
    if prefix.len() < 4 {
        return Err(format!("préfixe trop court (minimum 4 caractères) : {prefix}").into());
    }
    // Parcours de objects/ pour trouver un hash qui commence par prefix
    let objects_dir = repo.ioflow.join("objects");
    let sub = &prefix[..2];
    let rest = &prefix[2..];
    let dir = objects_dir.join(sub);
    if dir.is_dir() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with(rest) {
                return Ok(format!("{sub}{name}"));
            }
        }
    }
    Err(format!("commit introuvable : {prefix}").into())
}

fn human_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
