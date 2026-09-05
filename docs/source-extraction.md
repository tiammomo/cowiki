# Source extraction and quality

File import returns a structured extraction report and stores it in the Source's
`cowiki_extraction` frontmatter alongside `source_hash` and `source_extraction_hash`. Markdown and Git retain
this evidence when a Space is cloned; SQLite remains a rebuildable index.

| Status | Meaning |
|---|---|
| `pass` | The available checks found no issue. This is not a guarantee of semantic fidelity. |
| `warn` | Readable content was imported, with limitations or incomplete coverage to review. |
| `fallback` | An alternative extractor recovered readable content. Diagnostics still require review. |
| `fail` | No Source is written. The batch continues, and the file remains available for retry. |

Reports contain the format, selected extractor and contract version, character
count, available expected/extracted unit counts, attempted extraction paths, and
actionable diagnostics. PPTX coverage counts slides; spreadsheet coverage counts
sheets. Empty units are reported because they can contain visual-only content.
DOCX native output is compared with document XML text; missing content falls back
to a formatting-flattened XML extraction. Empty or predominantly unreadable glyph
output fails, while partial damage is visible. These heuristics do not infer
correct PDF reading order, OCR accuracy, chart meaning or document completeness.

Inputs are copied into a private, size-bounded temporary snapshot before parsing.
The hash describes the exact bytes extracted, even if the original is later
edited. Existing input, archive expansion, entry-count and output-size limits
remain in force. Re-importing identical bytes with the same extracted text preserves the existing
Source and its human/Agent annotations. Improved extraction creates a separate,
deterministic Source; old Sources are not silently overwritten or rewritten to add metadata.

Structured text (JSON, YAML, CSV/TSV, XML and local HTML listings) is preserved in
Markdown code fences. Local HTML uses a `text` fence so it cannot become an
interactive HTML View. Web-page capture belongs to the separate URL adapter
(PR #144); this change does not replace it with a second fetch implementation.

## Optional local converters

In the desktop file-import tab, enable **Use installed local extraction tools**
for this import. No converter runs without that choice, and no Cloud service or
account is required. CoWiki does not install tools automatically.

- Images: Tesseract (`tesseract`). Install language data appropriate to your input;
  the current adapter uses the tool's default language.
- PDF recovery: Poppler (`pdftotext`, then `pdfinfo`/`pdftoppm`) and Tesseract.
- Legacy `.doc`: antiword.

The adapter launches fixed programs and argument arrays without a shell, with a
clean environment and closed stdin. Local extraction has a 90-second total budget
and a 32 MiB text limit. Scanned PDFs are limited to 20 pages; pages render at up
to 2,000 pixels on their longest edge. A missing tool, timeout or unreadable OCR
page produces an actionable failure; an incomplete OCR PDF is never presented as
a complete import. Files stay local. These tools run as ordinary local processes,
not inside a CoWiki operating-system sandbox.

OCR output and PDF layout still require human review. Paid/external parsing
providers are not selected implicitly; adding one needs a separate explicit
provider contract and privacy decision.

![Partial import with extraction warnings and an OCR retry option](images/source-quality.png)

## Verification

Run the normal web and Rust suites. Regression fixtures cover partial slide
coverage, DOCX nested text recovery, unreadable nonempty input, Unicode text,
structured-text fences, opt-in conversion, tool timeouts, batch failures and
portable quality metadata. No test requires an external model or network service.
