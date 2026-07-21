pub mod commit;
pub mod diff;
pub mod error;
pub mod hash;
pub mod objects;
pub mod repo;
pub mod stu;
pub mod tree;

pub use commit::Commit;
pub use diff::{diff_trees, file_label, is_text_diffable, text_diff, FileChange};
pub use error::VcsError;
pub use hash::{hash_bytes, short, Hash};
pub use repo::Repo;
pub use stu::StuArchive;
pub use tree::Tree;
