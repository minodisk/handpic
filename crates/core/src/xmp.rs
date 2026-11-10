//! XMP sidecar reading and writing for ratings, flags and color labels.
//!
//! A rating is `xmp:Rating` in `0..=5`, `0` being unrated. The pick / reject
//! [`Flag`] is Lightroom's `xmpDM:good`: `"True"` a pick, `"False"` a reject,
//! absent unflagged; a reject keeps its stars. A legacy `xmp:Rating="-1"`
//! (the reject Adobe Bridge writes and darktable reads) reads as a reject
//! with no stars and is rewritten to Lightroom's shape. A label is read from
//! Lightroom's language-independent `photoshop:LabelColor` (`"purple"`, read
//! as `"Purple"`), else from `xmp:Label`, which a localized Lightroom fills in
//! its own language (`"パープル"`): a configured [`LabelNames`] name or an
//! English name maps to its canonical color, any other string reads raw. A
//! label is written to both, `LabelColor` the lowercase English color and
//! `xmp:Label` the configured name for it. Nothing but
//! `xmp:Rating`, `xmpDM:good`, `xmp:Label` and `photoshop:LabelColor` is
//! ever written.
//!
//! An existing sidecar is patched by splicing the bytes of those values (or
//! removing `xmpDM:good`, `xmp:Label` or `photoshop:LabelColor`), so a
//! Lightroom sidecar
//! keeps its `crs:` develop settings byte-for-byte; a sidecar is never
//! regenerated from a parse.

use std::path::{Path, PathBuf};

use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::NsReader;

use crate::Flag;

const RDF_NS: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";

/// A namespace and the prefixes to write it with, in order of preference.
struct Ns {
    uri: &'static str,
    prefixes: &'static [&'static str],
}

const XMP: Ns = Ns {
    uri: "http://ns.adobe.com/xap/1.0/",
    prefixes: &["xmp", "xap"],
};
const XMP_DM: Ns = Ns {
    uri: "http://ns.adobe.com/xmp/1.0/DynamicMedia/",
    prefixes: &["xmpDM"],
};
const PHOTOSHOP: Ns = Ns {
    uri: "http://ns.adobe.com/photoshop/1.0/",
    prefixes: &["photoshop"],
};

/// The sidecar path of an ARW: the extension replaced with `xmp`.
///
/// Pure by design: when the directory already holds a sidecar differing only
/// in case (`FOO.XMP`), the caller is the one that lists the directory and
/// prefers the existing name.
pub fn sidecar_path(arw: &Path) -> PathBuf {
    let mut path = arw.to_path_buf();
    path.set_extension("xmp");
    path
}

/// The `xmp:Rating` of the first `rdf:Description` that has one, when it is
/// in `0..=5`.
///
/// `None` when the property is absent or its value is outside the range (a
/// legacy `-1` included: the reject it meant is reported by [`read_flag`]);
/// `Err` when the bytes are not parseable XMP.
pub fn read_rating(bytes: &[u8]) -> Result<Option<i8>, String> {
    Ok(raw_rating(bytes)?.filter(|n| (0..=5).contains(n)))
}

/// The flag of the sidecar: `Pick` for `xmpDM:good="True"`, `Reject` for
/// `"False"` or for a legacy `xmp:Rating="-1"`, `None` otherwise.
///
/// The value is compared trimmed and case-insensitively; `Err` when the
/// bytes are not parseable XMP.
pub fn read_flag(bytes: &[u8]) -> Result<Flag, String> {
    let text = std::str::from_utf8(bytes).map_err(|e| format!("not UTF-8: {e}"))?;
    if let Location::Value { start, end, .. } = locate(text, &XMP_DM, "good")? {
        let value = text[start..end].trim();
        if value.eq_ignore_ascii_case("true") {
            return Ok(Flag::Pick);
        }
        if value.eq_ignore_ascii_case("false") {
            return Ok(Flag::Reject);
        }
    }
    Ok(if raw_rating(bytes)? == Some(-1) {
        Flag::Reject
    } else {
        Flag::None
    })
}

fn raw_rating(bytes: &[u8]) -> Result<Option<i8>, String> {
    let text = std::str::from_utf8(bytes).map_err(|e| format!("not UTF-8: {e}"))?;
    match locate(text, &XMP, "Rating")? {
        Location::Value { start, end, .. } => Ok(text[start..end].trim().parse::<i8>().ok()),
        Location::Insert { .. } => Ok(None),
    }
}

/// The sidecar bytes carrying `rating` and `flag`.
///
/// With `existing` `None` this is a fresh template; otherwise the existing
/// bytes with only the `Rating` value replaced in the first `rdf:Description`
/// that has one, or one attribute inserted into the first `rdf:Description`
/// when the property is absent from all of them. `xmpDM:good` is set the
/// same way to `"True"` for a pick and `"False"` for a reject, and removed
/// when unflagged.
///
/// `rating` `None` means unrated and writes `0`. On a file that has no
/// sidecar yet the caller must not write anything at all rather than call
/// this with `None` and no flag, so that clearing a rating does not litter a
/// folder with empty sidecars.
pub fn write_rating(
    existing: Option<&[u8]>,
    rating: Option<i8>,
    flag: Flag,
) -> Result<Vec<u8>, String> {
    let value = rating.unwrap_or(0);
    if !(0..=5).contains(&value) {
        return Err(format!("rating {value} is outside 0..=5"));
    }
    let value = value.to_string();
    let good = match flag {
        Flag::Pick => Some("True"),
        Flag::Reject => Some("False"),
        Flag::None => None,
    };
    let Some(existing) = existing else {
        let mut props = vec![(&XMP, "Rating", value.as_str())];
        props.extend(good.map(|g| (&XMP_DM, "good", g)));
        return Ok(template(&props).into_bytes());
    };
    let rated = set(existing, &XMP, "Rating", &value)?;
    match good {
        Some(good) => set(&rated, &XMP_DM, "good", good),
        None => remove(&rated, &XMP_DM, "good"),
    }
}

/// The five colors with an `xmp:Label` name, in canonical (English) form.
const COLORS: [&str; 5] = ["Red", "Yellow", "Green", "Blue", "Purple"];

/// The `xmp:Label` string written for each of the five canonical colors.
/// An empty name falls back to the English default.
#[derive(Clone, Debug, PartialEq)]
pub struct LabelNames {
    pub red: String,
    pub yellow: String,
    pub green: String,
    pub blue: String,
    pub purple: String,
}

impl Default for LabelNames {
    fn default() -> Self {
        Self::from_names(COLORS)
    }
}

impl LabelNames {
    fn from_names([red, yellow, green, blue, purple]: [&str; 5]) -> Self {
        Self {
            red: red.to_string(),
            yellow: yellow.to_string(),
            green: green.to_string(),
            blue: blue.to_string(),
            purple: purple.to_string(),
        }
    }

    /// The `xmp:Label` name for a canonical color (`"Red"` ... `"Purple"`),
    /// `None` for any other label.
    pub fn name<'a>(&'a self, color: &'a str) -> Option<&'a str> {
        let name = match color {
            "Red" => &self.red,
            "Yellow" => &self.yellow,
            "Green" => &self.green,
            "Blue" => &self.blue,
            "Purple" => &self.purple,
            _ => return None,
        };
        Some(if name.trim().is_empty() { color } else { name })
    }

    /// The canonical color whose configured or English name is `label`.
    fn color(&self, label: &str) -> Option<&'static str> {
        let label = label.trim();
        COLORS
            .into_iter()
            .find(|color| self.name(color).map(str::trim) == Some(label))
            .or_else(|| COLORS.into_iter().find(|color| *color == label))
    }
}

/// The label of the sidecar: a non-empty `photoshop:LabelColor` with its
/// first letter capitalized and the rest lowercased (`"purple"` ->
/// `"Purple"`), else a non-empty `xmp:Label`, mapped to its canonical color
/// when it is one of `names` or an English name and raw otherwise.
///
/// `None` when both are absent or empty; `Err` when the bytes are not
/// parseable XMP.
pub fn read_label(bytes: &[u8], names: &LabelNames) -> Result<Option<String>, String> {
    let text = std::str::from_utf8(bytes).map_err(|e| format!("not UTF-8: {e}"))?;
    if let Location::Value { start, end, .. } = locate(text, &PHOTOSHOP, "LabelColor")? {
        let color = text[start..end].trim().to_lowercase();
        let mut chars = color.chars();
        if let Some(first) = chars.next() {
            return Ok(Some(first.to_uppercase().chain(chars).collect()));
        }
    }
    match locate(text, &XMP, "Label")? {
        Location::Value { start, end, .. } if !text[start..end].trim().is_empty() => {
            let label = &text[start..end];
            Ok(Some(names.color(label).unwrap_or(label).to_string()))
        }
        _ => Ok(None),
    }
}

/// The sidecar bytes carrying `label`: `xmp:Label` set to its name in
/// `names` (`label` itself when it is not one of the five colors) and
/// `photoshop:LabelColor` to `label` lowercased.
///
/// `Some` splices each value in place or inserts one attribute, as
/// [`write_rating`] does; `None` removes both properties and leaves a sidecar
/// without them byte-identical. With `existing` `None`, `Some` is a fresh
/// template and `None` is an error: no sidecar is minted for "no label".
pub fn write_label(
    existing: Option<&[u8]>,
    label: Option<&str>,
    names: &LabelNames,
) -> Result<Vec<u8>, String> {
    let color = label.map(str::to_lowercase);
    let label = label.map(|label| names.name(label).unwrap_or(label));
    let Some(existing) = existing else {
        return match (label, color.as_deref()) {
            (Some(label), Some(color)) => {
                Ok(
                    template(&[(&XMP, "Label", label), (&PHOTOSHOP, "LabelColor", color)])
                        .into_bytes(),
                )
            }
            _ => Err("no sidecar to clear a label from".to_string()),
        };
    };
    match (label, color.as_deref()) {
        (Some(label), Some(color)) => set(
            &set(existing, &XMP, "Label", label)?,
            &PHOTOSHOP,
            "LabelColor",
            color,
        ),
        _ => remove(&remove(existing, &XMP, "Label")?, &PHOTOSHOP, "LabelColor"),
    }
}

fn remove(existing: &[u8], ns: &Ns, name: &str) -> Result<Vec<u8>, String> {
    let text = std::str::from_utf8(existing).map_err(|e| format!("not UTF-8: {e}"))?;
    Ok(match locate(text, ns, name)? {
        Location::Value { remove, .. } => {
            format!("{}{}", &text[..remove.0], &text[remove.1..]).into_bytes()
        }
        Location::Insert { .. } => existing.to_vec(),
    })
}

fn set(existing: &[u8], ns: &Ns, name: &str, value: &str) -> Result<Vec<u8>, String> {
    let text = std::str::from_utf8(existing).map_err(|e| format!("not UTF-8: {e}"))?;
    let mut out = String::with_capacity(text.len() + 32);
    match locate(text, ns, name)? {
        Location::Value { start, end, .. } => {
            out.push_str(&text[..start]);
            out.push_str(value);
            out.push_str(&text[end..]);
        }
        Location::Insert {
            at,
            prefix,
            declare,
        } => {
            out.push_str(&text[..at]);
            if declare {
                out.push_str(&format!(" xmlns:{prefix}=\"{}\"", ns.uri));
            }
            out.push_str(&format!(" {prefix}:{name}=\"{value}\""));
            out.push_str(&text[at..]);
        }
    }
    Ok(out.into_bytes())
}

/// A fresh sidecar whose single `rdf:Description` declares each namespace of
/// `props` once, with its preferred prefix, and carries `props` in order.
fn template(props: &[(&Ns, &str, &str)]) -> String {
    let mut attrs = String::new();
    for (i, (ns, _, _)) in props.iter().enumerate() {
        if !props[..i].iter().any(|(seen, _, _)| seen.uri == ns.uri) {
            attrs.push_str(&format!(" xmlns:{}=\"{}\"", ns.prefixes[0], ns.uri));
        }
    }
    for (ns, name, value) in props {
        attrs.push_str(&format!(" {}:{name}=\"{value}\"", ns.prefixes[0]));
    }
    format!(
        "<?xpacket begin=\"\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
         <x:xmpmeta xmlns:x=\"adobe:ns:meta/\" x:xmptk=\"handpic\">\n\
         \x20<rdf:RDF xmlns:rdf=\"{RDF_NS}\">\n\
         \x20 <rdf:Description rdf:about=\"\"{attrs}/>\n\
         \x20</rdf:RDF>\n\
         </x:xmpmeta>\n\
         <?xpacket end=\"w\"?>\n"
    )
}

enum Location {
    /// The byte range of the property's value, attribute or element text,
    /// and the range removing the whole property: the attribute with the
    /// whitespace before it, or the element (with its line when alone on it).
    Value {
        start: usize,
        end: usize,
        remove: (usize, usize),
    },
    /// Where to insert an attribute in the first `rdf:Description` start tag.
    Insert {
        at: usize,
        prefix: String,
        declare: bool,
    },
}

fn locate(text: &str, ns: &Ns, name: &str) -> Result<Location, String> {
    let mut reader = NsReader::from_str(text);
    let mut pos = 0usize;
    let mut insert: Option<Location> = None;
    let mut first_description_seen = false;
    let mut element: Option<(usize, usize)> = None;
    loop {
        let event = reader
            .read_event()
            .map_err(|e| format!("XMP parse error: {e}"))?;
        let end = reader.buffer_position() as usize;
        let span = &text[pos..end];
        match &event {
            Event::Eof => break,
            Event::Start(e) | Event::Empty(e) => {
                let (ens, local) = reader.resolver().resolve_element(e.name());
                if bound_to(&ens, RDF_NS) && local.as_ref() == "Description" {
                    for attr in e.attributes() {
                        let attr = attr.map_err(|e| format!("XMP parse error: {e}"))?;
                        let (ans, alocal) = reader.resolver().resolve_attribute(attr.key);
                        if bound_to(&ans, ns.uri) && alocal.as_ref() == name {
                            if let Some((k, s, e)) = attr_value_range(span, attr.key.as_ref()) {
                                let lead = span[..k].trim_end().len();
                                return Ok(Location::Value {
                                    start: pos + s,
                                    end: pos + e,
                                    remove: (pos + lead, pos + e + 1),
                                });
                            }
                        }
                    }
                    if !first_description_seen {
                        first_description_seen = true;
                        let (prefix, declare) = prefix_for(&reader, ns);
                        insert = Some(Location::Insert {
                            at: pos + insert_offset(span),
                            prefix,
                            declare,
                        });
                    }
                } else if bound_to(&ens, ns.uri) && local.as_ref() == name {
                    element = matches!(event, Event::Start(_)).then_some((pos, end));
                }
            }
            Event::End(e) => {
                let (ens, local) = reader.resolver().resolve_element(e.name());
                if bound_to(&ens, ns.uri) && local.as_ref() == name {
                    if let Some((open, open_tag_end)) = element {
                        // The value range spans everything between the open
                        // and close tags, `open_tag_end..pos` (`pos` is the
                        // start of this End event), so entity references such
                        // as `&amp;` inside the text stay part of the value
                        // instead of being cut at the first `Text` event.
                        return Ok(Location::Value {
                            start: open_tag_end,
                            end: pos,
                            remove: line_or_element(text, open, end),
                        });
                    }
                }
                element = None;
            }
            _ => {}
        }
        pos = end;
    }
    insert.ok_or_else(|| "no rdf:Description element".to_string())
}

fn bound_to(ns: &ResolveResult<'_>, expected: &str) -> bool {
    matches!(ns, ResolveResult::Bound(n) if n.as_ref() == expected)
}

/// The whole line when the element `open..close` sits alone on it, otherwise
/// just the element.
fn line_or_element(text: &str, open: usize, close: usize) -> (usize, usize) {
    let line_start = text[..open].rfind('\n').map_or(0, |i| i + 1);
    let Some(nl) = text[close..].find('\n') else {
        return (open, close);
    };
    if text[line_start..open].trim().is_empty() && text[close..close + nl].trim().is_empty() {
        (line_start, close + nl + 1)
    } else {
        (open, close)
    }
}

/// The prefix to write a property with, and whether it has to be declared.
///
/// For each preferred prefix of `ns` (`xmp` then `xap` for XMP): a prefix
/// already bound to the namespace is reused as is, an undeclared one is used
/// and declared, and one bound to another namespace is skipped so that
/// binding is never rewritten. When all are bound elsewhere, the first
/// undeclared `<first>1`, `<first>2`, ... (`xmp1`, `xmpDM1`) is declared
/// instead.
fn prefix_for(reader: &NsReader<&[u8]>, ns: &Ns) -> (String, bool) {
    for candidate in ns.prefixes {
        match resolve_prefix(reader, candidate) {
            ResolveResult::Bound(n) if n.as_ref() == ns.uri => {
                return (candidate.to_string(), false)
            }
            ResolveResult::Bound(_) => continue,
            _ => return (candidate.to_string(), true),
        }
    }
    let first = ns.prefixes[0];
    for n in 1..=99 {
        let candidate = format!("{first}{n}");
        if !matches!(resolve_prefix(reader, &candidate), ResolveResult::Bound(_)) {
            return (candidate, true);
        }
    }
    (format!("{first}99"), true)
}

fn resolve_prefix<'a>(reader: &'a NsReader<&[u8]>, prefix: &str) -> ResolveResult<'a> {
    let name = format!("{prefix}:Rating");
    reader
        .resolver()
        .resolve_attribute(quick_xml::name::QName(&name))
        .0
}

/// The offset of `key` and the byte range of its value inside a start tag,
/// quotes excluded.
///
/// quick-xml reports positions per event, not per attribute, so the value is
/// located by scanning the start tag's own range, which is exact.
fn attr_value_range(tag: &str, key: &str) -> Option<(usize, usize, usize)> {
    let mut from = 0usize;
    while let Some(rel) = tag[from..].find(key) {
        let start = from + rel;
        from = start + key.len();
        if !tag[..start]
            .chars()
            .next_back()
            .is_some_and(char::is_whitespace)
        {
            continue;
        }
        let after_key = tag[from..].trim_start();
        let Some(after_eq) = after_key.strip_prefix('=') else {
            continue;
        };
        let after_eq = after_eq.trim_start();
        let Some(quote) = after_eq.chars().next() else {
            continue;
        };
        if quote != '"' && quote != '\'' {
            continue;
        }
        let value_start = tag.len() - after_eq.len() + 1;
        let value_end = value_start + tag[value_start..].find(quote)?;
        return Some((start, value_start, value_end));
    }
    None
}

/// Where an attribute can be appended in a start tag: before `>` or `/>`.
fn insert_offset(tag: &str) -> usize {
    let close = tag.rfind('>').unwrap_or(tag.len());
    let before = tag[..close].trim_end();
    if before.ends_with('/') {
        before.len() - 1
    } else {
        close
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn japanese() -> LabelNames {
        crate::i18n::preset("ja").unwrap().names.clone()
    }

    const BRIDGE: &str = concat!(
        "<?xpacket begin=\"\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n",
        "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\" x:xmptk=\"Adobe XMP Core\">\n",
        " <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n",
        "  <rdf:Description rdf:about=\"\"\n",
        "    xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\"\n",
        "   xmp:Rating=\"3\"\n",
        "   xmp:CreatorTool=\"Bridge\"/>\n",
        " </rdf:RDF>\n",
        "</x:xmpmeta>\n",
        "<?xpacket end=\"w\"?>\n",
    );

    const LIGHTROOM: &str = concat!(
        "<?xpacket begin=\"\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n",
        "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\" x:xmptk=\"Adobe XMP Core 9.0\">\n",
        " <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n",
        "  <rdf:Description rdf:about=\"\"\n",
        "    xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\"\n",
        "    xmlns:dc=\"http://purl.org/dc/elements/1.1/\"\n",
        "    xmlns:crs=\"http://ns.adobe.com/camera-raw-settings/1.0/\"\n",
        "    crs:Version=\"15.0\"\n",
        "    crs:ProcessVersion=\"11.0\"\n",
        "    crs:WhiteBalance=\"As Shot\"\n",
        "    crs:Temperature=\"5200\"\n",
        "    crs:Tint=\"+8\"\n",
        "    crs:Exposure2012=\"+0.35\"\n",
        "    crs:Contrast2012=\"+12\"\n",
        "    crs:Highlights2012=\"-40\"\n",
        "    crs:Shadows2012=\"+25\"\n",
        "    crs:ToneCurveName2012=\"Medium Contrast\">\n",
        "   <xmp:Rating>2</xmp:Rating>\n",
        "   <xmp:CreateDate>2026-09-18T10:00:00</xmp:CreateDate>\n",
        "   <dc:subject>\n",
        "    <rdf:Bag>\n",
        "     <rdf:li>game</rdf:li>\n",
        "     <rdf:li>keeper</rdf:li>\n",
        "    </rdf:Bag>\n",
        "   </dc:subject>\n",
        "  </rdf:Description>\n",
        " </rdf:RDF>\n",
        "</x:xmpmeta>\n",
        "<?xpacket end=\"w\"?>\n",
    );

    const NO_RATING: &str = concat!(
        "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n",
        " <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n",
        "  <rdf:Description rdf:about=\"\" xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\"\n",
        "   xmp:CreatorTool=\"darktable\"/>\n",
        " </rdf:RDF>\n",
        "</x:xmpmeta>\n",
    );

    fn patched(source: &str, rating: Option<i8>) -> String {
        flagged(source, rating, Flag::None)
    }

    fn flagged(source: &str, rating: Option<i8>, flag: Flag) -> String {
        String::from_utf8(write_rating(Some(source.as_bytes()), rating, flag).unwrap()).unwrap()
    }

    #[test]
    fn sidecar_path_replaces_the_extension() {
        assert_eq!(
            sidecar_path(Path::new("/photos/FOO.ARW")),
            PathBuf::from("/photos/FOO.xmp")
        );
    }

    #[test]
    fn reads_an_attribute_sidecar() {
        assert_eq!(read_rating(BRIDGE.as_bytes()).unwrap(), Some(3));
    }

    #[test]
    fn reads_an_element_sidecar() {
        assert_eq!(read_rating(LIGHTROOM.as_bytes()).unwrap(), Some(2));
    }

    #[test]
    fn patches_only_the_attribute_value() {
        let out = patched(BRIDGE, Some(5));
        assert_eq!(out, BRIDGE.replace("xmp:Rating=\"3\"", "xmp:Rating=\"5\""));
        assert_eq!(read_rating(out.as_bytes()).unwrap(), Some(5));
    }

    #[test]
    fn patches_only_the_element_text() {
        let out = patched(LIGHTROOM, Some(4));
        assert_eq!(
            out,
            LIGHTROOM.replace("<xmp:Rating>2</xmp:Rating>", "<xmp:Rating>4</xmp:Rating>")
        );
        assert_eq!(read_rating(out.as_bytes()).unwrap(), Some(4));
    }

    #[test]
    fn clearing_writes_zero_into_an_existing_sidecar() {
        assert_eq!(
            read_rating(patched(LIGHTROOM, None).as_bytes()).unwrap(),
            Some(0)
        );
    }

    #[test]
    fn inserts_exactly_one_attribute_when_absent() {
        let out = patched(NO_RATING, Some(4));
        assert_eq!(
            out,
            NO_RATING.replace(
                "xmp:CreatorTool=\"darktable\"/>",
                "xmp:CreatorTool=\"darktable\" xmp:Rating=\"4\"/>"
            )
        );
        assert_eq!(read_rating(out.as_bytes()).unwrap(), Some(4));
    }

    #[test]
    fn declares_the_namespace_when_the_description_has_none() {
        let source = concat!(
            "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n",
            " <rdf:Description rdf:about=\"\"></rdf:Description>\n",
            "</rdf:RDF>\n",
        );
        let out = patched(source, Some(1));
        assert!(out.contains(
            "<rdf:Description rdf:about=\"\" xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\" xmp:Rating=\"1\">"
        ));
        assert_eq!(read_rating(out.as_bytes()).unwrap(), Some(1));
    }

    #[test]
    fn handles_the_xap_prefix() {
        let source = concat!(
            "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"\n",
            "  xmlns:xap=\"http://ns.adobe.com/xap/1.0/\">\n",
            " <rdf:Description rdf:about=\"\" xap:Rating=\"1\"/>\n",
            "</rdf:RDF>\n",
        );
        assert_eq!(read_rating(source.as_bytes()).unwrap(), Some(1));
        assert_eq!(
            patched(source, Some(4)),
            source.replace("xap:Rating=\"1\"", "xap:Rating=\"4\"")
        );
    }

    #[test]
    fn skips_an_xmp_prefix_bound_elsewhere() {
        let source = concat!(
            "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n",
            " <rdf:Description rdf:about=\"\" xmlns:xmp=\"http://example.com/other/\"></rdf:Description>\n",
            "</rdf:RDF>\n",
        );
        let out = patched(source, Some(3));
        assert!(out.contains("xmlns:xmp=\"http://example.com/other/\""));
        assert!(out.contains("xmlns:xap=\"http://ns.adobe.com/xap/1.0/\""));
        assert!(out.contains("xap:Rating=\"3\""));
        assert_eq!(read_rating(out.as_bytes()).unwrap(), Some(3));
    }

    #[test]
    fn generates_a_prefix_when_xmp_and_xap_are_bound_elsewhere() {
        let source = concat!(
            "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n",
            " <rdf:Description rdf:about=\"\" xmlns:xmp=\"http://example.com/other/\"",
            " xmlns:xap=\"http://example.com/another/\"></rdf:Description>\n",
            "</rdf:RDF>\n",
        );
        let out = patched(source, Some(2));
        assert!(out.contains("xmlns:xmp=\"http://example.com/other/\""));
        assert!(out.contains("xmlns:xap=\"http://example.com/another/\""));
        assert!(out.contains("xmlns:xmp1=\"http://ns.adobe.com/xap/1.0/\""));
        assert!(out.contains("xmp1:Rating=\"2\""));
        assert_eq!(read_rating(out.as_bytes()).unwrap(), Some(2));
        assert_eq!(
            patched(&out, Some(5)),
            out.replace("xmp1:Rating=\"2\"", "xmp1:Rating=\"5\"")
        );
    }

    #[test]
    fn handles_a_default_namespace_description() {
        let source = concat!(
            "<RDF xmlns=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"\n",
            "  xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\">\n",
            " <Description about=\"\">\n",
            "  <xmp:Rating>4</xmp:Rating>\n",
            " </Description>\n",
            "</RDF>\n",
        );
        assert_eq!(read_rating(source.as_bytes()).unwrap(), Some(4));
        assert_eq!(
            patched(source, Some(0)),
            source.replace("<xmp:Rating>4<", "<xmp:Rating>0<")
        );
    }

    #[test]
    fn an_out_of_range_value_reads_as_absent() {
        let source = BRIDGE.replace("xmp:Rating=\"3\"", "xmp:Rating=\"9\"");
        assert_eq!(read_rating(source.as_bytes()).unwrap(), None);
    }

    #[test]
    fn a_sidecar_without_a_rating_reads_as_none() {
        assert_eq!(read_rating(NO_RATING.as_bytes()).unwrap(), None);
    }

    #[test]
    fn finds_a_rating_on_a_sibling_description() {
        let source = concat!(
            "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n",
            " <rdf:Description rdf:about=\"\"\n",
            "   xmlns:tiff=\"http://ns.adobe.com/tiff/1.0/\"\n",
            "   tiff:Make=\"SONY\"/>\n",
            " <rdf:Description rdf:about=\"\"\n",
            "   xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\"\n",
            "   xmp:Rating=\"4\"/>\n",
            "</rdf:RDF>\n",
        );
        assert_eq!(read_rating(source.as_bytes()).unwrap(), Some(4));
        assert_eq!(
            patched(source, Some(2)),
            source.replace("xmp:Rating=\"4\"", "xmp:Rating=\"2\"")
        );
    }

    #[test]
    fn splices_an_empty_element_rating_in_place() {
        let source = concat!(
            "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n",
            " <rdf:Description rdf:about=\"\"\n",
            "   xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\">\n",
            "  <xmp:Rating></xmp:Rating>\n",
            " </rdf:Description>\n",
            "</rdf:RDF>\n",
        );
        let out = patched(source, Some(2));
        assert_eq!(
            out,
            source.replace("<xmp:Rating></xmp:Rating>", "<xmp:Rating>2</xmp:Rating>")
        );
        assert_eq!(read_rating(out.as_bytes()).unwrap(), Some(2));
    }

    #[test]
    fn a_truncated_document_is_an_error() {
        let source = &BRIDGE[..BRIDGE.len() / 2];
        assert!(read_rating(source.as_bytes()).is_err());
        assert!(write_rating(Some(source.as_bytes()), Some(1), Flag::None).is_err());
        assert!(read_flag(source.as_bytes()).is_err());
    }

    #[test]
    fn a_document_without_a_description_is_an_error() {
        assert!(read_rating(b"<html><body>hello</body></html>").is_err());
    }

    #[test]
    fn a_fresh_template_round_trips() {
        for n in 0..=5 {
            for flag in [Flag::None, Flag::Pick, Flag::Reject] {
                let bytes = write_rating(None, Some(n), flag).unwrap();
                assert_eq!(read_rating(&bytes).unwrap(), Some(n));
                assert_eq!(read_flag(&bytes).unwrap(), flag);
            }
        }
    }

    #[test]
    fn the_fresh_template_is_the_documented_one() {
        assert_eq!(
            String::from_utf8(write_rating(None, Some(3), Flag::None).unwrap()).unwrap(),
            concat!(
                "<?xpacket begin=\"\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n",
                "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\" x:xmptk=\"handpic\">\n",
                " <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n",
                "  <rdf:Description rdf:about=\"\" xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\" xmp:Rating=\"3\"/>\n",
                " </rdf:RDF>\n",
                "</x:xmpmeta>\n",
                "<?xpacket end=\"w\"?>\n",
            )
        );
    }

    #[test]
    fn a_rating_outside_the_range_is_an_error() {
        assert!(write_rating(None, Some(6), Flag::None).is_err());
        assert!(write_rating(None, Some(-1), Flag::Reject).is_err());
    }

    #[test]
    fn the_fresh_template_declares_xmp_dm_only_for_a_flag() {
        let template =
            |flag| String::from_utf8(write_rating(None, Some(2), flag).unwrap()).unwrap();
        let description = |attrs: &str| {
            format!(
                concat!(
                    "<?xpacket begin=\"\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n",
                    "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\" x:xmptk=\"handpic\">\n",
                    " <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n",
                    "  <rdf:Description rdf:about=\"\" xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\"{}/>\n",
                    " </rdf:RDF>\n",
                    "</x:xmpmeta>\n",
                    "<?xpacket end=\"w\"?>\n",
                ),
                attrs
            )
        };
        assert_eq!(template(Flag::None), description(" xmp:Rating=\"2\""));
        assert_eq!(
            template(Flag::Pick),
            description(
                " xmlns:xmpDM=\"http://ns.adobe.com/xmp/1.0/DynamicMedia/\" xmp:Rating=\"2\" xmpDM:good=\"True\""
            )
        );
        assert_eq!(
            template(Flag::Reject),
            description(
                " xmlns:xmpDM=\"http://ns.adobe.com/xmp/1.0/DynamicMedia/\" xmp:Rating=\"2\" xmpDM:good=\"False\""
            )
        );
    }

    const LIGHTROOM_FLAGGED: &str = concat!(
        "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\" x:xmptk=\"Adobe XMP Core 7.0-c000 1.000000, 0000/00/00-00:00:00        \">\n",
        " <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n",
        "  <rdf:Description rdf:about=\"Leica Camera AG\"\n",
        "    xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\"\n",
        "    xmlns:photoshop=\"http://ns.adobe.com/photoshop/1.0/\"\n",
        "    xmlns:xmpDM=\"http://ns.adobe.com/xmp/1.0/DynamicMedia/\"\n",
        "    xmlns:crs=\"http://ns.adobe.com/camera-raw-settings/1.0/\"\n",
        "   xmp:Rating=\"0\"\n",
        "   xmp:CreatorTool=\"2.6.0\"\n",
        "   photoshop:SidecarForExtension=\"DNG\"\n",
        "   xmpDM:good=\"False\"\n",
        "   crs:Version=\"18.5.1\"\n",
        "   crs:AlreadyApplied=\"False\">\n",
        "  </rdf:Description>\n",
        " </rdf:RDF>\n",
        "</x:xmpmeta>\n",
    );

    #[test]
    fn reads_a_lightroom_flag_attribute() {
        assert_eq!(
            read_flag(LIGHTROOM_FLAGGED.as_bytes()).unwrap(),
            Flag::Reject
        );
        assert_eq!(read_rating(LIGHTROOM_FLAGGED.as_bytes()).unwrap(), Some(0));
        let pick = LIGHTROOM_FLAGGED.replace("xmpDM:good=\"False\"", "xmpDM:good=\"True\"");
        assert_eq!(read_flag(pick.as_bytes()).unwrap(), Flag::Pick);
        let loose = LIGHTROOM_FLAGGED.replace("xmpDM:good=\"False\"", "xmpDM:good=\" true \"");
        assert_eq!(read_flag(loose.as_bytes()).unwrap(), Flag::Pick);
        let unflagged = LIGHTROOM_FLAGGED.replace("   xmpDM:good=\"False\"\n", "");
        assert_eq!(read_flag(unflagged.as_bytes()).unwrap(), Flag::None);
        assert_eq!(read_flag(BRIDGE.as_bytes()).unwrap(), Flag::None);
    }

    #[test]
    fn reads_and_patches_a_flag_element() {
        let source = LIGHTROOM
            .replace(
                "   <xmp:Rating>2</xmp:Rating>\n",
                "   <xmp:Rating>2</xmp:Rating>\n   <xmpDM:good>True</xmpDM:good>\n",
            )
            .replace(
                "    xmlns:dc=",
                "    xmlns:xmpDM=\"http://ns.adobe.com/xmp/1.0/DynamicMedia/\"\n    xmlns:dc=",
            );
        assert_eq!(read_flag(source.as_bytes()).unwrap(), Flag::Pick);
        let rejected = flagged(&source, Some(2), Flag::Reject);
        assert_eq!(
            rejected,
            source.replace("<xmpDM:good>True<", "<xmpDM:good>False<")
        );
        let cleared = flagged(&rejected, Some(2), Flag::None);
        assert_eq!(
            cleared,
            source.replace("   <xmpDM:good>True</xmpDM:good>\n", "")
        );
        assert_eq!(read_flag(cleared.as_bytes()).unwrap(), Flag::None);
    }

    #[test]
    fn pick_reject_none_transitions_touch_only_the_flag() {
        let unflagged = LIGHTROOM_FLAGGED.replace("   xmpDM:good=\"False\"\n", "");
        let picked = flagged(&unflagged, Some(0), Flag::Pick);
        assert_eq!(
            picked,
            unflagged.replace(
                "crs:AlreadyApplied=\"False\">",
                "crs:AlreadyApplied=\"False\" xmpDM:good=\"True\">"
            )
        );
        let rejected = flagged(&picked, Some(0), Flag::Reject);
        assert_eq!(
            rejected,
            picked.replace("xmpDM:good=\"True\"", "xmpDM:good=\"False\"")
        );
        assert_eq!(flagged(&rejected, Some(0), Flag::None), unflagged);

        let picked = flagged(LIGHTROOM_FLAGGED, Some(0), Flag::Pick);
        assert_eq!(
            picked,
            LIGHTROOM_FLAGGED.replace("xmpDM:good=\"False\"", "xmpDM:good=\"True\"")
        );
        assert_eq!(flagged(LIGHTROOM_FLAGGED, Some(0), Flag::None), unflagged);
    }

    #[test]
    fn a_reject_keeps_its_stars() {
        let out = flagged(LIGHTROOM_FLAGGED, Some(3), Flag::Reject);
        assert_eq!(
            out,
            LIGHTROOM_FLAGGED.replace("xmp:Rating=\"0\"", "xmp:Rating=\"3\"")
        );
        assert_eq!(read_rating(out.as_bytes()).unwrap(), Some(3));
        assert_eq!(read_flag(out.as_bytes()).unwrap(), Flag::Reject);
    }

    #[test]
    fn a_legacy_minus_one_reads_as_a_starless_reject_and_is_rewritten() {
        let legacy = BRIDGE.replace("xmp:Rating=\"3\"", "xmp:Rating=\"-1\"");
        assert_eq!(read_rating(legacy.as_bytes()).unwrap(), None);
        assert_eq!(read_flag(legacy.as_bytes()).unwrap(), Flag::Reject);
        let out = flagged(&legacy, None, Flag::Reject);
        assert_eq!(
            out,
            BRIDGE
                .replace("xmp:Rating=\"3\"", "xmp:Rating=\"0\"")
                .replace(
                    "xmp:CreatorTool=\"Bridge\"/>",
                    "xmp:CreatorTool=\"Bridge\" xmlns:xmpDM=\"http://ns.adobe.com/xmp/1.0/DynamicMedia/\" xmpDM:good=\"False\"/>"
                )
        );
        assert_eq!(read_rating(out.as_bytes()).unwrap(), Some(0));
        assert_eq!(read_flag(out.as_bytes()).unwrap(), Flag::Reject);
        let starred = flagged(&legacy, Some(2), Flag::None);
        assert_eq!(
            starred,
            BRIDGE.replace("xmp:Rating=\"3\"", "xmp:Rating=\"2\"")
        );
    }

    #[test]
    fn skips_an_xmp_dm_prefix_bound_elsewhere() {
        let source = BRIDGE.replace(
            "    xmlns:xmp=",
            "    xmlns:xmpDM=\"http://example.com/other/\"\n    xmlns:xmp=",
        );
        let out = flagged(&source, Some(3), Flag::Pick);
        assert_eq!(
            out,
            source.replace(
                "xmp:CreatorTool=\"Bridge\"/>",
                "xmp:CreatorTool=\"Bridge\" xmlns:xmpDM1=\"http://ns.adobe.com/xmp/1.0/DynamicMedia/\" xmpDM1:good=\"True\"/>"
            )
        );
        assert_eq!(read_flag(out.as_bytes()).unwrap(), Flag::Pick);
        assert_eq!(
            flagged(&out, Some(3), Flag::None),
            out.replace(" xmpDM1:good=\"True\"", "")
        );
    }
    fn labeled(source: &str, label: Option<&str>) -> String {
        String::from_utf8(
            write_label(Some(source.as_bytes()), label, &LabelNames::default()).unwrap(),
        )
        .unwrap()
    }

    const PHOTOSHOP_DECL: &str = "xmlns:photoshop=\"http://ns.adobe.com/photoshop/1.0/\"";

    fn bridge_declaring_photoshop() -> String {
        BRIDGE.replace(
            "xmp:CreatorTool=\"Bridge\"/>",
            &format!("xmp:CreatorTool=\"Bridge\" {PHOTOSHOP_DECL}/>"),
        )
    }

    fn with_bridge_label(label: &str) -> String {
        BRIDGE.replace(
            "   xmp:Rating=\"3\"\n",
            &format!("   xmp:Rating=\"3\"\n   xmp:Label=\"{label}\"\n"),
        )
    }

    fn with_attribute_label(label: &str) -> String {
        BRIDGE.replace(
            "   xmp:Rating=\"3\"\n",
            &format!(
                "   xmp:Rating=\"3\"\n   xmp:Label=\"{label}\"\n   {PHOTOSHOP_DECL}\n   photoshop:LabelColor=\"{}\"\n",
                label.to_lowercase()
            ),
        )
    }

    fn with_element_label(label: &str) -> String {
        LIGHTROOM
            .replace(
                "    xmlns:dc=",
                &format!("    {PHOTOSHOP_DECL}\n    xmlns:dc="),
            )
            .replace(
                "   <xmp:Rating>2</xmp:Rating>\n",
                &format!(
                    "   <xmp:Rating>2</xmp:Rating>\n   <xmp:Label>{label}</xmp:Label>\n   <photoshop:LabelColor>{}</photoshop:LabelColor>\n",
                    label.to_lowercase()
                ),
            )
    }

    const LIGHTROOM_LABELED: &str = concat!(
        "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\" x:xmptk=\"Adobe XMP Core 7.0-c000 1.000000, 0000/00/00-00:00:00        \">\n",
        " <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n",
        "  <rdf:Description rdf:about=\"Leica Camera AG\"\n",
        "    xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\"\n",
        "    xmlns:photoshop=\"http://ns.adobe.com/photoshop/1.0/\"\n",
        "    xmlns:crs=\"http://ns.adobe.com/camera-raw-settings/1.0/\"\n",
        "   xmp:Rating=\"0\"\n",
        "   xmp:Label=\"パープル\"\n",
        "   xmp:CreatorTool=\"2.6.0\"\n",
        "   photoshop:SidecarForExtension=\"DNG\"\n",
        "   photoshop:LabelColor=\"purple\"\n",
        "   crs:Version=\"18.5.1\">\n",
        "  </rdf:Description>\n",
        " </rdf:RDF>\n",
        "</x:xmpmeta>\n",
    );

    #[test]
    fn reads_a_lightroom_label_color_over_the_localized_label() {
        assert_eq!(
            read_label(LIGHTROOM_LABELED.as_bytes(), &LabelNames::default()).unwrap(),
            Some("Purple".to_string())
        );
        let empty = LIGHTROOM_LABELED.replace("LabelColor=\"purple\"", "LabelColor=\"\"");
        assert_eq!(
            read_label(empty.as_bytes(), &LabelNames::default()).unwrap(),
            Some("パープル".to_string())
        );
    }

    #[test]
    fn writes_and_reads_japanese_label_names() {
        let ja = japanese();
        let out = write_label(Some(BRIDGE.as_bytes()), Some("Red"), &ja).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("xmp:Label=\"レッド\""));
        assert!(text.contains("LabelColor=\"red\""));
        assert_eq!(
            read_label(text.as_bytes(), &ja).unwrap(),
            Some("Red".to_string())
        );
        let stripped = remove(text.as_bytes(), &PHOTOSHOP, "LabelColor").unwrap();
        assert_eq!(read_label(&stripped, &ja).unwrap(), Some("Red".to_string()));
    }

    #[test]
    fn maps_english_and_configured_label_names_without_a_label_color() {
        let ja = japanese();
        assert_eq!(
            read_label(with_bridge_label("Red").as_bytes(), &ja).unwrap(),
            Some("Red".to_string())
        );
        let stripped = LIGHTROOM_LABELED.replace("   photoshop:LabelColor=\"purple\"\n", "");
        assert_eq!(
            read_label(stripped.as_bytes(), &ja).unwrap(),
            Some("Purple".to_string())
        );
        assert_eq!(
            read_label(stripped.as_bytes(), &LabelNames::default()).unwrap(),
            Some("パープル".to_string())
        );
        assert_eq!(
            read_label(with_bridge_label("Violet").as_bytes(), &ja).unwrap(),
            Some("Violet".to_string())
        );
    }

    #[test]
    fn an_empty_label_name_falls_back_to_english() {
        let names = LabelNames {
            red: String::new(),
            ..japanese()
        };
        assert_eq!(names.name("Red"), Some("Red"));
        let out = write_label(Some(BRIDGE.as_bytes()), Some("Red"), &names).unwrap();
        assert!(String::from_utf8(out)
            .unwrap()
            .contains("xmp:Label=\"Red\""));
        assert_eq!(names.name("Orange"), None);
    }

    #[test]
    fn relabels_and_clears_a_lightroom_sidecar() {
        let red = labeled(LIGHTROOM_LABELED, Some("Red"));
        assert_eq!(
            red,
            LIGHTROOM_LABELED
                .replace("xmp:Label=\"パープル\"", "xmp:Label=\"Red\"")
                .replace("LabelColor=\"purple\"", "LabelColor=\"red\"")
        );
        assert_eq!(
            read_label(red.as_bytes(), &LabelNames::default()).unwrap(),
            Some("Red".to_string())
        );
        assert_eq!(
            labeled(LIGHTROOM_LABELED, None),
            LIGHTROOM_LABELED
                .replace("   xmp:Label=\"パープル\"\n", "")
                .replace("   photoshop:LabelColor=\"purple\"\n", "")
        );
    }

    #[test]
    fn a_bridge_label_reads_and_gains_a_label_color() {
        let source = with_bridge_label("Red");
        assert_eq!(
            read_label(source.as_bytes(), &LabelNames::default()).unwrap(),
            Some("Red".to_string())
        );
        let out = labeled(&source, Some("Green"));
        assert_eq!(
            out,
            with_bridge_label("Green").replace(
                "xmp:CreatorTool=\"Bridge\"/>",
                &format!(
                    "xmp:CreatorTool=\"Bridge\" {PHOTOSHOP_DECL} photoshop:LabelColor=\"green\"/>"
                )
            )
        );
        assert_eq!(labeled(&out, None), bridge_declaring_photoshop());
    }

    #[test]
    fn reads_an_element_label_color() {
        let source = with_element_label("Red")
            .replace("<xmp:Label>Red</xmp:Label>", "<xmp:Label>赤</xmp:Label>");
        assert_eq!(
            read_label(source.as_bytes(), &LabelNames::default()).unwrap(),
            Some("Red".to_string())
        );
    }

    #[test]
    fn reads_an_attribute_label() {
        assert_eq!(
            read_label(
                with_attribute_label("Red").as_bytes(),
                &LabelNames::default()
            )
            .unwrap(),
            Some("Red".to_string())
        );
    }

    #[test]
    fn reads_an_element_label() {
        assert_eq!(
            read_label(
                with_element_label("Blue").as_bytes(),
                &LabelNames::default()
            )
            .unwrap(),
            Some("Blue".to_string())
        );
    }

    #[test]
    fn an_absent_or_empty_label_reads_as_none() {
        assert_eq!(
            read_label(BRIDGE.as_bytes(), &LabelNames::default()).unwrap(),
            None
        );
        assert_eq!(
            read_label(with_attribute_label("").as_bytes(), &LabelNames::default()).unwrap(),
            None
        );
        assert_eq!(
            read_label(with_element_label("").as_bytes(), &LabelNames::default()).unwrap(),
            None
        );
    }

    #[test]
    fn a_truncated_document_is_a_label_error() {
        let source = &BRIDGE[..BRIDGE.len() / 2];
        assert!(read_label(source.as_bytes(), &LabelNames::default()).is_err());
        assert!(write_label(Some(source.as_bytes()), Some("Red"), &LabelNames::default()).is_err());
    }

    #[test]
    fn sets_only_the_label_value() {
        let source = with_attribute_label("Red");
        assert_eq!(
            labeled(&source, Some("Green")),
            with_attribute_label("Green")
        );
        let source = with_element_label("Red");
        assert_eq!(
            labeled(&source, Some("Purple")),
            with_element_label("Purple")
        );
    }

    #[test]
    fn inserts_a_label_attribute_when_absent() {
        let out = labeled(NO_RATING, Some("Yellow"));
        assert_eq!(
            out,
            NO_RATING.replace(
                "xmp:CreatorTool=\"darktable\"/>",
                &format!("xmp:CreatorTool=\"darktable\" xmp:Label=\"Yellow\" {PHOTOSHOP_DECL} photoshop:LabelColor=\"yellow\"/>")
            )
        );
        assert_eq!(
            read_label(out.as_bytes(), &LabelNames::default()).unwrap(),
            Some("Yellow".to_string())
        );
    }

    #[test]
    fn clearing_removes_the_attribute() {
        assert_eq!(
            labeled(&with_attribute_label("Red"), None),
            BRIDGE.replace(
                "   xmp:Rating=\"3\"\n",
                &format!("   xmp:Rating=\"3\"\n   {PHOTOSHOP_DECL}\n")
            )
        );
        let inline = NO_RATING.replace(
            "xmp:CreatorTool=\"darktable\"/>",
            &format!("xmp:CreatorTool=\"darktable\" xmp:Label=\"Red\" {PHOTOSHOP_DECL} photoshop:LabelColor=\"red\"/>"),
        );
        assert_eq!(
            labeled(&inline, None),
            NO_RATING.replace(
                "\"darktable\"/>",
                &format!("\"darktable\" {PHOTOSHOP_DECL}/>")
            )
        );
    }

    #[test]
    fn clearing_removes_the_element_and_its_line() {
        assert_eq!(
            labeled(&with_element_label("Red"), None),
            with_element_label("Red")
                .replace("   <xmp:Label>Red</xmp:Label>\n", "")
                .replace("   <photoshop:LabelColor>red</photoshop:LabelColor>\n", "")
        );
        let shared = LIGHTROOM.replace(
            "<xmp:Rating>2</xmp:Rating>",
            "<xmp:Rating>2</xmp:Rating><xmp:Label>Red</xmp:Label>",
        );
        assert_eq!(labeled(&shared, None), LIGHTROOM);
    }

    #[test]
    fn clearing_without_a_label_is_a_no_op() {
        assert_eq!(labeled(BRIDGE, None), BRIDGE);
        assert_eq!(labeled(LIGHTROOM, None), LIGHTROOM);
    }

    #[test]
    fn a_fresh_label_template_and_no_sidecar_to_clear() {
        let bytes = write_label(None, Some("Red"), &LabelNames::default()).unwrap();
        assert_eq!(
            read_label(&bytes, &LabelNames::default()).unwrap(),
            Some("Red".to_string())
        );
        assert_eq!(read_rating(&bytes).unwrap(), None);
        assert!(write_label(None, None, &LabelNames::default()).is_err());
    }

    #[test]
    fn rating_and_label_keep_each_other() {
        let rated = patched(&with_element_label("Red"), Some(5));
        assert_eq!(
            read_label(rated.as_bytes(), &LabelNames::default()).unwrap(),
            Some("Red".to_string())
        );
        let both = labeled(&rated, Some("Blue"));
        assert_eq!(read_rating(both.as_bytes()).unwrap(), Some(5));
        assert_eq!(
            read_label(both.as_bytes(), &LabelNames::default()).unwrap(),
            Some("Blue".to_string())
        );

        let labeled_fresh = labeled(NO_RATING, Some("Green"));
        let rated = patched(&labeled_fresh, Some(2));
        assert_eq!(
            read_label(rated.as_bytes(), &LabelNames::default()).unwrap(),
            Some("Green".to_string())
        );
        assert_eq!(read_rating(rated.as_bytes()).unwrap(), Some(2));
    }

    #[test]
    fn an_element_label_with_an_entity_reference_reads_and_sets_whole() {
        let source = with_element_label("Red &amp; Blue").replace(
            "   <photoshop:LabelColor>red &amp; blue</photoshop:LabelColor>\n",
            "",
        );
        assert_eq!(
            read_label(source.as_bytes(), &LabelNames::default()).unwrap(),
            Some("Red &amp; Blue".to_string())
        );
        assert_eq!(
            labeled(&source, Some("Green")),
            with_element_label("Green")
                .replace(
                    "   <photoshop:LabelColor>green</photoshop:LabelColor>\n",
                    ""
                )
                .replace(
                    "crs:ToneCurveName2012=\"Medium Contrast\">",
                    "crs:ToneCurveName2012=\"Medium Contrast\" photoshop:LabelColor=\"green\">"
                )
        );
    }

    #[test]
    fn foreign_label_names_round_trip() {
        for label in ["Orange", "Rouge vif", "赤"] {
            let out = labeled(BRIDGE, Some(label));
            assert_eq!(
                read_label(out.as_bytes(), &LabelNames::default()).unwrap(),
                Some(label.to_string())
            );
            assert_eq!(labeled(&out, None), bridge_declaring_photoshop());
            let out = labeled(&with_element_label("Red"), Some(label));
            assert_eq!(out, with_element_label(label));
        }
    }
}
