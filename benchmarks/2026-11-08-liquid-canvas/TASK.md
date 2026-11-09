# Benchmark: rndr Production Integration — Stateful Liquid & Editable Canvas

## Objective
Make two existing rndr subsystems ready for direct host-application integration:
1. Liquid Markdown must support incremental document updates without unnecessarily resetting interactive component identity, state, or focus.
2. Canvas text objects must support meaningful interactive editing using rndr's existing text-editing and transaction infrastructure.
Deliver both capabilities in a **single runnable integration example** that demonstrates them operating within one host application.
This is a production-integration benchmark, not a request for additional showcase components.

## Starting instructions
Inspect the current repository and relevant architecture before implementing anything.
The previous review identified two gaps:
- Liquid source replacement recreated interactive state.
- Canvas text editing was limited to append/backspace behavior.
These findings may be outdated. Verify the current implementation and work from what actually exists.
Preserve existing architectural boundaries. Keep reusable behavior in rndr and application-specific authority in host adapters.
You may decompose the work, use subagents, and choose your own implementation strategy.

## Slice 1 — Update-safe Liquid reconciliation
Implement or complete a supported host-driven update contract for rendered Liquid documents.

### Required behavior
- Stable explicit IDs preserve logically unchanged components across document updates.
- Define deterministic fallback identity semantics for components without explicit IDs, including the limitations of inferred identity.
- Insertions, removals, and reordering have predictable reconciliation behavior.
- Duplicate IDs cannot silently alias unrelated component state.
- Component-type changes reset incompatible state without corrupting other components.
- Host-provided data updates must respect a documented controlled/uncontrolled state contract.
- Incomplete streaming input must not destroy the last valid interactive UI unnecessarily.
- Removed components release their state and owned subscriptions/resources.
- Intentional document replacement/reset remains possible.

### Acceptance scenario
Create a document containing a checkbox, text input, slider, and surrounding Markdown.
1. Modify the controls and focus the text input.
2. Insert and stream Markdown before and between the controls.
3. Update component data from the host.
4. Reorder components with explicit IDs.
5. Introduce incomplete Liquid syntax, then complete it.
6. Remove one component and change another's type.
7. Intentionally reset the document.
Unchanged components retain appropriate local state and focus. Host-controlled values follow the documented update policy. Removed and incompatible components are handled predictably without retaining stale resources.
Add regression tests for these transitions.

## Slice 2 — Interactive canvas text editing
Extend the existing canvas text editing path by reusing rndr's established text-editing primitives.
Do not create a separate text-editor implementation unless an unavoidable architectural limitation is demonstrated.

### Required behavior
- Enter and exit text-editing mode.
- Position the caret through pointer interaction.
- Select text with pointer and keyboard input.
- Navigate, insert, and delete text at arbitrary positions.
- Support clipboard copy, cut, and paste through the existing host/platform abstractions.
- Route focus and shortcuts correctly between text editing and canvas commands.
- Position text hit testing and editing UI correctly under canvas zoom, pan, and supported object transforms.
- Commit edits through the existing document/transaction model.
- Support undo/redo across both text edits and object manipulation.
- Reuse existing IME/composition infrastructure where available; document and separately verify native composition behavior rather than assuming unit-test coverage proves it.

### Acceptance scenario
Create a canvas containing a text object.
1. Enter editing mode.
2. Position the caret in the middle of the text.
3. Select and replace a substring.
4. Copy and paste a selection.
5. Change zoom and pan, then edit again.
6. Commit the text edit.
7. Move or transform the object.
8. Undo and redo the editing and manipulation sequence.
9. Exit editing and confirm that shortcuts return to canvas commands.
Where supported, exercise native composition input and record the actual result.
Tests should cover editor state, transactions, coordinate transformations, and relevant input routing.

## Integrated demonstration
Provide one runnable example, tentatively named `liquid_canvas_integration`, that exposes both slices within the same host application.
The example must demonstrate:
- A Liquid document updated by simulated host streaming.
- Stateful Liquid controls that survive appropriate updates.
- An editable canvas text object.
- A visible way to trigger both workflows.
- Observable state transitions, focus behavior, and undo/redo.
Use a mock host where needed. Do not introduce Medousa-specific services or daemon dependencies into rndr.
Keep the example useful as a reproducible integration fixture, not merely a visual showcase.

## Non-goals
Do not implement the full source-backed Live-document editor, hosted HTML/browser bridges, image asset import, broad chart/media parity, or a new general-purpose text engine.
Fix directly encountered defects when necessary for the two slices. Document unrelated gaps rather than expanding the project.

## Engineering constraints
- Follow existing repository conventions and architectural contracts.
- Preserve public API compatibility where practical; document unavoidable changes.
- Prefer independently reviewable commits for each slice.
- Do not bypass failing tests, hide unsupported behavior, or remove existing safeguards to satisfy acceptance.
- Do not claim real native interaction, accessibility, IME, or performance verification without appropriate evidence.
- Avoid speculative refactoring unrelated to the required capabilities.

## Required delivery
Provide:
1. The implemented changes, separated by slice.
2. Unit, regression, and integration tests with executable commands and results.
3. One runnable integrated example with clear instructions.
4. An explanation of new or updated public integration contracts.
5. A verification report distinguishing automated tests, rendered-image inspection, and actual native interactions.
6. Explicit remaining limitations, incomplete requirements, and known risks.
7. The final commit references and repository status.

## Completion rules
Prioritize correct, verified outcomes over the amount of code written.
If the available budget prevents full completion, leave the repository in a coherent, testable state and report exactly what remains.
Do not describe partially implemented behavior as production-ready.
**Success means a host application can update stateful Liquid content and edit canvas text without rebuilding the lifecycle or editing semantics itself.**
