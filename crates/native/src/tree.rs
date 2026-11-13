//! The folder tree's state, kept pure: which folders are known, listed and
//! expanded, and the path spelling its nodes are keyed by. A port of the
//! state half of the Tauri app's `tree.ts`; the window draws `rows` and fills
//! the tree from `riffle_backend::folders`.

use std::collections::HashMap;

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
    #[allow(dead_code)]
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
/// onto `new_dir` in `path`'s own spelling. Only `respell` uses it so far;
/// part 2's folder rename re-keys the tree with it.
fn rebase(path: &str, old_dir: &str, new_dir: &str, ignore_case: bool) -> Option<String> {
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

/// Ported from `tree.test.ts`. Left out: the keyboard (`step`, `treeKey`,
/// `appendTyped`, `typeAhead`) and `descendants` / `collapseAll`, which come
/// with the tree's keyboard step; the multi-folder selection (`clickSelect`,
/// `pruneSelection`), the public `rebase` and `renameFolder`, which are
/// part 2's.
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
}
