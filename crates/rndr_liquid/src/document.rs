//! Whole-document preprocessing: finds Liquid fences (innermost first),
//! shields documentation fences, unwraps mistaken prose fences and
//! normalizes `{{icon:…}}` shortcodes.

use crate::block::{parse_fence, LiquidBlock, FENCE_LANGS};
use crate::icons::normalize_icon_id;
use crate::kv::re;

/// Delimiter around a block reference in [`LiquidDocument::markdown`].
pub const BLOCK_TOKEN_DELIMITER: char = '\u{E001}';
const DOC_SLOT: char = '\u{E000}';

const PROSE_MISTAKEN_LANGS: &[&str] = &[
    "code",
    "text",
    "plaintext",
    "plain",
    "markdown",
    "md",
    "output",
    "response",
    "answer",
    "txt",
];

re!(SOURCE_WHISPER, r"(?mi)^\|\s*Source:\s*(.+?)\s*\|\s*$");
re!(
    CODE_LEAD,
    r#"(?m)^(import |export |from ['"]|const |let |var |function |class |def |package |using |#include|<\?php|#!/)"#
);
re!(CODEY_LINE, r"^\s*[{}\]);]|^\s*//|^\s*/\*|=>|:=|;\s*$");
re!(
    EMPHASIS,
    r"\*\*[^*]+\*\*|(^|[^*])\*[^*\n]+\*([^*]|$)|_[^_\n]+_"
);
re!(SENTENCE_END, r#"[.!?]["'”’)]?\s*$"#);
re!(SENTENCE_BREAK, r#"[.!?][\s"'”’]"#);
re!(FENCE_OPEN, r"^(```+)([a-zA-Z0-9_-]*)[ \t]*$");
re!(FENCE_CLOSE, r"^(```+)[ \t]*$");
re!(BODY_HAS_OPENER, r"(?m)^```+[a-zA-Z0-9_-]*[ \t]*$");
re!(ICON, r"\{\{icon:([a-zA-Z0-9_-]+)\}\}");
re!(INNER_OPEN, r"^```([a-zA-Z0-9_-]*)[ \t]*$");
re!(INNER_CLOSE, r"^```[ \t]*$");

/// Markdown with Liquid fences replaced by block references.
///
/// Each parsed block is replaced by its own line, `\u{E001}N\u{E001}`, where
/// `N` indexes [`blocks`](Self::blocks). Block bodies that hold Markdown
/// (reports, tabs, sections, …) may themselves contain such references to
/// nested blocks; [`segments`] splits any such text.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LiquidDocument {
    /// Markdown with block references.
    pub markdown: String,
    /// Parsed blocks in creation order (innermost fences first).
    pub blocks: Vec<LiquidBlock>,
}

/// Identity information for one parsed block.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlockMetadata {
    /// Explicit `id:` field, trimmed and case-sensitive. Empty IDs are absent.
    /// IDs are reserved, unindented header fields (before the first `---`).
    /// They are removed before props parsing except legacy `feed` and `block`
    /// props, where `id` already has public domain meaning and is retained.
    /// Hosts must reject duplicates across the entire document.
    pub explicit_id: Option<String>,
}

/// A parsed document together with reconciliation information.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParsedLiquidDocument {
    /// The ordinary, backwards-compatible document representation.
    pub document: LiquidDocument,
    /// One entry per `document.blocks` element, in the same innermost-first order.
    pub metadata: Vec<BlockMetadata>,
    /// An open Liquid fence or possible streamed Liquid opener was detected.
    /// Hosts may retain the last accepted UI until a complete update arrives.
    /// Ordinary code/documentation fences are opaque to this check.
    pub incomplete: bool,
}

/// A run of Markdown or a block reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Segment<'a> {
    /// Plain Markdown.
    Markdown(&'a str),
    /// Index into [`LiquidDocument::blocks`].
    Block(usize),
}

impl LiquidDocument {
    /// Preprocesses `source`.
    pub fn parse(source: &str) -> Self {
        Self::parse_with_metadata(source).document
    }

    /// Parses source and returns stable explicit identities and streaming status.
    ///
    /// Unindented header `id:` lines are reserved metadata, not component content.
    /// Legacy `feed` and styled `block` retain the field in props as well.
    /// Metadata includes nested blocks and aligns exactly with `blocks`.
    /// Completeness is syntactic, not a guarantee that every props payload is valid.
    pub fn parse_with_metadata(source: &str) -> ParsedLiquidDocument {
        let mut blocks = Vec::new();
        let mut metadata = Vec::new();
        let markdown = preprocess(source, &mut blocks, &mut metadata);
        ParsedLiquidDocument {
            document: Self { markdown, blocks },
            metadata,
            incomplete: has_incomplete_liquid(source),
        }
    }

    /// Block at `index`.
    pub fn block(&self, index: usize) -> Option<&LiquidBlock> {
        self.blocks.get(index)
    }

    /// Top-level segments of [`markdown`](Self::markdown).
    pub fn segments(&self) -> Vec<Segment<'_>> {
        segments(&self.markdown)
    }
}

/// The reference text for block `index`.
pub fn block_token(index: usize) -> String {
    format!("{BLOCK_TOKEN_DELIMITER}{index}{BLOCK_TOKEN_DELIMITER}")
}

/// Splits text into Markdown runs and block references.
pub fn segments(text: &str) -> Vec<Segment<'_>> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(BLOCK_TOKEN_DELIMITER) {
        let after = &rest[start + BLOCK_TOKEN_DELIMITER.len_utf8()..];
        let Some(end) = after.find(BLOCK_TOKEN_DELIMITER) else {
            break;
        };
        let Ok(index) = after[..end].parse::<usize>() else {
            out.push(Segment::Markdown(
                &rest[..start + BLOCK_TOKEN_DELIMITER.len_utf8()],
            ));
            rest = after;
            continue;
        };
        if start > 0 {
            out.push(Segment::Markdown(&rest[..start]));
        }
        out.push(Segment::Block(index));
        rest = &after[end + BLOCK_TOKEN_DELIMITER.len_utf8()..];
    }
    if !rest.is_empty() {
        out.push(Segment::Markdown(rest));
    }
    out
}

/// True when a fenced body reads as narrative prose rather than source code.
pub fn looks_like_prose(body: &str) -> bool {
    let normalized = body.replace("\r\n", "\n");
    let t = normalized.trim();
    let len = t.encode_utf16().count();
    if len < 20 || t.contains("```") || CODE_LEAD.is_match(t) {
        return false;
    }
    if t.split('\n').filter(|l| CODEY_LINE.is_match(l)).count() >= 3 {
        return false;
    }
    EMPHASIS.is_match(t)
        || (SENTENCE_END.is_match(t) && len >= 40)
        || (SENTENCE_BREAK.is_match(t) && len > 48)
}

fn extract_id(body: &str) -> (String, Option<String>) {
    let mut explicit_id = None;
    let mut header = true;
    let kept = body
        .split('\n')
        .filter(|line| {
            if line.trim() == "---" {
                header = false;
            }
            if let Some(value) = line.strip_prefix("id:").filter(|_| header) {
                if explicit_id.is_none() && !value.trim().is_empty() {
                    explicit_id = Some(value.trim().to_string());
                }
                false
            } else {
                true
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    (kept, explicit_id)
}

fn has_incomplete_liquid(source: &str) -> bool {
    // Ordinary fences are opaque. Only Liquid containers support nested fences.
    // Keep delimiter lengths so a documentation fence containing ``` examples
    // cannot accidentally open a live component or hide an unfinished one.
    let normalized = source.replace("\r\n", "\n");
    let shielded = shield_opaque_fences(&normalized, &mut Vec::new());
    let mut stack: Vec<(usize, bool)> = Vec::new();
    for line in shielded.split('\n') {
        if let Some(&(ticks, liquid)) = stack.last() {
            if fence_close(line, ticks) {
                stack.pop();
                continue;
            }
            if !liquid {
                continue;
            }
        }
        if let Some((ticks, lang)) = fence_open(line) {
            let liquid =
                lang.is_empty() || FENCE_LANGS.iter().any(|known| known.starts_with(&lang));
            stack.push((ticks, liquid));
        }
    }
    if stack.iter().any(|(_, liquid)| *liquid) {
        return true;
    }
    // Detect a final partial language tag (e.g. ```check) or delimiter, but
    // never interpret apparent Liquid syntax inside a normal code fence.
    let tail = shielded
        .trim_end_matches('\n')
        .rsplit('\n')
        .next()
        .unwrap_or("");
    if tail == "`" || tail == "``" {
        return stack.is_empty();
    }
    if let Some((_, lang)) = fence_open(tail) {
        return !lang.is_empty()
            && stack.len() == 1
            && FENCE_LANGS.iter().any(|known| known.starts_with(&lang));
    }
    false
}

fn is_liquid_lang(lang: &str) -> bool {
    FENCE_LANGS.contains(&lang)
}

fn fence_open(line: &str) -> Option<(usize, String)> {
    let caps = FENCE_OPEN.captures(line)?;
    Some((caps[1].len(), caps[2].to_lowercase()))
}

fn fence_close(line: &str, ticks: usize) -> bool {
    FENCE_CLOSE
        .captures(line)
        .is_some_and(|c| c[1].len() >= ticks)
}

struct TopFence {
    open: usize,
    close: usize,
    lang: String,
    body: String,
}

fn top_level_fences(lines: &[&str]) -> Vec<TopFence> {
    let mut fences = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let Some((ticks, lang)) = fence_open(lines[i]) else {
            i += 1;
            continue;
        };
        let mut depth = 1;
        let mut j = i + 1;
        while j < lines.len() && depth > 0 {
            if let Some((t, l)) = fence_open(lines[j]) {
                if t >= ticks && !l.is_empty() {
                    depth += 1;
                    j += 1;
                    continue;
                }
            }
            if fence_close(lines[j], ticks) {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            j += 1;
        }
        if j >= lines.len() || depth != 0 {
            break;
        }
        let body = lines[i + 1..j].join("\n");
        let mut close = j;
        if !is_liquid_lang(&lang)
            && BODY_HAS_OPENER.is_match(&body)
            && j + 1 < lines.len()
            && fence_close(lines[j + 1], ticks)
        {
            close = j + 1;
        }
        fences.push(TopFence {
            open: i,
            close,
            lang,
            body,
        });
        i = close + 1;
    }
    fences
}

// The legacy documentation pass operates at top level. Shield ordinary code
// nested inside Liquid as well, and tilde fences (which are never live Liquid).
// Unfinished ordinary code is opaque through EOF, not an invitation to mount
// apparent controls from an example being streamed inside it.
fn shield_opaque_fences(source: &str, slots: &mut Vec<String>) -> String {
    let lines: Vec<&str> = source.split('\n').collect();
    let mut out = Vec::new();
    let mut liquid_stack = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if liquid_stack
            .last()
            .is_some_and(|ticks| fence_close(line, *ticks))
        {
            liquid_stack.pop();
            out.push(line.to_string());
            i += 1;
            continue;
        }
        let tilde_ticks = line.chars().take_while(|c| *c == '~').count();
        let opener = if tilde_ticks >= 3 {
            Some((tilde_ticks, String::new(), '~'))
        } else {
            fence_open(line).map(|(ticks, lang)| (ticks, lang, '`'))
        };
        let Some((ticks, lang, marker)) = opener else {
            out.push(line.to_string());
            i += 1;
            continue;
        };
        let potential_liquid = marker == '`'
            && (lang.is_empty() || FENCE_LANGS.iter().any(|known| known.starts_with(&lang)));
        if potential_liquid {
            liquid_stack.push(ticks);
            out.push(line.to_string());
            i += 1;
            continue;
        }
        let close = (i + 1..lines.len()).find(|&j| {
            let n = lines[j].chars().take_while(|c| *c == marker).count();
            n >= ticks && lines[j][n..].trim().is_empty()
        });
        let end = close.map_or(lines.len(), |j| j + 1);
        let nested = !liquid_stack.is_empty();
        let body = lines[i + 1..close.unwrap_or(lines.len())].join("\n");
        if marker == '~' || nested || close.is_none() || BODY_HAS_OPENER.is_match(&body) {
            // Keep legacy trailing-prose repair for plain prose, but not for
            // a fenced code example containing apparent Liquid openers.
            let repairable = close.is_none()
                && !nested
                && marker == '`'
                && !BODY_HAS_OPENER.is_match(&body)
                && (PROSE_MISTAKEN_LANGS.contains(&lang.as_str()) || looks_like_prose(&body));
            if !repairable {
                let token = format!("{DOC_SLOT}{}{DOC_SLOT}", slots.len());
                slots.push(lines[i..end].join("\n"));
                out.push(token);
                i = end;
                continue;
            }
        }
        // Complete top-level ordinary fences retain legacy prose heuristics.
        out.extend(lines[i..end].iter().map(|line| line.to_string()));
        i = end;
    }
    out.join("\n")
}

fn shield_documentation_fences(source: &str, slots: &mut Vec<String>) -> String {
    let lines: Vec<&str> = source.split('\n').collect();
    let fences = top_level_fences(&lines);
    if fences.is_empty() {
        return source.to_string();
    }
    let mut out: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    for fence in fences.iter().rev() {
        if is_liquid_lang(&fence.lang) {
            continue;
        }
        let range = fence.open..fence.close + 1;
        let mistaken = (fence.lang.is_empty()
            || PROSE_MISTAKEN_LANGS.contains(&fence.lang.as_str()))
            && !BODY_HAS_OPENER.is_match(&fence.body)
            && looks_like_prose(&fence.body);
        if mistaken {
            let body: Vec<String> = if fence.body.is_empty() {
                Vec::new()
            } else {
                fence.body.split('\n').map(String::from).collect()
            };
            out.splice(range, body);
            continue;
        }
        let token = format!("{DOC_SLOT}{}{DOC_SLOT}", slots.len());
        slots.push(lines[range.clone()].join("\n"));
        out.splice(range, [token]);
    }
    out.join("\n")
}

fn restore_slots(source: &str, slots: &[String]) -> String {
    if slots.is_empty() {
        return source.to_string();
    }
    let mut out = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(start) = rest.find(DOC_SLOT) {
        let after = &rest[start + DOC_SLOT.len_utf8()..];
        let slot = after
            .find(DOC_SLOT)
            .and_then(|end| Some((end, after[..end].parse::<usize>().ok()?)));
        match slot {
            Some((end, index)) => {
                out.push_str(&rest[..start]);
                out.push_str(slots.get(index).map_or("", String::as_str));
                rest = &after[end + DOC_SLOT.len_utf8()..];
            }
            None => {
                out.push_str(&rest[..start + DOC_SLOT.len_utf8()]);
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Replacement text for one innermost fence, or `None` to keep it.
fn replace_fence(
    lang: &str,
    body: &str,
    blocks: &mut Vec<LiquidBlock>,
    metadata: &mut Vec<BlockMetadata>,
    slots: &[String],
) -> Option<String> {
    let lang = lang.trim().to_lowercase();
    let (without_id, explicit_id) = extract_id(body);
    // These legacy props already assign meaning to id. Preserve that public
    // contract while also exposing it as block identity metadata.
    let body = if lang == "feed" || lang == "block" {
        body
    } else {
        without_id.as_str()
    };
    let restored = restore_slots(body, slots);
    let body = restored.as_str();
    let mut emit = |block: LiquidBlock| {
        blocks.push(block);
        metadata.push(BlockMetadata {
            explicit_id: explicit_id.clone(),
        });
        Some(format!("\n{}\n", block_token(blocks.len() - 1)))
    };
    if lang == "code" {
        if let Some(block) = parse_fence("code", body) {
            return emit(block);
        }
        return looks_like_prose(body).then(|| format!("\n{}\n", body.trim()));
    }
    if (lang.is_empty() || PROSE_MISTAKEN_LANGS.contains(&lang.as_str())) && looks_like_prose(body)
    {
        return Some(format!("\n{}\n", body.trim()));
    }
    parse_fence(&lang, body).and_then(emit)
}

/// One pass over innermost fences (no line inside starts with ```).
/// Returns the first replacement made, as `(start, end, text)` byte offsets.
fn first_innermost(
    text: &str,
    blocks: &mut Vec<LiquidBlock>,
    metadata: &mut Vec<BlockMetadata>,
    slots: &[String],
) -> Option<(usize, usize, String)> {
    let mut starts = vec![0];
    starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
    let line_at = |n: usize| {
        let s = starts[n];
        let e = starts.get(n + 1).map_or(text.len(), |&e| e - 1);
        &text[s..e]
    };
    let mut n = 0;
    while n < starts.len() {
        let line = line_at(n);
        let Some(caps) = INNER_OPEN.captures(line).filter(|_| n + 1 < starts.len()) else {
            n += 1;
            continue;
        };
        let Some(close) = (n + 1..starts.len()).find(|&k| line_at(k).starts_with("```")) else {
            n += 1;
            continue;
        };
        if !INNER_CLOSE.is_match(line_at(close)) {
            n += 1;
            continue;
        }
        let body_start = starts[n + 1];
        let body_end = starts[close];
        let end = starts[close] + line_at(close).len();
        if let Some(rep) = replace_fence(
            &caps[1],
            &text[body_start..body_end],
            blocks,
            metadata,
            slots,
        ) {
            return Some((starts[n], end, rep));
        }
        n = close + 1;
    }
    None
}

fn process_nested(
    text: String,
    blocks: &mut Vec<LiquidBlock>,
    metadata: &mut Vec<BlockMetadata>,
    slots: &[String],
) -> String {
    let mut out = text;
    while let Some((start, end, rep)) = first_innermost(&out, blocks, metadata, slots) {
        out.replace_range(start..end, &rep);
    }
    out
}

fn repair_trailing_prose_fence(source: &str) -> String {
    let lines: Vec<&str> = source.split('\n').collect();
    let mut open: Option<(usize, usize, String)> = None;
    for (i, line) in lines.iter().enumerate() {
        match &open {
            None => {
                if let Some((ticks, lang)) = fence_open(line) {
                    open = Some((i, ticks, lang));
                }
            }
            Some((_, ticks, _)) => {
                if fence_close(line, *ticks) {
                    open = None;
                }
            }
        }
    }
    let Some((line, _, lang)) = open else {
        return source.to_string();
    };
    let after = lines[line + 1..].join("\n");
    if after.split('\n').any(|l| l.starts_with("```")) || is_liquid_lang(&lang) {
        return source.to_string();
    }
    if after.trim().is_empty() {
        return lines[..line].join("\n");
    }
    if lang.is_empty() || PROSE_MISTAKEN_LANGS.contains(&lang.as_str()) || looks_like_prose(&after)
    {
        let mut kept = lines[..line].to_vec();
        kept.extend_from_slice(&lines[line + 1..]);
        return kept.join("\n");
    }
    source.to_string()
}

fn normalize_icons(source: &str) -> String {
    let mut in_fence = false;
    source
        .split('\n')
        .map(|line| {
            if line.trim_start().starts_with("```") {
                in_fence = !in_fence;
                return line.to_string();
            }
            if in_fence {
                return line.to_string();
            }
            ICON.replace_all(line, |caps: &regex::Captures| {
                match normalize_icon_id(&caps[1]) {
                    Some(id) => format!("{{{{icon:{id}}}}}"),
                    None => String::new(),
                }
            })
            .into_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn preprocess(
    source: &str,
    blocks: &mut Vec<LiquidBlock>,
    metadata: &mut Vec<BlockMetadata>,
) -> String {
    let normalized = source.replace("\r\n", "\n");
    let out = SOURCE_WHISPER.replace_all(&normalized, |caps: &regex::Captures| {
        format!("*Source: {}*", caps[1].trim())
    });
    let mut slots = Vec::new();
    let out = shield_opaque_fences(&out, &mut slots);
    let out = shield_documentation_fences(&out, &mut slots);
    let out = process_nested(out, blocks, metadata, &slots);
    let out = restore_slots(&out, &slots);
    let out = repair_trailing_prose_fence(&out);
    normalize_icons(&out)
}
