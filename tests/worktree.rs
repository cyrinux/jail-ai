use jail_ai::worktree::get_required_parent_dirs;
use std::path::{Path, PathBuf};

#[test]
fn test_get_required_parent_dirs() {
    let path1 = Path::new("/home/user/work/feature");
    let path2 = Path::new("/home/user/projects/main/.git");

    let dirs = get_required_parent_dirs(&[path1, path2]);

    assert!(dirs.contains(&PathBuf::from("/home")));
    assert!(dirs.contains(&PathBuf::from("/home/user")));
    assert!(dirs.contains(&PathBuf::from("/home/user/work")));
    assert!(dirs.contains(&PathBuf::from("/home/user/projects")));
    assert!(dirs.contains(&PathBuf::from("/home/user/projects/main")));
}

#[test]
fn test_get_required_parent_dirs_ordering() {
    let path = Path::new("/a/b/c/d");
    let dirs = get_required_parent_dirs(&[path]);

    let a_pos = dirs.iter().position(|p| p == Path::new("/a"));
    let b_pos = dirs.iter().position(|p| p == Path::new("/a/b"));
    let c_pos = dirs.iter().position(|p| p == Path::new("/a/b/c"));

    assert!(a_pos < b_pos);
    assert!(b_pos < c_pos);
}
