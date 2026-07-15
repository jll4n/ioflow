use std::collections::HashMap;
use std::io::Write as _;
use std::path::Path;

use chrono::Utc;
use tempfile::TempDir;

use stu_vcs::{
    diff_trees, file_label, hash_bytes, short, Commit, FileChange, Repo, StuArchive, Tree,
    VcsError,
};

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Crée un fichier ZIP en mémoire à partir d'une liste (nom, contenu).
fn make_stu_bytes(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::SimpleFileOptions::default();
        for (name, data) in files {
            zip.start_file(*name, opts).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
    }
    buf.into_inner()
}

/// Écrit un STU synthétique sur disque et retourne son chemin.
fn write_stu(dir: &Path, name: &str, files: &[(&str, &[u8])]) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, make_stu_bytes(files)).unwrap();
    path
}

/// Crée un repo + fait un snapshot complet, retourne (repo, commit_hash).
fn snapshot(repo: &Repo, stu_path: &Path, message: &str) -> String {
    let archive = StuArchive::open(stu_path).unwrap();
    let mut tree = Tree::new();
    for (name, data) in &archive.files {
        let h = repo.objects.write(data).unwrap();
        tree.insert(name.clone(), h);
    }
    let tree_hash = repo.objects.write(&tree.to_bytes()).unwrap();
    let parent = repo.head().unwrap();
    let commit = Commit {
        parent,
        tree: tree_hash,
        message: message.to_string(),
        author: "test".to_string(),
        timestamp: Utc::now(),
    };
    let commit_hash = repo.objects.write(&commit.to_bytes()).unwrap();
    repo.set_head(&commit_hash).unwrap();
    commit_hash
}

// ─── Tests : Repo / init ─────────────────────────────────────────────────────

#[test]
fn init_cree_structure_ioflow() {
    let dir = TempDir::new().unwrap();
    Repo::init(dir.path()).unwrap();

    assert!(dir.path().join(".ioflow").is_dir(), "dossier .ioflow manquant");
    assert!(dir.path().join(".ioflow/HEAD").is_file(), "HEAD manquant");
    assert!(dir.path().join(".ioflow/config.toml").is_file(), "config.toml manquant");
    assert!(dir.path().join(".ioflow/refs/heads").is_dir(), "refs/heads manquant");
    assert!(dir.path().join(".ioflow/objects").is_dir(), "objects/ manquant");

    let head = std::fs::read_to_string(dir.path().join(".ioflow/HEAD")).unwrap();
    assert_eq!(head.trim(), "ref: refs/heads/main");
}

#[test]
fn init_echoue_si_deja_initialise() {
    let dir = TempDir::new().unwrap();
    Repo::init(dir.path()).unwrap();
    let result = Repo::init(dir.path());
    assert!(
        matches!(result, Err(VcsError::AlreadyInitialized(_))),
        "devrait retourner AlreadyInitialized"
    );
}

#[test]
fn open_remonte_larborescence() {
    let dir = TempDir::new().unwrap();
    Repo::init(dir.path()).unwrap();

    // On ouvre depuis un sous-dossier inexistant → doit remonter jusqu'à trouver .ioflow/
    let sub = dir.path().join("sous/dossier");
    std::fs::create_dir_all(&sub).unwrap();
    let repo = Repo::open(&sub).unwrap();
    assert_eq!(repo.root, dir.path());
}

#[test]
fn open_echoue_sans_ioflow() {
    let dir = TempDir::new().unwrap();
    let result = Repo::open(dir.path());
    assert!(matches!(result, Err(VcsError::NotARepo)));
}

#[test]
fn head_est_none_avant_premier_commit() {
    let dir = TempDir::new().unwrap();
    let repo = Repo::init(dir.path()).unwrap();
    assert!(repo.head().unwrap().is_none());
}

// ─── Tests : ObjectStore ─────────────────────────────────────────────────────

#[test]
fn write_et_read_round_trip() {
    let dir = TempDir::new().unwrap();
    let repo = Repo::init(dir.path()).unwrap();

    let data = b"contenu de test 12345";
    let hash = repo.objects.write(data).unwrap();

    assert_eq!(hash.len(), 64, "hash SHA-256 doit faire 64 chars hex");
    let lu = repo.objects.read(&hash).unwrap();
    assert_eq!(lu, data);
}

#[test]
fn write_est_idempotent() {
    let dir = TempDir::new().unwrap();
    let repo = Repo::init(dir.path()).unwrap();

    let data = b"donnee dupliquee";
    let h1 = repo.objects.write(data).unwrap();
    let h2 = repo.objects.write(data).unwrap(); // deuxième écriture
    assert_eq!(h1, h2, "même contenu → même hash");

    // Un seul fichier doit exister dans objects/
    let prefix = &h1[..2];
    let suffix = &h1[2..];
    let path = dir.path().join(".ioflow/objects").join(prefix).join(suffix);
    assert!(path.exists());
}

#[test]
fn read_objet_inexistant_retourne_erreur() {
    let dir = TempDir::new().unwrap();
    let repo = Repo::init(dir.path()).unwrap();
    let fake_hash = "a".repeat(64);
    assert!(matches!(
        repo.objects.read(&fake_hash),
        Err(VcsError::ObjectNotFound(_))
    ));
}

// ─── Tests : Hash ────────────────────────────────────────────────────────────

#[test]
fn hash_est_deterministe() {
    let data = b"projet Control Expert v42";
    assert_eq!(hash_bytes(data), hash_bytes(data));
}

#[test]
fn hash_differents_donnent_hash_differents() {
    assert_ne!(hash_bytes(b"aaa"), hash_bytes(b"bbb"));
}

#[test]
fn short_retourne_7_premiers_chars() {
    let h = "abcdef1234567890".repeat(4); // 64 chars
    assert_eq!(short(&h), "abcdef1");
}

// ─── Tests : Tree ────────────────────────────────────────────────────────────

#[test]
fn tree_serialisation_round_trip() {
    let mut tree = Tree::new();
    tree.insert("Project_Settings.xso".to_string(), "abc123".to_string());
    tree.insert("ASPROG.db".to_string(), "def456".to_string());

    let bytes = tree.to_bytes();
    let tree2 = Tree::from_bytes(&bytes).unwrap();

    assert_eq!(tree2.files.len(), 2);
    assert_eq!(tree2.files["Project_Settings.xso"], "abc123");
    assert_eq!(tree2.files["ASPROG.db"], "def456");
}

#[test]
fn tree_ordre_deterministe() {
    // BTreeMap garantit l'ordre alphabétique → le JSON est stable
    let mut t1 = Tree::new();
    t1.insert("z_fichier.db".to_string(), "111".to_string());
    t1.insert("a_fichier.xso".to_string(), "222".to_string());

    let mut t2 = Tree::new();
    t2.insert("a_fichier.xso".to_string(), "222".to_string());
    t2.insert("z_fichier.db".to_string(), "111".to_string());

    // Même contenu inséré dans l'ordre inverse → même bytes JSON
    assert_eq!(t1.to_bytes(), t2.to_bytes());
}

// ─── Tests : Diff ────────────────────────────────────────────────────────────

#[test]
fn diff_detecte_tous_les_types_de_changement() {
    let mut old = Tree::new();
    old.insert("inchange.xso".to_string(), "hash_a".to_string());
    old.insert("modifie.db".to_string(), "hash_b".to_string());
    old.insert("supprime.asm".to_string(), "hash_c".to_string());

    let mut new = Tree::new();
    new.insert("inchange.xso".to_string(), "hash_a".to_string()); // inchangé
    new.insert("modifie.db".to_string(), "hash_b_v2".to_string()); // modifié
    new.insert("ajoute.apx".to_string(), "hash_d".to_string()); // ajouté
    // "supprime.asm" absent → supprimé

    let changes = diff_trees(&old, &new);

    let unchanged: Vec<_> =
        changes.iter().filter(|c| matches!(c, FileChange::Unchanged { .. })).collect();
    let modified: Vec<_> =
        changes.iter().filter(|c| matches!(c, FileChange::Modified { .. })).collect();
    let added: Vec<_> =
        changes.iter().filter(|c| matches!(c, FileChange::Added { .. })).collect();
    let removed: Vec<_> =
        changes.iter().filter(|c| matches!(c, FileChange::Removed { .. })).collect();

    assert_eq!(unchanged.len(), 1, "1 fichier inchangé");
    assert_eq!(modified.len(), 1, "1 fichier modifié");
    assert_eq!(added.len(), 1, "1 fichier ajouté");
    assert_eq!(removed.len(), 1, "1 fichier supprimé");

    assert_eq!(unchanged[0].path(), "inchange.xso");
    assert_eq!(modified[0].path(), "modifie.db");
    assert_eq!(added[0].path(), "ajoute.apx");
    assert_eq!(removed[0].path(), "supprime.asm");
}

#[test]
fn diff_retourne_resultats_tries_par_chemin() {
    let mut old = Tree::new();
    old.insert("z.db".to_string(), "1".to_string());
    old.insert("a.xso".to_string(), "2".to_string());
    old.insert("m.asm".to_string(), "3".to_string());

    let mut new = Tree::new();
    new.insert("z.db".to_string(), "1_v2".to_string());
    new.insert("a.xso".to_string(), "2_v2".to_string());
    new.insert("m.asm".to_string(), "3_v2".to_string());

    let changes = diff_trees(&old, &new);
    let paths: Vec<_> = changes.iter().map(|c| c.path()).collect();
    let mut sorted = paths.clone();
    sorted.sort();
    assert_eq!(paths, sorted, "résultats doivent être triés par chemin");
}

#[test]
fn diff_trees_identiques_donne_uniquement_unchanged() {
    let mut tree = Tree::new();
    tree.insert("f1.xso".to_string(), "h1".to_string());
    tree.insert("f2.db".to_string(), "h2".to_string());

    let changes = diff_trees(&tree, &tree);
    assert!(
        changes.iter().all(|c| matches!(c, FileChange::Unchanged { .. })),
        "tous les fichiers doivent être Unchanged"
    );
}

#[test]
fn file_label_retourne_bonne_etiquette() {
    assert_eq!(file_label("Project_Settings.xso"), "XML paramètres");
    assert_eq!(file_label("Project_Definition.xpdf"), "XML chiffré Schneider");
    assert_eq!(file_label("ASPROG.db"), "base propriétaire eXc");
    assert_eq!(file_label("code_section_001.asm"), "assembleur généré");
    assert_eq!(file_label("Station.apx"), "binaire compilé");
    assert_eq!(file_label("Station.apb"), "binaire compilé");
    assert_eq!(file_label("Station.apd"), "binaire compilé");
    assert_eq!(file_label("FLAGS.CTX"), "contexte binaire");
    assert_eq!(file_label("TypeManager.ODB"), "base objets");
    assert_eq!(file_label("Contact0.bmp"), "image");
    assert_eq!(file_label("inconnu.xyz"), "binaire");
}

// ─── Tests : StuArchive ──────────────────────────────────────────────────────

#[test]
fn stu_open_extrait_tous_les_fichiers() {
    let dir = TempDir::new().unwrap();
    let stu = write_stu(
        dir.path(),
        "test.stu",
        &[
            ("Project_Settings.xso", b"<settings/>"),
            ("ASPROG.db", b"\x65\x58\x63fake"),
            ("BinAppli/Station.apx", b"\x00\x01\x02\x03"),
        ],
    );

    let archive = StuArchive::open(&stu).unwrap();
    assert_eq!(archive.files.len(), 3);
    assert_eq!(archive.files["Project_Settings.xso"], b"<settings/>");
    assert_eq!(archive.files["ASPROG.db"], b"\x65\x58\x63fake");
    assert_eq!(archive.files["BinAppli/Station.apx"], b"\x00\x01\x02\x03");
}

#[test]
fn stu_write_et_open_round_trip() {
    let dir = TempDir::new().unwrap();
    let original: HashMap<String, Vec<u8>> = [
        ("Project_Settings.xso".to_string(), b"<settings/>".to_vec()),
        ("ASPROG.db".to_string(), b"binary_data_here".to_vec()),
    ]
    .into();

    let out = dir.path().join("out.stu");
    StuArchive::write(&original, &out).unwrap();

    let restored = StuArchive::open(&out).unwrap();
    assert_eq!(restored.files.len(), original.len());
    for (name, data) in &original {
        assert_eq!(&restored.files[name], data, "fichier {name} différent");
    }
}

// ─── Tests : Intégration snapshot → diff → restore ───────────────────────────

#[test]
fn snapshot_cree_commit_avec_bon_tree() {
    let dir = TempDir::new().unwrap();
    let repo = Repo::init(dir.path()).unwrap();

    let stu = write_stu(
        dir.path(),
        "v1.stu",
        &[
            ("Project_Settings.xso", b"<settings version='1'/>"),
            ("ASPROG.db", b"db_v1"),
        ],
    );
    let commit_hash = snapshot(&repo, &stu, "version 1");

    // HEAD pointe sur le commit
    assert_eq!(repo.head().unwrap().unwrap(), commit_hash);

    // Le commit contient le bon tree
    let commit = repo.read_commit(&commit_hash).unwrap();
    assert_eq!(commit.message, "version 1");
    assert!(commit.parent.is_none(), "premier commit sans parent");

    let tree = repo.read_tree(&commit.tree).unwrap();
    assert_eq!(tree.files.len(), 2);
    assert!(tree.files.contains_key("Project_Settings.xso"));
    assert!(tree.files.contains_key("ASPROG.db"));
}

#[test]
fn deux_snapshots_chaine_de_commits() {
    let dir = TempDir::new().unwrap();
    let repo = Repo::init(dir.path()).unwrap();

    let stu1 = write_stu(dir.path(), "v1.stu", &[("f.xso", b"v1")]);
    let stu2 = write_stu(dir.path(), "v2.stu", &[("f.xso", b"v2")]);

    let h1 = snapshot(&repo, &stu1, "v1");
    let h2 = snapshot(&repo, &stu2, "v2");

    // v2 a v1 comme parent
    let commit2 = repo.read_commit(&h2).unwrap();
    assert_eq!(commit2.parent.as_deref(), Some(h1.as_str()));

    // v1 n'a pas de parent
    let commit1 = repo.read_commit(&h1).unwrap();
    assert!(commit1.parent.is_none());
}

#[test]
fn blobs_inchanges_sont_dedupliques() {
    let dir = TempDir::new().unwrap();
    let repo = Repo::init(dir.path()).unwrap();

    let contenu_commun = b"fichier_inchange_dans_les_deux_versions";
    let stu1 = write_stu(
        dir.path(),
        "v1.stu",
        &[("commun.xso", contenu_commun), ("specifique.db", b"db_v1")],
    );
    let stu2 = write_stu(
        dir.path(),
        "v2.stu",
        &[("commun.xso", contenu_commun), ("specifique.db", b"db_v2")],
    );

    snapshot(&repo, &stu1, "v1");
    snapshot(&repo, &stu2, "v2");

    // Le blob du fichier commun n'existe qu'une fois dans objects/
    let hash_commun = hash_bytes(contenu_commun);
    let (prefix, suffix) = hash_commun.split_at(2);
    let blob_path = dir.path().join(".ioflow/objects").join(prefix).join(suffix);
    assert!(blob_path.exists(), "blob commun doit exister");

    // Compter les fichiers dans objects/ pour vérifier la déduplication
    let mut count = 0;
    for entry in walkdir(&dir.path().join(".ioflow/objects")) {
        if entry.is_file() {
            count += 1;
        }
    }
    // v1: 2 blobs fichiers + 1 tree + 1 commit = 4
    // v2: 1 nouveau blob (specifique v2) + 1 tree + 1 commit = 3 nouveaux
    // commun.xso est partagé → 7 objets au total (pas 8)
    assert_eq!(count, 7, "le blob commun ne doit pas être dupliqué");
}

#[test]
fn restore_reproduit_fichiers_originaux() {
    let dir = TempDir::new().unwrap();
    let repo = Repo::init(dir.path()).unwrap();

    let original_files: &[(&str, &[u8])] = &[
        ("Project_Settings.xso", b"<settings version='410'/>"),
        ("ASPROG.db", b"\x65\x58\x63\r\nelon\rdb\n!\x00\x05"),
        ("BinAppli/Station.apx", b"\xDE\xAD\xBE\xEF"),
    ];
    let stu = write_stu(dir.path(), "original.stu", original_files);
    let commit_hash = snapshot(&repo, &stu, "v1");

    // Restore depuis le commit
    let commit = repo.read_commit(&commit_hash).unwrap();
    let tree = repo.read_tree(&commit.tree).unwrap();
    let mut files_to_restore = HashMap::new();
    for (path, blob_hash) in &tree.files {
        files_to_restore.insert(path.clone(), repo.objects.read(blob_hash).unwrap());
    }
    let restored_path = dir.path().join("restored.stu");
    StuArchive::write(&files_to_restore, &restored_path).unwrap();

    // Vérifier que chaque fichier est identique
    let restored = StuArchive::open(&restored_path).unwrap();
    assert_eq!(restored.files.len(), original_files.len());
    for (name, expected) in original_files {
        let got = restored.files.get(*name).expect(&format!("{name} manquant"));
        assert_eq!(got.as_slice(), *expected, "contenu de {name} différent après restore");
    }
}

#[test]
fn diff_entre_deux_snapshots() {
    let dir = TempDir::new().unwrap();
    let repo = Repo::init(dir.path()).unwrap();

    let h1 = snapshot(
        &repo,
        &write_stu(
            dir.path(),
            "v1.stu",
            &[
                ("settings.xso", b"<s version='1'/>"),
                ("prog.db", b"db_v1"),
                ("a_supprimer.asm", b"asm_code"),
            ],
        ),
        "v1",
    );
    let h2 = snapshot(
        &repo,
        &write_stu(
            dir.path(),
            "v2.stu",
            &[
                ("settings.xso", b"<s version='1'/>"), // inchangé
                ("prog.db", b"db_v2"),                 // modifié
                ("nouveau.apx", b"bin"),                // ajouté
                // "a_supprimer.asm" absent → supprimé
            ],
        ),
        "v2",
    );

    let c1 = repo.read_commit(&h1).unwrap();
    let c2 = repo.read_commit(&h2).unwrap();
    let t1 = repo.read_tree(&c1.tree).unwrap();
    let t2 = repo.read_tree(&c2.tree).unwrap();
    let changes = diff_trees(&t1, &t2);

    let nb_unchanged = changes.iter().filter(|c| matches!(c, FileChange::Unchanged { .. })).count();
    let nb_modified = changes.iter().filter(|c| matches!(c, FileChange::Modified { .. })).count();
    let nb_added = changes.iter().filter(|c| matches!(c, FileChange::Added { .. })).count();
    let nb_removed = changes.iter().filter(|c| matches!(c, FileChange::Removed { .. })).count();

    assert_eq!(nb_unchanged, 1, "settings.xso inchangé");
    assert_eq!(nb_modified, 1, "prog.db modifié");
    assert_eq!(nb_added, 1, "nouveau.apx ajouté");
    assert_eq!(nb_removed, 1, "a_supprimer.asm supprimé");
}

// ─── Helper récursion fichiers ────────────────────────────────────────────────

fn walkdir(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut result = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                result.extend(walkdir(&path));
            } else {
                result.push(path);
            }
        }
    }
    result
}
