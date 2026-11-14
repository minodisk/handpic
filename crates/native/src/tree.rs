//! The folder tree's state, kept pure: which folders are known, listed and
//! expanded, the path spelling its nodes are keyed by, and where its keyboard
//! moves the cursor. A port of the Tauri app's `tree.ts` less the
//! multi-folder selection; the window draws `rows` and fills the tree from
//! `riffle_backend::folders`.

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use riffle_backend::folders::FolderNode;

/// Whether paths compare ignoring case, as the file systems of Windows and
/// macOS do by default.
pub const IGNORE_CASE: bool = cfg!(any(windows, target_os = "macos"));

/// A folder as a listing names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Child {
    pub name: String,
    pub path: String,
    /// A symlink or junction, which `Expand All` lists without descending,
    /// since it can point back at an ancestor.
    pub is_link: bool,
}

impl From<FolderNode> for Child {
    fn from(node: FolderNode) -> Self {
        Self {
            name: node.name,
            path: node.path,
            is_link: node.is_link,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub name: String,
    pub path: String,
    /// `None` until a listing of this folder has answered.
    pub children: Option<Vec<Child>>,
    pub expanded: bool,
    pub raw_count: Option<usize>,
    /// The error of the last listing that failed, until one succeeds.
    pub failed: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tree {
    roots: Vec<Child>,
    nodes: HashMap<String, Node>,
}

/// One drawn row, owned so the window keeps the rows between changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub depth: usize,
    pub path: String,
    pub name: String,
    pub expanded: bool,
    /// Not listed yet, or listed with a drawn child: it gets an expander.
    pub expandable: bool,
    pub raw_count: Option<usize>,
    pub failed: Option<String>,
}

impl Tree {
    pub fn roots(&self) -> impl Iterator<Item = &str> {
        self.roots.iter().map(|root| root.path.as_str())
    }

    pub fn has(&self, path: &str) -> bool {
        self.nodes.contains_key(path)
    }

    pub fn is_expanded(&self, path: &str) -> bool {
        self.nodes.get(path).is_some_and(|node| node.expanded)
    }

    /// The drawn children of the known folder `path`, empty until it is
    /// listed.
    pub fn drawn_children_of(&self, path: &str) -> Vec<Child> {
        self.nodes
            .get(path)
            .and_then(|node| self.drawn_children(node))
            .map(|children| children.into_iter().cloned().collect())
            .unwrap_or_default()
    }

    fn known(&mut self, folders: &[Child]) {
        for Child { name, path, .. } in folders {
            self.nodes.entry(path.clone()).or_insert_with(|| Node {
                name: name.clone(),
                path: path.clone(),
                children: None,
                expanded: false,
                raw_count: None,
                failed: None,
            });
        }
    }

    /// Appends top-level folders; one already at the top level is not
    /// repeated.
    pub fn add_roots(&mut self, roots: impl IntoIterator<Item = Child>) {
        let fresh: Vec<Child> = roots
            .into_iter()
            .filter(|root| !self.roots.iter().any(|r| r.path == root.path))
            .collect();
        self.known(&fresh);
        self.roots.extend(fresh);
    }

    pub fn expand(&mut self, path: &str) {
        if let Some(node) = self.nodes.get_mut(path) {
            node.expanded = true;
        }
    }

    pub fn collapse(&mut self, path: &str) {
        if let Some(node) = self.nodes.get_mut(path) {
            node.expanded = false;
        }
    }

    pub fn mark_failed(&mut self, path: &str, error: String) {
        if let Some(node) = self.nodes.get_mut(path) {
            node.failed = Some(error);
        }
    }

    /// A child already known keeps its own state, so re-listing a folder
    /// does not collapse the folders open under it.
    pub fn set_children(&mut self, path: &str, raw_count: usize, children: Vec<Child>) {
        if !self.nodes.contains_key(path) {
            return;
        }
        self.known(&children);
        if let Some(node) = self.nodes.get_mut(path) {
            node.children = Some(children);
            node.raw_count = Some(raw_count);
            node.failed = None;
        }
    }

    /// The children of `node` the tree draws, or `None` until it is listed.
    /// A folder that is itself a root is drawn as that root only, so home is
    /// not repeated under the volume it lives on.
    pub fn drawn_children<'a>(&'a self, node: &'a Node) -> Option<Vec<&'a Child>> {
        let children = node.children.as_ref()?;
        let roots: Vec<String> = self.roots.iter().map(|r| normalize(&r.path)).collect();
        Some(
            children
                .iter()
                .filter(|child| !roots.contains(&normalize(&child.path)))
                .collect(),
        )
    }

    /// The rows to draw, depth first: the roots, and the drawn children of
    /// every expanded folder that has been listed.
    pub fn rows(&self) -> Vec<Row> {
        let mut out = Vec::new();
        let roots: Vec<&Child> = self.roots.iter().collect();
        self.walk(&roots, 0, &mut out);
        out
    }

    fn walk(&self, folders: &[&Child], depth: usize, out: &mut Vec<Row>) {
        for Child { path, .. } in folders {
            let Some(node) = self.nodes.get(path) else {
                continue;
            };
            let children = self.drawn_children(node);
            out.push(Row {
                depth,
                path: node.path.clone(),
                name: node.name.clone(),
                expanded: node.expanded,
                expandable: children.as_ref().is_none_or(|c| !c.is_empty()),
                raw_count: node.raw_count,
                failed: node.failed.clone(),
            });
            if node.expanded
                && let Some(children) = children
            {
                self.walk(&children, depth + 1, out);
            }
        }
    }

    /// The folders the tree watches: the expanded ones among the drawn rows,
    /// sorted. A node left expanded under a collapsed parent is not drawn, so
    /// it is not watched.
    pub fn watched_folders(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .rows()
            .into_iter()
            .filter(|row| row.expanded)
            .map(|row| row.path)
            .collect();
        out.sort();
        out
    }

    /// The listed folders under `path`, depth first, through the children
    /// the tree draws: a folder not yet listed ends its branch, and a root
    /// drawn elsewhere is left to its own row. Empty for an unknown path.
    pub fn descendants(&self, path: &str) -> Vec<String> {
        let mut out = Vec::new();
        self.descend(path, &mut out);
        out
    }

    fn descend(&self, dir: &str, out: &mut Vec<String>) {
        let Some(node) = self.nodes.get(dir) else {
            return;
        };
        for child in self.drawn_children(node).unwrap_or_default() {
            if self.nodes.contains_key(&child.path) {
                out.push(child.path.clone());
                self.descend(&child.path, out);
            }
        }
    }

    /// Collapses every folder under `path`, leaving `path` itself as it is.
    pub fn collapse_all(&mut self, path: &str) {
        for dir in self.descendants(path) {
            self.collapse(&dir);
        }
    }

    /// The folder `old_path` was renamed to `new_name` at `new_path`: it and
    /// every node under it are re-keyed with their state (expansion,
    /// listing, count) kept, and its entry in its parent's children is
    /// replaced and re-sorted case-insensitively, as a listing sorts. A stale
    /// node already under `new_path` is dropped. `tree.ts`'s `renameFolder`.
    pub fn rename_folder(
        &mut self,
        old_path: &str,
        new_path: &str,
        new_name: &str,
        ignore_case: bool,
    ) {
        let renamed = fold(old_path, ignore_case);
        let stale = |path: &str| {
            rebase(path, old_path, new_path, ignore_case).is_none()
                && rebase(path, new_path, new_path, ignore_case).is_some()
        };
        let moved = |name: &str, path: &str| match rebase(path, old_path, new_path, ignore_case) {
            Some(rebased) if fold(path, ignore_case) == renamed => (new_name.to_owned(), rebased),
            Some(rebased) => (name.to_owned(), rebased),
            None => (name.to_owned(), path.to_owned()),
        };
        let child = |c: &Child| {
            let (name, path) = moved(&c.name, &c.path);
            Child {
                name,
                path,
                is_link: c.is_link,
            }
        };
        let list = |children: &[Child]| {
            let mut out: Vec<Child> = children
                .iter()
                .filter(|c| !stale(&c.path))
                .map(child)
                .collect();
            if children
                .iter()
                .any(|c| fold(&c.path, ignore_case) == renamed)
            {
                out.sort_by_cached_key(|c| c.name.to_lowercase());
            }
            out
        };
        self.nodes = std::mem::take(&mut self.nodes)
            .into_values()
            .filter(|node| !stale(&node.path))
            .map(|node| {
                let (name, path) = moved(&node.name, &node.path);
                let children = node.children.as_deref().map(list);
                (
                    path.clone(),
                    Node {
                        name,
                        path,
                        children,
                        ..node
                    },
                )
            })
            .collect();
        self.roots = self.roots.iter().map(child).collect();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepKey {
    Up,
    Down,
    Home,
    End,
}

/// The path of the row the keyboard cursor moves to, or `None` when there is
/// no row. `Up` / `Down` stop at the ends; a cursor not among `rows` (none
/// yet, or its parent collapsed) lands on the first row.
pub fn step(rows: &[Row], cursor: Option<&str>, key: StepKey) -> Option<String> {
    let last = rows.len().checked_sub(1)?;
    let at = rows
        .iter()
        .position(|row| Some(row.path.as_str()) == cursor);
    let to = match (key, at) {
        (StepKey::Home, _) => 0,
        (StepKey::End, _) => last,
        (_, None) => 0,
        (StepKey::Up, Some(at)) => at.saturating_sub(1),
        (StepKey::Down, Some(at)) => (at + 1).min(last),
    };
    Some(rows[to].path.clone())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeKey {
    Left,
    Right,
    Enter,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeCommand {
    Focus(String),
    Expand(String),
    Collapse(String),
    Open(String),
}

/// What `Right` / `Left` / `Enter` do on the cursor row, following the
/// WAI-ARIA tree pattern, or `None` when the key does nothing there (or the
/// cursor is not among the drawn rows).
pub fn tree_key(rows: &[Row], cursor: Option<&str>, key: TreeKey) -> Option<TreeCommand> {
    let at = rows
        .iter()
        .position(|row| Some(row.path.as_str()) == cursor)?;
    let row = &rows[at];
    match key {
        TreeKey::Enter => Some(TreeCommand::Open(row.path.clone())),
        TreeKey::Right => {
            if !row.expandable {
                None
            } else if !row.expanded {
                Some(TreeCommand::Expand(row.path.clone()))
            } else {
                rows.get(at + 1)
                    .filter(|child| child.depth > row.depth)
                    .map(|child| TreeCommand::Focus(child.path.clone()))
            }
        }
        TreeKey::Left => {
            if row.expanded && row.expandable {
                return Some(TreeCommand::Collapse(row.path.clone()));
            }
            rows[..at]
                .iter()
                .rev()
                .find(|parent| parent.depth < row.depth)
                .map(|parent| TreeCommand::Focus(parent.path.clone()))
        }
    }
}

/// The type-ahead buffer: the text typed so far and when its last key came.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Typed {
    pub text: String,
    pub at: Option<Instant>,
}

pub const TYPE_AHEAD_TIMEOUT: Duration = Duration::from_millis(500);

/// The type-ahead buffer after typing `typed` at `now`: appended while the
/// keys come within `timeout` of each other, else started over.
pub fn append_typed(buffer: &Typed, typed: &str, now: Instant, timeout: Duration) -> Typed {
    let within = buffer
        .at
        .is_some_and(|at| now.saturating_duration_since(at) <= timeout);
    Typed {
        text: if within {
            format!("{}{typed}", buffer.text)
        } else {
            typed.to_owned()
        },
        at: Some(now),
    }
}

/// The path of the row whose name starts with `prefix` (case-insensitive),
/// searching down from the cursor and wrapping to the top, or `None` when
/// none does. A single character (or one repeated, as a tapped letter gives)
/// starts past the cursor, so tapping cycles through the matches; a longer
/// prefix starts at the cursor, so it keeps the row its first letter found.
pub fn type_ahead(rows: &[Row], cursor: Option<&str>, prefix: &str) -> Option<String> {
    let mut needle = prefix.to_lowercase();
    let first = needle.chars().next()?;
    if rows.is_empty() {
        return None;
    }
    let repeated = needle.chars().all(|c| c == first);
    if repeated {
        needle = first.to_string();
    }
    let at = rows
        .iter()
        .position(|row| Some(row.path.as_str()) == cursor);
    let start = match at {
        None => 0,
        Some(at) if repeated => at + 1,
        Some(at) => at,
    };
    (0..rows.len())
        .map(|i| &rows[(start + i) % rows.len()])
        .find(|row| row.name.to_lowercase().starts_with(&needle))
        .map(|row| row.path.clone())
}

fn is_windows_path(path: &str) -> bool {
    let b = path.as_bytes();
    (b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':') || path.starts_with("\\\\")
}

fn segments(path: &str) -> Vec<&str> {
    path.split(['\\', '/']).filter(|s| !s.is_empty()).collect()
}

/// Forward slashes, no trailing separator, and a lower-case drive letter, so
/// `C:\Users` and `c:/Users/` compare equal. `/` becomes the empty string.
fn normalize(path: &str) -> String {
    let mut slashed = path.replace('\\', "/");
    while slashed.ends_with('/') {
        slashed.pop();
    }
    let b = slashed.as_bytes();
    if b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
        slashed[..1].make_ascii_lowercase();
    }
    slashed
}

fn fold(path: &str, ignore_case: bool) -> String {
    if ignore_case {
        normalize(path).to_lowercase()
    } else {
        normalize(path)
    }
}

fn join(parent: &str, name: &str) -> String {
    if parent.ends_with(['/', '\\']) {
        format!("{parent}{name}")
    } else if is_windows_path(parent) {
        format!("{parent}\\{name}")
    } else {
        format!("{parent}/{name}")
    }
}

/// The paths from the root holding `path` down to `path` itself, spelled the
/// way a listing spells its children (the root's own spelling, then one
/// separator per level), or `None` when no root holds it. The deepest root
/// wins, so a folder under home is reached through home rather than through
/// the volume home lives on. With `ignore_case` a root holds the path
/// whatever the case of either; the chain still takes the caller's segments,
/// which `respell` corrects from the listings.
pub fn ancestors_within<'a>(
    roots: impl IntoIterator<Item = &'a str>,
    path: &str,
    ignore_case: bool,
) -> Option<Vec<String>> {
    let target = fold(path, ignore_case);
    let mut best: Option<(&str, usize)> = None;
    for root in roots {
        let prefix = fold(root, ignore_case);
        let holds = target == prefix || target.starts_with(&format!("{prefix}/"));
        if holds && best.is_none_or(|(_, len)| prefix.len() > len) {
            best = Some((root, prefix.len()));
        }
    }
    let (best, _) = best?;
    let mut chain = vec![best.to_owned()];
    for name in segments(path).into_iter().skip(segments(best).len()) {
        let next = join(chain.last()?, name);
        chain.push(next);
    }
    Some(chain)
}

/// `chain` with `chain[at]` and everything after it rebased onto the listed
/// child that is `chain[at]` (ignoring case when asked), so the rest of the
/// chain is spelled the way the tree keys its nodes; unchanged when no child
/// matches.
pub fn respell(
    chain: Vec<String>,
    at: usize,
    children: &[Child],
    ignore_case: bool,
) -> Vec<String> {
    let Some(dir) = chain.get(at).cloned() else {
        return chain;
    };
    let Some(child) = children
        .iter()
        .find(|c| fold(&c.path, ignore_case) == fold(&dir, ignore_case))
    else {
        return chain;
    };
    chain
        .into_iter()
        .enumerate()
        .map(|(i, p)| {
            if i < at {
                p
            } else {
                rebase(&p, &dir, &child.path, ignore_case).unwrap_or(p)
            }
        })
        .collect()
}

/// The root to add for a folder no root holds: its drive (`D:\`), its UNC
/// share (`\\server\share`), or `/`.
pub fn root_of(path: &str) -> String {
    let b = path.as_bytes();
    if b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
        return format!("{}\\", &path[..2]);
    }
    if path.starts_with("\\\\") {
        return format!(
            "\\\\{}",
            segments(path)
                .into_iter()
                .take(2)
                .collect::<Vec<_>>()
                .join("\\")
        );
    }
    "/".to_owned()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Relation {
    Same,
    Under,
}

/// Whether `path` is `dir` itself or a folder under it, compared the way
/// `ancestors_within` compares.
pub fn relation(path: &str, dir: &str, ignore_case: bool) -> Option<Relation> {
    let target = fold(path, ignore_case);
    let prefix = fold(dir, ignore_case);
    if target == prefix {
        Some(Relation::Same)
    } else if target.starts_with(&format!("{prefix}/")) {
        Some(Relation::Under)
    } else {
        None
    }
}

/// The path under `new_dir` that `path` had under `old_dir`, or `None` when
/// `path` is neither `old_dir` nor under it; the rest of `path` is joined
/// onto `new_dir` in `path`'s own spelling.
pub fn rebase(path: &str, old_dir: &str, new_dir: &str, ignore_case: bool) -> Option<String> {
    let target = fold(path, ignore_case);
    let prefix = fold(old_dir, ignore_case);
    if target == prefix {
        return Some(new_dir.to_owned());
    }
    if !target.starts_with(&format!("{prefix}/")) {
        return None;
    }
    let mut rebased = new_dir.to_owned();
    for name in segments(path).into_iter().skip(segments(old_dir).len()) {
        rebased = join(&rebased, name);
    }
    Some(rebased)
}

/// Ported from `tree.test.ts`. Left out: the multi-folder selection
/// (`clickSelect`, `pruneSelection`).
#[cfg(test)]
mod tests {
    use super::*;

    fn child(name: &str, path: &str) -> Child {
        Child {
            name: name.into(),
            path: path.into(),
            is_link: false,
        }
    }

    fn home() -> Child {
        child("me", "/home/me")
    }

    fn card() -> Child {
        child("card", "/media/me/card")
    }

    fn with_roots(roots: &[Child]) -> Tree {
        let mut tree = Tree::default();
        tree.add_roots(roots.iter().cloned());
        tree
    }

    fn drawn(tree: &Tree) -> Vec<String> {
        tree.rows()
            .iter()
            .map(|row| format!("{}:{}", row.depth, row.name))
            .collect()
    }

    fn strings(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|p| (*p).to_owned()).collect()
    }

    #[test]
    fn roots_start_unlisted_and_collapsed_and_are_not_repeated() {
        let mut tree = with_roots(&[home(), card()]);
        tree.add_roots([card()]);
        assert_eq!(tree.roots, vec![home(), card()]);
        assert_eq!(
            tree.nodes.get("/home/me"),
            Some(&Node {
                name: "me".into(),
                path: "/home/me".into(),
                children: None,
                expanded: false,
                raw_count: None,
                failed: None,
            })
        );
    }

    #[test]
    fn a_listed_child_keeps_whether_it_is_a_link() {
        let node = FolderNode {
            name: "l".into(),
            path: "/l".into(),
            is_link: true,
        };
        assert!(Child::from(node).is_link);
    }

    #[test]
    fn an_expanded_folder_shows_its_children_once_listed() {
        let mut tree = with_roots(&[home(), card()]);
        tree.expand("/home/me");
        assert_eq!(drawn(&tree), ["0:me", "0:card"]);
        tree.set_children("/home/me", 3, vec![child("Pictures", "/home/me/Pictures")]);
        assert_eq!(drawn(&tree), ["0:me", "1:Pictures", "0:card"]);
        assert_eq!(tree.nodes["/home/me"].raw_count, Some(3));
    }

    #[test]
    fn collapsing_keeps_the_children_for_the_next_expand() {
        let mut tree = with_roots(&[home()]);
        tree.expand("/home/me");
        tree.set_children("/home/me", 0, vec![child("a", "/home/me/a")]);
        tree.collapse("/home/me");
        assert_eq!(drawn(&tree), ["0:me"]);
        tree.expand("/home/me");
        assert_eq!(drawn(&tree), ["0:me", "1:a"]);
    }

    #[test]
    fn a_failed_listing_marks_the_folder_until_a_later_listing_succeeds() {
        let mut tree = with_roots(&[home()]);
        tree.mark_failed("/home/me", "access denied".into());
        assert_eq!(
            tree.nodes["/home/me"].failed.as_deref(),
            Some("access denied")
        );
        assert_eq!(tree.rows()[0].failed.as_deref(), Some("access denied"));
        tree.expand("/home/me");
        tree.set_children("/home/me", 0, vec![]);
        assert_eq!(tree.nodes["/home/me"].failed, None);
    }

    #[test]
    fn re_listing_keeps_the_state_of_a_child_already_known() {
        let mut tree = with_roots(&[home()]);
        tree.expand("/home/me");
        tree.set_children("/home/me", 0, vec![child("a", "/home/me/a")]);
        tree.expand("/home/me/a");
        tree.set_children("/home/me/a", 0, vec![child("b", "/home/me/a/b")]);
        tree.set_children(
            "/home/me",
            0,
            vec![child("a", "/home/me/a"), child("new", "/home/me/new")],
        );
        assert_eq!(drawn(&tree), ["0:me", "1:a", "2:b", "1:new"]);
    }

    #[test]
    fn changes_to_an_unknown_path_leave_the_tree_as_is() {
        let tree = with_roots(&[home()]);
        let mut changed = tree.clone();
        changed.expand("/nope");
        changed.collapse("/nope");
        changed.mark_failed("/nope", "x".into());
        changed.set_children("/nope", 1, vec![child("x", "/nope/x")]);
        assert_eq!(changed, tree);
    }

    #[test]
    fn a_row_is_expandable_until_listed_empty() {
        let mut tree = with_roots(&[home()]);
        assert!(tree.rows()[0].expandable);
        tree.set_children("/home/me", 0, vec![child("a", "/home/me/a")]);
        assert!(tree.rows()[0].expandable);
        tree.set_children("/home/me", 0, vec![]);
        assert!(!tree.rows()[0].expandable);
    }

    #[test]
    fn a_collapsed_tree_watches_nothing() {
        assert!(with_roots(&[home(), card()]).watched_folders().is_empty());
    }

    #[test]
    fn an_expanded_root_is_watched() {
        let mut tree = with_roots(&[home(), card()]);
        tree.expand("/home/me");
        assert_eq!(tree.watched_folders(), ["/home/me"]);
    }

    #[test]
    fn an_expanded_child_under_an_expanded_parent_is_watched() {
        let mut tree = with_roots(&[home()]);
        tree.expand("/home/me");
        tree.set_children("/home/me", 0, vec![child("a", "/home/me/a")]);
        tree.expand("/home/me/a");
        assert_eq!(tree.watched_folders(), ["/home/me", "/home/me/a"]);
    }

    #[test]
    fn an_expanded_child_under_a_collapsed_parent_is_not_watched() {
        let mut tree = with_roots(&[home()]);
        tree.expand("/home/me");
        tree.set_children("/home/me", 0, vec![child("a", "/home/me/a")]);
        tree.expand("/home/me/a");
        tree.collapse("/home/me");
        assert!(tree.watched_folders().is_empty());
    }

    #[test]
    fn an_expanded_folder_not_listed_yet_is_watched() {
        let mut tree = with_roots(&[home()]);
        tree.expand("/home/me");
        assert_eq!(tree.nodes["/home/me"].children, None);
        assert_eq!(tree.watched_folders(), ["/home/me"]);
    }

    #[test]
    fn the_watched_paths_are_sorted_whatever_the_drawing_order() {
        let mut tree = with_roots(&[card(), home()]);
        tree.expand("/media/me/card");
        tree.expand("/home/me");
        assert_eq!(tree.watched_folders(), ["/home/me", "/media/me/card"]);
    }

    /// me > (a > b > c, d), every one listed and expanded.
    fn nested() -> Tree {
        let mut tree = with_roots(&[home()]);
        tree.expand("/home/me");
        tree.set_children(
            "/home/me",
            0,
            vec![child("a", "/home/me/a"), child("d", "/home/me/d")],
        );
        tree.expand("/home/me/a");
        tree.set_children("/home/me/a", 0, vec![child("b", "/home/me/a/b")]);
        tree.expand("/home/me/a/b");
        tree.set_children("/home/me/a/b", 0, vec![child("c", "/home/me/a/b/c")]);
        tree.expand("/home/me/a/b/c");
        tree.set_children("/home/me/a/b/c", 0, vec![]);
        tree.expand("/home/me/d");
        tree
    }

    #[test]
    fn descendants_walk_the_listed_folders_under_a_path_depth_first() {
        assert_eq!(
            nested().descendants("/home/me"),
            ["/home/me/a", "/home/me/a/b", "/home/me/a/b/c", "/home/me/d"]
        );
    }

    #[test]
    fn collapse_all_collapses_every_nested_folder_and_keeps_the_clicked_one_expanded() {
        let mut tree = nested();
        tree.collapse_all("/home/me");
        assert!(tree.is_expanded("/home/me"));
        for path in ["/home/me/a", "/home/me/a/b", "/home/me/a/b/c", "/home/me/d"] {
            assert!(!tree.is_expanded(path));
        }
        assert_eq!(drawn(&tree), ["0:me", "1:a", "1:d"]);
    }

    #[test]
    fn collapse_all_leaves_the_folders_outside_the_clicked_one_as_they_are() {
        let mut tree = nested();
        tree.collapse_all("/home/me/a");
        assert!(tree.is_expanded("/home/me/a"));
        assert!(!tree.is_expanded("/home/me/a/b"));
        assert!(tree.is_expanded("/home/me/d"));
    }

    #[test]
    fn collapse_all_of_an_unknown_path_leaves_the_tree_as_is() {
        let tree = nested();
        assert!(tree.descendants("/nope").is_empty());
        let mut changed = tree.clone();
        changed.collapse_all("/nope");
        assert_eq!(changed, tree);
    }

    #[test]
    fn collapse_all_skips_a_folder_not_yet_listed() {
        let mut tree = with_roots(&[home()]);
        tree.expand("/home/me");
        tree.set_children("/home/me", 0, vec![child("a", "/home/me/a")]);
        tree.expand("/home/me/a");
        assert!(tree.descendants("/home/me/a").is_empty());
        let mut changed = tree.clone();
        changed.collapse_all("/home/me/a");
        assert_eq!(changed, tree);
        assert_eq!(tree.descendants("/home/me"), ["/home/me/a"]);
    }

    #[test]
    fn drawn_children_of_a_folder_keep_whether_they_are_links() {
        let mut tree = with_roots(&[home()]);
        assert!(tree.drawn_children_of("/home/me").is_empty());
        let link = Child {
            is_link: true,
            ..child("l", "/home/me/l")
        };
        tree.set_children("/home/me", 0, vec![child("a", "/home/me/a"), link.clone()]);
        assert_eq!(
            tree.drawn_children_of("/home/me"),
            vec![child("a", "/home/me/a"), link]
        );
        assert!(tree.drawn_children_of("/nope").is_empty());
    }

    /// me > (Pictures > 2026), card
    fn stepped() -> Tree {
        let mut tree = with_roots(&[home(), card()]);
        tree.expand("/home/me");
        tree.set_children("/home/me", 0, vec![child("Pictures", "/home/me/Pictures")]);
        tree.expand("/home/me/Pictures");
        tree.set_children(
            "/home/me/Pictures",
            0,
            vec![child("2026", "/home/me/Pictures/2026")],
        );
        tree
    }

    #[test]
    fn step_down_and_up_walk_the_visible_rows_across_depth() {
        let rows = stepped().rows();
        let to = |cursor, key| step(&rows, Some(cursor), key);
        assert_eq!(
            to("/home/me", StepKey::Down).as_deref(),
            Some("/home/me/Pictures")
        );
        assert_eq!(
            to("/home/me/Pictures", StepKey::Down).as_deref(),
            Some("/home/me/Pictures/2026")
        );
        assert_eq!(
            to("/home/me/Pictures/2026", StepKey::Down).as_deref(),
            Some("/media/me/card")
        );
        assert_eq!(
            to("/media/me/card", StepKey::Up).as_deref(),
            Some("/home/me/Pictures/2026")
        );
        assert_eq!(
            to("/home/me/Pictures", StepKey::Up).as_deref(),
            Some("/home/me")
        );
    }

    #[test]
    fn step_down_and_up_between_siblings() {
        let rows = with_roots(&[home(), card()]).rows();
        assert_eq!(
            step(&rows, Some("/home/me"), StepKey::Down).as_deref(),
            Some("/media/me/card")
        );
        assert_eq!(
            step(&rows, Some("/media/me/card"), StepKey::Up).as_deref(),
            Some("/home/me")
        );
    }

    #[test]
    fn step_clamps_at_both_ends() {
        let rows = stepped().rows();
        assert_eq!(
            step(&rows, Some("/home/me"), StepKey::Up).as_deref(),
            Some("/home/me")
        );
        assert_eq!(
            step(&rows, Some("/media/me/card"), StepKey::Down).as_deref(),
            Some("/media/me/card")
        );
    }

    #[test]
    fn step_home_and_end_are_the_first_and_last_rows() {
        let rows = stepped().rows();
        assert_eq!(
            step(&rows, Some("/home/me/Pictures"), StepKey::Home).as_deref(),
            Some("/home/me")
        );
        assert_eq!(
            step(&rows, Some("/home/me/Pictures"), StepKey::End).as_deref(),
            Some("/media/me/card")
        );
        assert_eq!(
            step(&rows, None, StepKey::End).as_deref(),
            Some("/media/me/card")
        );
    }

    #[test]
    fn step_from_a_missing_or_stale_cursor_lands_on_the_first_row() {
        let mut tree = stepped();
        tree.collapse("/home/me");
        let rows = tree.rows();
        for key in [StepKey::Up, StepKey::Down] {
            assert_eq!(step(&rows, None, key).as_deref(), Some("/home/me"));
            assert_eq!(
                step(&rows, Some("/home/me/Pictures/2026"), key).as_deref(),
                Some("/home/me")
            );
        }
    }

    #[test]
    fn step_in_an_empty_tree_has_no_row() {
        for key in [StepKey::Up, StepKey::Down, StepKey::Home, StepKey::End] {
            assert_eq!(step(&[], None, key), None);
            assert_eq!(step(&[], Some("/home/me"), key), None);
        }
    }

    const PICTURES: &str = "/home/me/Pictures";
    const EMPTY: &str = "/home/me/empty";
    const Y2026: &str = "/home/me/Pictures/2026";
    const HOME: &str = "/home/me";
    const CARD: &str = "/media/me/card";

    /// me > (Pictures > 2026, empty), card
    fn keyed() -> Tree {
        let mut tree = with_roots(&[home(), card()]);
        tree.expand(HOME);
        tree.set_children(
            HOME,
            0,
            vec![child("Pictures", PICTURES), child("empty", EMPTY)],
        );
        tree.set_children(EMPTY, 0, vec![]);
        tree.expand(PICTURES);
        tree.set_children(PICTURES, 0, vec![child("2026", Y2026)]);
        tree
    }

    fn key(tree: &Tree, cursor: Option<&str>, key: TreeKey) -> Option<TreeCommand> {
        tree_key(&tree.rows(), cursor, key)
    }

    fn collapsed(mut tree: Tree, path: &str) -> Tree {
        tree.collapse(path);
        tree
    }

    fn expanded(mut tree: Tree, path: &str) -> Tree {
        tree.expand(path);
        tree
    }

    #[test]
    fn right_expands_a_collapsed_row_that_can_expand_listed_or_not() {
        assert_eq!(
            key(&collapsed(keyed(), HOME), Some(HOME), TreeKey::Right),
            Some(TreeCommand::Expand(HOME.into()))
        );
        assert_eq!(
            key(&keyed(), Some(CARD), TreeKey::Right),
            Some(TreeCommand::Expand(CARD.into()))
        );
    }

    #[test]
    fn right_on_an_expanded_row_enters_its_first_child() {
        assert_eq!(
            key(&keyed(), Some(HOME), TreeKey::Right),
            Some(TreeCommand::Focus(PICTURES.into()))
        );
    }

    #[test]
    fn right_does_nothing_on_a_leaf_or_an_expanded_row_still_listing() {
        assert_eq!(key(&keyed(), Some(EMPTY), TreeKey::Right), None);
        assert_eq!(
            key(&expanded(keyed(), CARD), Some(CARD), TreeKey::Right),
            None
        );
    }

    #[test]
    fn left_collapses_an_expanded_row_even_one_still_listing() {
        assert_eq!(
            key(&keyed(), Some(PICTURES), TreeKey::Left),
            Some(TreeCommand::Collapse(PICTURES.into()))
        );
        assert_eq!(
            key(&expanded(keyed(), CARD), Some(CARD), TreeKey::Left),
            Some(TreeCommand::Collapse(CARD.into()))
        );
    }

    #[test]
    fn left_on_a_collapsed_row_or_a_leaf_goes_to_the_parent() {
        assert_eq!(
            key(&keyed(), Some(Y2026), TreeKey::Left),
            Some(TreeCommand::Focus(PICTURES.into()))
        );
        assert_eq!(
            key(&keyed(), Some(EMPTY), TreeKey::Left),
            Some(TreeCommand::Focus(HOME.into()))
        );
        assert_eq!(
            key(&collapsed(keyed(), PICTURES), Some(PICTURES), TreeKey::Left),
            Some(TreeCommand::Focus(HOME.into()))
        );
    }

    #[test]
    fn left_on_an_expanded_leaf_goes_to_the_parent() {
        assert_eq!(
            key(&expanded(keyed(), EMPTY), Some(EMPTY), TreeKey::Left),
            Some(TreeCommand::Focus(HOME.into()))
        );
    }

    #[test]
    fn left_does_nothing_on_a_collapsed_root() {
        assert_eq!(key(&keyed(), Some(CARD), TreeKey::Left), None);
        assert_eq!(
            key(&collapsed(keyed(), HOME), Some(HOME), TreeKey::Left),
            None
        );
    }

    #[test]
    fn enter_opens_the_cursor_row() {
        assert_eq!(
            key(&keyed(), Some(Y2026), TreeKey::Enter),
            Some(TreeCommand::Open(Y2026.into()))
        );
    }

    #[test]
    fn a_missing_or_stale_cursor_does_nothing() {
        let tree = collapsed(keyed(), HOME);
        for k in [TreeKey::Left, TreeKey::Right, TreeKey::Enter] {
            assert_eq!(key(&tree, None, k), None);
            assert_eq!(key(&tree, Some(Y2026), k), None);
            assert_eq!(key(&Tree::default(), None, k), None);
        }
    }

    #[test]
    fn append_typed_appends_within_the_timeout_and_starts_over_after_it() {
        let t0 = Instant::now();
        let ms = |n: u64| t0 + Duration::from_millis(n);
        let first = append_typed(&Typed::default(), "p", ms(1000), TYPE_AHEAD_TIMEOUT);
        assert_eq!(
            first,
            Typed {
                text: "p".into(),
                at: Some(ms(1000))
            }
        );
        assert_eq!(
            append_typed(&first, "i", ms(1500), TYPE_AHEAD_TIMEOUT).text,
            "pi"
        );
        assert_eq!(
            append_typed(&first, "i", ms(1501), TYPE_AHEAD_TIMEOUT).text,
            "i"
        );
        assert_eq!(
            append_typed(&first, "i", ms(1100), Duration::from_millis(50)).text,
            "i"
        );
    }

    /// me, (Pictures, Pix, desktop, Downloads, My Photos), card
    fn typed_rows() -> Vec<Row> {
        let mut tree = with_roots(&[home(), card()]);
        tree.expand(HOME);
        tree.set_children(
            HOME,
            0,
            ["Pictures", "Pix", "desktop", "Downloads", "My Photos"]
                .iter()
                .map(|name| child(name, &format!("/home/me/{name}")))
                .collect(),
        );
        tree.rows()
    }

    #[test]
    fn a_prefix_lands_on_the_first_folder_it_starts_case_insensitively() {
        assert_eq!(
            type_ahead(&typed_rows(), Some(HOME), "PI").as_deref(),
            Some("/home/me/Pictures")
        );
        assert_eq!(
            type_ahead(&typed_rows(), Some(HOME), "pix").as_deref(),
            Some("/home/me/Pix")
        );
    }

    #[test]
    fn typing_on_keeps_the_row_the_first_letter_found() {
        let t0 = Instant::now();
        let ms = |n: u64| t0 + Duration::from_millis(n);
        let mut buffer = append_typed(&Typed::default(), "p", ms(1000), TYPE_AHEAD_TIMEOUT);
        let first = type_ahead(&typed_rows(), Some(HOME), &buffer.text);
        assert_eq!(first.as_deref(), Some("/home/me/Pictures"));
        buffer = append_typed(&buffer, "i", ms(1200), TYPE_AHEAD_TIMEOUT);
        assert_eq!(
            type_ahead(&typed_rows(), first.as_deref(), &buffer.text).as_deref(),
            Some("/home/me/Pictures")
        );
    }

    #[test]
    fn a_tapped_letter_cycles_through_its_folders_and_wraps_around() {
        let t0 = Instant::now();
        let ms = |n: u64| t0 + Duration::from_millis(n);
        let mut buffer = append_typed(&Typed::default(), "d", ms(1000), TYPE_AHEAD_TIMEOUT);
        let mut cursor = type_ahead(&typed_rows(), Some(HOME), &buffer.text);
        assert_eq!(cursor.as_deref(), Some("/home/me/desktop"));
        buffer = append_typed(&buffer, "d", ms(1100), TYPE_AHEAD_TIMEOUT);
        assert_eq!(buffer.text, "dd");
        cursor = type_ahead(&typed_rows(), cursor.as_deref(), &buffer.text);
        assert_eq!(cursor.as_deref(), Some("/home/me/Downloads"));
        buffer = append_typed(&buffer, "d", ms(1200), TYPE_AHEAD_TIMEOUT);
        assert_eq!(
            type_ahead(&typed_rows(), cursor.as_deref(), &buffer.text).as_deref(),
            Some("/home/me/desktop")
        );
    }

    #[test]
    fn the_search_wraps_to_the_top() {
        assert_eq!(
            type_ahead(&typed_rows(), Some(CARD), "me").as_deref(),
            Some(HOME)
        );
    }

    #[test]
    fn a_space_inside_a_name() {
        assert_eq!(
            type_ahead(&typed_rows(), Some(HOME), "my p").as_deref(),
            Some("/home/me/My Photos")
        );
    }

    #[test]
    fn no_match_leaves_the_cursor() {
        assert_eq!(type_ahead(&typed_rows(), Some(HOME), "zz"), None);
        assert_eq!(type_ahead(&typed_rows(), Some(HOME), "pic!"), None);
        assert_eq!(type_ahead(&[], None, "a"), None);
    }

    #[test]
    fn a_missing_or_stale_cursor_searches_from_the_first_row() {
        assert_eq!(type_ahead(&typed_rows(), None, "m").as_deref(), Some(HOME));
        assert_eq!(
            type_ahead(&typed_rows(), Some("/gone"), "m").as_deref(),
            Some(HOME)
        );
    }

    fn win_home() -> Child {
        child("me", "C:\\Users\\me")
    }

    fn drive() -> Child {
        child("C:\\", "C:\\")
    }

    fn users() -> Child {
        child("Users", "C:\\Users")
    }

    fn windows(listed: Child) -> Tree {
        let mut tree = with_roots(&[win_home(), drive()]);
        tree.expand("C:\\");
        tree.set_children("C:\\", 0, vec![users()]);
        tree.expand("C:\\Users");
        tree.set_children(
            "C:\\Users",
            0,
            vec![listed, child("Public", "C:\\Users\\Public")],
        );
        tree
    }

    #[test]
    fn home_is_drawn_as_its_root_only_on_windows() {
        assert_eq!(
            drawn(&windows(win_home())),
            ["0:me", "0:C:\\", "1:Users", "2:Public"]
        );
    }

    #[test]
    fn home_is_drawn_as_its_root_only_on_linux() {
        let mut tree = with_roots(&[home(), child("/", "/")]);
        tree.expand("/");
        tree.set_children("/", 0, vec![child("home", "/home")]);
        tree.expand("/home");
        tree.set_children("/home", 0, vec![home()]);
        assert_eq!(drawn(&tree), ["0:me", "0:/", "1:home"]);
    }

    #[test]
    fn a_child_spelled_differently_from_the_root_is_still_hidden() {
        assert_eq!(
            drawn(&windows(child("me", "c:/Users/me/"))),
            ["0:me", "0:C:\\", "1:Users", "2:Public"]
        );
    }

    #[test]
    fn a_root_added_after_its_parent_was_listed_is_hidden_too() {
        let mut tree = with_roots(&[child("/", "/")]);
        tree.expand("/");
        tree.set_children("/", 0, vec![child("home", "/home")]);
        tree.expand("/home");
        tree.set_children("/home", 0, vec![home()]);
        assert_eq!(drawn(&tree), ["0:/", "1:home", "2:me"]);
        tree.add_roots([home()]);
        assert_eq!(drawn(&tree), ["0:/", "1:home", "0:me"]);
    }

    #[test]
    fn a_parent_holding_only_roots_has_nothing_to_expand() {
        let mut tree = with_roots(&[win_home(), drive()]);
        tree.expand("C:\\");
        tree.set_children("C:\\", 0, vec![users()]);
        tree.set_children("C:\\Users", 0, vec![win_home()]);
        let node = &tree.nodes["C:\\Users"];
        assert_eq!(node.children, Some(vec![win_home()]));
        assert_eq!(tree.drawn_children(node), Some(vec![]));
        let rows = tree.rows();
        assert!(
            !rows
                .iter()
                .find(|r| r.path == "C:\\Users")
                .unwrap()
                .expandable
        );
    }

    #[test]
    fn the_folder_is_drawn_and_watched_once_when_its_root_row_is_expanded() {
        let mut tree = windows(win_home());
        tree.expand("C:\\Users\\me");
        assert_eq!(
            tree.rows()
                .iter()
                .filter(|r| r.path == "C:\\Users\\me")
                .count(),
            1
        );
        assert_eq!(
            tree.watched_folders(),
            ["C:\\", "C:\\Users", "C:\\Users\\me"]
        );
    }

    #[test]
    fn ancestors_walk_from_the_root_down_to_the_folder_itself() {
        assert_eq!(
            ancestors_within(["/home/me", "/mnt/c"], "/home/me/Pictures/2026", false),
            Some(strings(&[
                "/home/me",
                "/home/me/Pictures",
                "/home/me/Pictures/2026"
            ]))
        );
    }

    #[test]
    fn a_root_is_its_own_chain_trailing_separator_or_not() {
        assert_eq!(
            ancestors_within(["/home/me"], "/home/me/", false),
            Some(strings(&["/home/me"]))
        );
    }

    #[test]
    fn a_sibling_sharing_a_name_prefix_is_not_held() {
        assert_eq!(ancestors_within(["/home/me"], "/home/meg/x", false), None);
    }

    #[test]
    fn the_deepest_root_wins() {
        assert_eq!(
            ancestors_within(["/", "/home/me"], "/home/me/a", false),
            Some(strings(&["/home/me", "/home/me/a"]))
        );
    }

    #[test]
    fn the_filesystem_root_holds_every_absolute_path() {
        assert_eq!(
            ancestors_within(["/"], "/data/raw", false),
            Some(strings(&["/", "/data", "/data/raw"]))
        );
    }

    #[test]
    fn a_drive_root_joins_without_doubling_the_separator() {
        assert_eq!(
            ancestors_within(["C:\\Users\\me", "D:\\"], "D:\\Photos\\2026", false),
            Some(strings(&["D:\\", "D:\\Photos", "D:\\Photos\\2026"]))
        );
    }

    #[test]
    fn a_drive_letter_matches_in_either_case_spelled_as_the_root_has_it() {
        assert_eq!(
            ancestors_within(["C:\\"], "c:\\Users", false),
            Some(strings(&["C:\\", "C:\\Users"]))
        );
    }

    #[test]
    fn forward_slashes_in_a_windows_path_still_match() {
        assert_eq!(
            ancestors_within(["C:\\Users\\me"], "C:/Users/me/Pictures", false),
            Some(strings(&["C:\\Users\\me", "C:\\Users\\me\\Pictures"]))
        );
    }

    #[test]
    fn a_unc_share() {
        assert_eq!(
            ancestors_within(["\\\\nas\\photos"], "\\\\nas\\photos\\2026", false),
            Some(strings(&["\\\\nas\\photos", "\\\\nas\\photos\\2026"]))
        );
    }

    #[test]
    fn no_root_holds_the_folder() {
        assert_eq!(ancestors_within(["C:\\"], "\\\\nas\\photos", false), None);
        assert_eq!(ancestors_within([], "/home/me", false), None);
    }

    /// Re-spells the chain level by level the way the reveal does, from each
    /// level's listing.
    fn walk(chain: Vec<String>, listings: &[(&str, Vec<Child>)], ignore_case: bool) -> Vec<String> {
        let mut out = chain;
        for i in 0..out.len() {
            let listed = listings
                .iter()
                .find(|(dir, _)| *dir == out[i])
                .map(|(_, children)| children.as_slice())
                .unwrap_or_default();
            out = respell(out, i + 1, listed, ignore_case);
        }
        out
    }

    #[test]
    fn a_windows_path_in_another_case_resolves_to_the_listed_spelling() {
        let chain = ancestors_within(["C:\\Users\\me"], "c:\\users\\ME\\pictures\\RAW", true);
        assert_eq!(
            chain,
            Some(strings(&[
                "C:\\Users\\me",
                "C:\\Users\\me\\pictures",
                "C:\\Users\\me\\pictures\\RAW"
            ]))
        );
        let listings = [
            (
                "C:\\Users\\me",
                vec![child("Pictures", "C:\\Users\\me\\Pictures")],
            ),
            (
                "C:\\Users\\me\\Pictures",
                vec![child("raw", "C:\\Users\\me\\Pictures\\raw")],
            ),
        ];
        assert_eq!(
            walk(chain.unwrap(), &listings, true),
            [
                "C:\\Users\\me",
                "C:\\Users\\me\\Pictures",
                "C:\\Users\\me\\Pictures\\raw"
            ]
        );
    }

    #[test]
    fn a_macos_path_in_another_case_resolves_with_the_flag_on() {
        let chain = ancestors_within(["/Users/me"], "/users/ME/pictures", true);
        assert_eq!(chain, Some(strings(&["/Users/me", "/Users/me/pictures"])));
        let listings = [("/Users/me", vec![child("Pictures", "/Users/me/Pictures")])];
        assert_eq!(
            walk(chain.unwrap(), &listings, true),
            ["/Users/me", "/Users/me/Pictures"]
        );
    }

    #[test]
    fn with_the_flag_off_the_case_must_match() {
        assert_eq!(ancestors_within(["/home/me"], "/home/ME/x", false), None);
        let chain = ancestors_within(["/home/me"], "/home/me/pictures", false).unwrap();
        let listings = [("/home/me", vec![child("Pictures", "/home/me/Pictures")])];
        assert_eq!(
            walk(chain, &listings, false),
            ["/home/me", "/home/me/pictures"]
        );
    }

    fn listed_under_me() -> Vec<Child> {
        vec![
            child("Pictures", "/Users/me/Pictures"),
            child("Music", "/Users/me/Music"),
        ]
    }

    fn lower_chain() -> Vec<String> {
        strings(&["/Users/me", "/Users/me/pictures", "/Users/me/pictures/2026"])
    }

    #[test]
    fn respell_rebases_the_rest_of_the_chain_onto_the_listed_child_ignoring_case() {
        assert_eq!(
            respell(lower_chain(), 1, &listed_under_me(), true),
            ["/Users/me", "/Users/me/Pictures", "/Users/me/Pictures/2026"]
        );
    }

    #[test]
    fn respell_leaves_the_chain_when_case_matters_or_nothing_matches() {
        assert_eq!(
            respell(lower_chain(), 1, &listed_under_me(), false),
            lower_chain()
        );
        assert_eq!(respell(lower_chain(), 1, &[], true), lower_chain());
        assert_eq!(
            respell(lower_chain(), 3, &listed_under_me(), true),
            lower_chain()
        );
    }

    #[test]
    fn root_of_a_drive_a_unc_share_or_the_filesystem_root() {
        assert_eq!(root_of("E:\\DCIM\\100MSDCF"), "E:\\");
        assert_eq!(root_of("\\\\nas\\photos\\2026"), "\\\\nas\\photos");
        assert_eq!(root_of("/srv/photos"), "/");
    }

    #[test]
    fn the_added_root_holds_the_folder() {
        for path in ["E:\\DCIM", "\\\\nas\\photos\\2026", "/srv/photos"] {
            let root = root_of(path);
            let chain = ancestors_within([root.as_str()], path, false).unwrap();
            assert_eq!(chain.last().map(String::as_str), Some(path));
        }
    }

    #[test]
    fn relation_ignores_case_only_when_asked() {
        assert_eq!(
            relation("D:\\photos\\2026", "d:/Photos", true),
            Some(Relation::Under)
        );
        assert_eq!(relation("D:\\photos\\2026", "d:/Photos", false), None);
        assert_eq!(
            relation("/Users/me/pictures", "/Users/me/Pictures", true),
            Some(Relation::Same)
        );
        assert_eq!(
            relation("/Users/me/pictures", "/Users/me/Pictures", false),
            None
        );
    }

    #[test]
    fn rebase_maps_the_folder_itself_and_a_path_under_it() {
        assert_eq!(
            rebase("/home/me/a", "/home/me/a", "/home/me/b", false).as_deref(),
            Some("/home/me/b")
        );
        assert_eq!(
            rebase("/home/me/a/x/y", "/home/me/a", "/home/me/b", false).as_deref(),
            Some("/home/me/b/x/y")
        );
    }

    #[test]
    fn rebase_leaves_a_sibling_sharing_the_prefix_as_a_string_alone() {
        assert_eq!(
            rebase("/home/me/a2", "/home/me/a", "/home/me/b", false),
            None
        );
        assert_eq!(rebase("/home/me", "/home/me/a", "/home/me/b", false), None);
    }

    #[test]
    fn rebase_compares_windows_paths_regardless_of_separators_and_drive_case() {
        assert_eq!(
            rebase("d:/Photos/2026/", "D:\\Photos", "D:\\Shoots", false).as_deref(),
            Some("D:\\Shoots\\2026")
        );
    }

    #[test]
    fn rebase_ignoring_case_joins_the_rest_in_its_own_spelling() {
        assert_eq!(
            rebase("d:/photos/2026/Raw", "D:\\Photos", "D:\\Shoots", true).as_deref(),
            Some("D:\\Shoots\\2026\\Raw")
        );
        assert_eq!(
            rebase("d:/photos/2026/Raw", "D:\\Photos", "D:\\Shoots", false),
            None
        );
    }

    fn renamed_tree() -> Tree {
        let mut tree = with_roots(&[home()]);
        tree.expand("/home/me");
        tree.set_children(
            "/home/me",
            0,
            vec![
                child("a", "/home/me/a"),
                child("b", "/home/me/b"),
                child("c", "/home/me/c"),
            ],
        );
        tree.expand("/home/me/a");
        tree.set_children("/home/me/a", 2, vec![child("x", "/home/me/a/x")]);
        tree.expand("/home/me/a/x");
        tree.set_children("/home/me/a/x", 1, Vec::new());
        tree
    }

    #[test]
    fn rename_folder_re_keys_the_folder_and_everything_under_it_keeping_their_state() {
        let mut tree = renamed_tree();
        tree.rename_folder("/home/me/a", "/home/me/a1", "a1", false);
        assert_eq!(drawn(&tree), ["0:me", "1:a1", "2:x", "1:b", "1:c"]);
        assert!(!tree.has("/home/me/a"));
        assert_eq!(
            tree.nodes.get("/home/me/a1"),
            Some(&Node {
                name: "a1".into(),
                path: "/home/me/a1".into(),
                children: Some(vec![child("x", "/home/me/a1/x")]),
                expanded: true,
                raw_count: Some(2),
                failed: None,
            })
        );
        assert_eq!(tree.nodes["/home/me/a1/x"].raw_count, Some(1));
    }

    #[test]
    fn rename_folder_re_sorts_the_parents_children_case_insensitively() {
        let mut tree = renamed_tree();
        tree.rename_folder("/home/me/a", "/home/me/Bz", "Bz", false);
        assert_eq!(drawn(&tree), ["0:me", "1:b", "1:Bz", "2:x", "1:c"]);
    }

    #[test]
    fn rename_folder_drops_a_stale_node_already_at_the_new_path() {
        let mut tree = renamed_tree();
        tree.set_children(
            "/home/me",
            0,
            vec![child("a", "/home/me/a"), child("d", "/home/me/d")],
        );
        tree.rename_folder("/home/me/a", "/home/me/d", "d", false);
        assert_eq!(drawn(&tree), ["0:me", "1:d", "2:x"]);
        assert_eq!(tree.nodes["/home/me/d"].raw_count, Some(2));
    }

    #[test]
    fn rename_folder_ignoring_case_re_keys_a_node_whose_key_differs_in_case() {
        let mut tree = renamed_tree();
        tree.rename_folder("/home/me/A", "/home/me/a1", "a1", true);
        assert_eq!(drawn(&tree), ["0:me", "1:a1", "2:x", "1:b", "1:c"]);
        assert!(!tree.has("/home/me/a"));
        assert_eq!(tree.nodes["/home/me/a1/x"].raw_count, Some(1));
        let mut kept = renamed_tree();
        kept.rename_folder("/home/me/A", "/home/me/a1", "a1", false);
        assert_eq!(kept, renamed_tree());
    }

    #[test]
    fn rename_folder_ignoring_case_keeps_the_state_of_a_case_only_rename() {
        let mut tree = renamed_tree();
        tree.rename_folder("/home/me/a", "/home/me/A", "A", true);
        assert_eq!(drawn(&tree), ["0:me", "1:A", "2:x", "1:b", "1:c"]);
        assert!(!tree.has("/home/me/a"));
        assert_eq!(tree.nodes["/home/me/A"].raw_count, Some(2));
    }

    #[test]
    fn rename_folder_ignoring_case_drops_a_stale_node_at_the_new_path_in_another_case() {
        let mut tree = renamed_tree();
        tree.set_children(
            "/home/me",
            0,
            vec![child("a", "/home/me/a"), child("D", "/home/me/D")],
        );
        tree.rename_folder("/home/me/a", "/home/me/d", "d", true);
        assert_eq!(drawn(&tree), ["0:me", "1:d", "2:x"]);
        assert!(!tree.has("/home/me/D"));
        assert_eq!(tree.nodes["/home/me/d"].raw_count, Some(2));
    }
}
