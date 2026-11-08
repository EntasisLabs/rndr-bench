//! The fence catalog: language tag to typed block.

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::chart::{self, ChartProps};
use crate::props::*;
use crate::{kanban, parse, slides, styled};

/// Every fence language Liquid recognizes, including aliases
/// (`action_row`, `chip_group`, `procedure`).
pub const FENCE_LANGS: &[&str] = &[
    "checkbox",
    "input",
    "slider",
    "card",
    "carousel",
    "actions",
    "action_row",
    "callout",
    "section",
    "block",
    "chips",
    "chip_group",
    "media",
    "cite",
    "compare",
    "plan",
    "timeline",
    "shortlist",
    "decision",
    "brief",
    "dashboard",
    "chart",
    "report",
    "slides",
    "tabs",
    "steps",
    "recipe",
    "procedure",
    "accordion",
    "code",
    "tree",
    "kanban",
    "feed",
];

/// A parsed Liquid block. Serializes as `{"kind": …, "props": {…}}`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "props", rename_all = "lowercase")]
#[allow(missing_docs)]
pub enum LiquidBlock {
    Checkbox(CheckboxProps),
    Input(InputProps),
    Slider(SliderProps),
    Card(CardProps),
    Carousel(CarouselProps),
    Actions(ActionsProps),
    Callout(CalloutProps),
    Section(SectionProps),
    Block(StyledBlockProps),
    Chips(ChipsProps),
    Media(MediaProps),
    Cite(CiteProps),
    Compare(CompareProps),
    Plan(PlanProps),
    Timeline(TimelineProps),
    Shortlist(ShortlistProps),
    Decision(DecisionProps),
    Brief(BriefProps),
    Dashboard(DashboardProps),
    Chart(Box<ChartProps>),
    Report(ReportProps),
    Slides(SlidesProps),
    Tabs(TabsProps),
    Steps(StepsProps),
    Recipe(RecipeProps),
    Accordion(AccordionProps),
    Code(CodeProps),
    Tree(TreeProps),
    Kanban(KanbanProps),
    Feed(FeedProps),
}

impl LiquidBlock {
    /// Canonical kind name (aliases resolved).
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Checkbox(_) => "checkbox",
            Self::Input(_) => "input",
            Self::Slider(_) => "slider",
            Self::Card(_) => "card",
            Self::Carousel(_) => "carousel",
            Self::Actions(_) => "actions",
            Self::Callout(_) => "callout",
            Self::Section(_) => "section",
            Self::Block(_) => "block",
            Self::Chips(_) => "chips",
            Self::Media(_) => "media",
            Self::Cite(_) => "cite",
            Self::Compare(_) => "compare",
            Self::Plan(_) => "plan",
            Self::Timeline(_) => "timeline",
            Self::Shortlist(_) => "shortlist",
            Self::Decision(_) => "decision",
            Self::Brief(_) => "brief",
            Self::Dashboard(_) => "dashboard",
            Self::Chart(_) => "chart",
            Self::Report(_) => "report",
            Self::Slides(_) => "slides",
            Self::Tabs(_) => "tabs",
            Self::Steps(_) => "steps",
            Self::Recipe(_) => "recipe",
            Self::Accordion(_) => "accordion",
            Self::Code(_) => "code",
            Self::Tree(_) => "tree",
            Self::Kanban(_) => "kanban",
            Self::Feed(_) => "feed",
        }
    }
}

/// Remove reserved component metadata from the leading header only.
pub(crate) fn component_metadata(body: &str) -> (Option<String>, Cow<'_, str>) {
    if !body.starts_with("key:") && !body.contains("\nkey:") {
        return (None, Cow::Borrowed(body));
    }
    let mut id = None;
    let mut stripped = String::with_capacity(body.len());
    let mut in_header = true;
    let mut started = false;
    for line in body.split_inclusive('\n') {
        if line.trim() == "---" || (started && line.trim().is_empty()) {
            in_header = false;
        }
        started |= !line.trim().is_empty();
        if let Some(value) = line.strip_prefix("key:").filter(|_| in_header) {
            let value = value.trim();
            if id.is_none() && !value.is_empty() {
                id = Some(value.to_string());
            }
        } else {
            stripped.push_str(line);
        }
    }
    (id, Cow::Owned(stripped))
}

fn nonempty<T>(items: Vec<T>) -> Option<Vec<T>> {
    (!items.is_empty()).then_some(items)
}

/// Parses one fence body. `None` means the fence is not valid Liquid and
/// should render as ordinary fenced code.
/// Reserved unindented `key:` header lines are stripped before parsing. To obtain
/// those explicit component IDs, use [`crate::LiquidDocument::parse`].
///
/// ```
/// use rndr_liquid::{parse_fence, LiquidBlock};
/// let Some(LiquidBlock::Callout(c)) = parse_fence("callout", "tone: tip\nbody: Hi") else {
///     panic!()
/// };
/// assert_eq!((c.tone.as_deref(), c.body.as_str()), (Some("tip"), "Hi"));
/// assert_eq!(parse_fence("rust", "fn main() {}"), None);
/// ```
pub fn parse_fence(lang: &str, body: &str) -> Option<LiquidBlock> {
    use LiquidBlock as B;
    let (_, stripped) = component_metadata(body);
    let body = stripped.as_ref();
    Some(match lang.trim().to_lowercase().as_str() {
        "checkbox" => B::Checkbox(parse::checkbox(body)),
        "input" => B::Input(parse::input(body)),
        "slider" => B::Slider(parse::slider(body)?),
        "card" => B::Card(parse::card(body)?),
        "carousel" => B::Carousel(CarouselProps {
            items: nonempty(parse::carousel(body))?,
        }),
        "actions" | "action_row" => B::Actions(ActionsProps {
            actions: nonempty(parse::actions(body))?,
        }),
        "callout" => B::Callout(parse::callout(body)?),
        "section" => B::Section(parse::section(body)?),
        "block" => B::Block(styled::styled_block(body)?),
        "chips" | "chip_group" => B::Chips(ChipsProps {
            chips: nonempty(parse::chips(body))?,
        }),
        "media" => B::Media(parse::media(body)?),
        "feed" => B::Feed(parse::feed(body)?),
        "cite" => B::Cite(parse::cite(body)?),
        "compare" => B::Compare(parse::compare(body)?),
        "plan" => B::Plan(parse::plan(body)?),
        "timeline" => B::Timeline(parse::timeline(body)?),
        "shortlist" => B::Shortlist(parse::shortlist(body)?),
        "decision" => B::Decision(parse::decision(body)?),
        "brief" => B::Brief(parse::brief(body)?),
        "dashboard" => B::Dashboard(parse::dashboard(body)?),
        "report" => B::Report(parse::report(body)?),
        "slides" => B::Slides(slides::deck(body)?),
        "chart" => B::Chart(Box::new(chart::chart(body)?)),
        "tabs" => B::Tabs(parse::tabs(body)?),
        "steps" => B::Steps(parse::steps(body)?),
        "recipe" | "procedure" => B::Recipe(parse::recipe(body)?),
        "accordion" => B::Accordion(parse::accordion(body)?),
        "code" => B::Code(parse::code(body)?),
        "tree" => B::Tree(parse::tree(body)?),
        "kanban" => B::Kanban(KanbanProps {
            columns: kanban::columns(body),
        }),
        _ => return None,
    })
}
