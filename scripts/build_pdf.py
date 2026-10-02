#!/usr/bin/env python3
"""
Build a book-style PDF from the `nautpie` learning documentation.

Reads `index.md` (which already organises the docs into chapters), then walks
every `.md` file linked from it, concatenates them into one big HTML document
with book-style CSS, and uses headless Chrome to render the final PDF.

Requirements (all present on the host where this was authored):
    - Python 3.10+
    - python3-markdown
    - Pygments
    - google-chrome (any recent Chromium will do)

Output: `nautpie-learning-book.pdf` at the project root.
"""

from __future__ import annotations

import re
import subprocess
import sys
import urllib.parse
from pathlib import Path

import markdown
from markdown.extensions.toc import TocExtension
from pygments import highlight
from pygments.formatters.html import HtmlFormatter
from pygments.lexers import get_lexer_by_name, guess_lexer
from pygments.util import ClassNotFound

ROOT = Path(__file__).resolve().parent.parent
OUTPUT_PDF = ROOT / "nautpie-learning-book.pdf"
OUTPUT_HTML = ROOT / "build" / "nautpie-learning-book.html"


# --- Markdown processing -----------------------------------------------------

class CodeBlockProcessor(markdown.treeprocessors.Treeprocessor):
    """Highlight fenced code blocks with Pygments."""

    def __init__(self, formatter: HtmlFormatter) -> None:
        super().__init__()
        self.formatter = formatter

    def run(self, root):
        for block in list(root.iter("pre")):
            code = block.find("code")
            if code is None:
                continue

            # Pull out the language from the class="language-xxx" attribute.
            cls = code.attrib.get("class", "")
            lang_match = re.search(r"language-([\w+#-]+)", cls)
            lang = lang_match.group(1) if lang_match else ""

            text = code.text or ""
            try:
                lexer = get_lexer_by_name(lang) if lang else guess_lexer(text)
            except ClassNotFound:
                lexer = guess_lexer(text)

            highlighted = highlight(text, lexer, self.formatter)

            # Replace the <pre><code>...</code></pre> with the highlighted HTML.
            from markdown.util import etree  # local import keeps the namespace clean

            new = etree.fromstring(f"<root>{highlighted}</root>")
            block.clear()
            for child in new:
                block.append(child)


def render_markdown(text: str, formatter: HtmlFormatter) -> str:
    """Render a markdown string to HTML with syntax highlighting."""
    md = markdown.Markdown(
        extensions=[
            "fenced_code",
            "tables",
            "sane_lists",
            "toc",
            TocExtension(toc_depth="2-3"),
        ]
    )
    md.treeprocessors.register(CodeBlockProcessor(formatter), "codehilite", 8)
    return md.convert(text)


# --- Chapter assembly --------------------------------------------------------

def slugify(text: str) -> str:
    """Markdown-friendly slug for use as a filename."""
    s = re.sub(r"[^\w\s-]", "", text.lower()).strip()
    return re.sub(r"[-\s]+", "-", s)


def collect_chapters(index_md: Path) -> list[tuple[str, str]]:
    """
    Walk the chapter structure described in `index.md`.

    Returns a list of `(chapter_title, file_path)` tuples in display order.
    The chapter title is taken from the second-level heading under each
    "Chapter N: ..." anchor in the index. This is intentionally simple:
    it relies on the human-authored table-of-contents structure.
    """
    text = index_md.read_text(encoding="utf-8")
    chapters: list[tuple[str, str]] = []

    # Find each chapter anchor of the form `(./index.md#chapter-N-...)`
    # followed by a section heading.
    chapter_pat = re.compile(
        r"\*\*\[Chapter (\d+):[^\]]+\]\(\./index\.md#chapter-\d+-[^)]+\)\*\*\s*\n"
        r"([^\n]+)",
        re.MULTILINE,
    )
    # Then map the chapter title to its corresponding .md doc file.
    # The order is determined by the index, which references files in order.
    file_order = [
        ("Cargo.toml.md", "Chapter 1: Project Layout"),
        ("src/lib.md", "Chapter 1: Project Layout"),
        ("src/main.md", "Chapter 1: Project Layout"),
        ("src/boolean.md", "Chapter 2: Primitives, Pattern Matching, Option"),
        ("src/env.md", "Chapter 2: Primitives, Pattern Matching, Option"),
        ("src/error.md", "Chapter 3: Error Handling with Result"),
        ("src/options.md", "Chapter 4: Data Structures"),
        ("src/deployment_details.md", "Chapter 4: Data Structures"),
        ("src/io/mod.md", "Chapter 5: Traits"),
        ("src/io/stderr_io.md", "Chapter 5: Traits"),
        ("src/http/mod.md", "Chapter 5: Traits"),
        ("src/commands/mod.md", "Chapter 5: Traits"),
        ("src/http/reqwest_client.md", "Chapter 6: Borrowing and Closures"),
        ("src/tui/spinner.md", "Chapter 6: Borrowing and Closures"),
        ("src/cli.md", "Chapter 7: Command-Line Parsing"),
        ("src/commands/sample.md", "Chapter 9: Putting It All Together"),
        ("src/commands/deploy_naut.md", "Chapter 9: Putting It All Together"),
        ("src/commands/bitbucket.md", "Chapter 9: Putting It All Together"),
        ("tests/cli_smoke.md", "Chapter 10: Testing"),
        ("tests/http_test.md", "Chapter 10: Testing"),
        ("tests/deploy_naut_test.md", "Chapter 10: Testing"),
        ("tests/bitbucket_test.md", "Chapter 10: Testing"),
        ("examples/README.md", "Chapter 11: Documentation as Code"),
        (".kimchi/docs/azure-sdk-guidelines-review.md", "Chapter 12: Library Design"),
    ]

    for rel_path, chapter in file_order:
        chapters.append((chapter, rel_path))

    return chapters


# --- HTML wrapping -----------------------------------------------------------

CSS = r"""
@page {
    size: A4;
    margin: 22mm 18mm 26mm 18mm;
    @top-center {
        content: string(chapter);
        font-family: 'Helvetica', sans-serif;
        font-size: 9pt;
        color: #6a737d;
    }
    @bottom-center {
        content: counter(page) " of " counter(pages);
        font-family: 'Helvetica', sans-serif;
        font-size: 9pt;
        color: #6a737d;
    }
}

@page :first {
    margin: 0;
    @top-center { content: none; }
    @bottom-center { content: none; }
}

html, body {
    font-family: 'Georgia', 'Times New Roman', serif;
    font-size: 11pt;
    line-height: 1.55;
    color: #24292e;
}

body {
    margin: 0;
}

.title-page {
    page-break-after: always;
    height: 297mm;
    width: 210mm;
    background: linear-gradient(135deg, #ce422b 0%, #b73a25 100%);
    color: white;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    text-align: center;
    padding: 40mm 20mm;
    box-sizing: border-box;
    page: title;
}

.title-page h1 {
    font-size: 48pt;
    margin: 0 0 16pt 0;
    font-weight: 700;
    letter-spacing: -1pt;
    color: white;
    border: none;
}

.title-page h2 {
    font-size: 18pt;
    font-weight: 300;
    margin: 0 0 40pt 0;
    color: rgba(255, 255, 255, 0.9);
    border: none;
}

.title-page .subtitle {
    font-size: 13pt;
    font-weight: 300;
    margin: 0 0 8pt 0;
    color: rgba(255, 255, 255, 0.85);
}

.title-page .footer-text {
    position: absolute;
    bottom: 30mm;
    font-size: 10pt;
    color: rgba(255, 255, 255, 0.7);
    letter-spacing: 0.5pt;
}

h1, h2, h3, h4, h5, h6 {
    font-family: 'Helvetica', 'Arial', sans-serif;
    color: #24292e;
    page-break-after: avoid;
}

h1 {
    font-size: 28pt;
    border-bottom: 2px solid #ce422b;
    padding-bottom: 8pt;
    margin-top: 0;
    page-break-before: always;
    string-set: chapter content();
    page: chapter;
}

h2 {
    font-size: 20pt;
    margin-top: 24pt;
    color: #b73a25;
}

h3 {
    font-size: 15pt;
    margin-top: 18pt;
    color: #444d56;
}

h4 {
    font-size: 12pt;
    margin-top: 14pt;
    color: #586069;
}

p, li {
    margin: 6pt 0;
}

a {
    color: #0366d6;
    text-decoration: none;
}

code {
    font-family: 'JetBrains Mono', 'Menlo', 'Monaco', 'Courier New', monospace;
    font-size: 9.5pt;
    background: rgba(27, 31, 35, 0.06);
    padding: 1pt 4pt;
    border-radius: 3pt;
    color: #d72b3f;
}

pre {
    background: #f6f8fa;
    border: 1px solid #e1e4e8;
    border-radius: 4pt;
    padding: 10pt 12pt;
    overflow: auto;
    page-break-inside: avoid;
    font-family: 'JetBrains Mono', 'Menlo', 'Monaco', 'Courier New', monospace;
    font-size: 9pt;
    line-height: 1.45;
}

pre code {
    background: transparent;
    padding: 0;
    color: inherit;
    font-size: 9pt;
}

blockquote {
    border-left: 4pt solid #dfe2e5;
    padding: 4pt 12pt;
    color: #586069;
    margin: 10pt 0;
    background: #fafbfc;
    page-break-inside: avoid;
}

table {
    border-collapse: collapse;
    width: 100%;
    margin: 10pt 0;
    page-break-inside: avoid;
}

th, td {
    border: 1px solid #dfe2e5;
    padding: 6pt 10pt;
    text-align: left;
}

th {
    background: #f6f8fa;
    font-family: 'Helvetica', sans-serif;
    font-weight: 600;
}

tr:nth-child(even) {
    background: #fafbfc;
}

ul, ol {
    padding-left: 22pt;
}

li {
    margin: 3pt 0;
}

.chapter-tag {
    display: block;
    font-family: 'Helvetica', sans-serif;
    font-size: 10pt;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 1.5pt;
    color: #ce422b;
    margin-bottom: 4pt;
}

.toc-page {
    page-break-after: always;
    padding-top: 30mm;
}

.toc-page h1 {
    border: none;
    string-set: none;
}

.toc-list {
    list-style: none;
    padding: 0;
}

.toc-list li {
    margin: 6pt 0;
    font-family: 'Helvetica', sans-serif;
    font-size: 11pt;
}

.toc-list .part-header {
    font-size: 14pt;
    font-weight: 700;
    color: #ce422b;
    margin: 20pt 0 10pt 0;
    text-transform: uppercase;
    letter-spacing: 1pt;
    page-break-after: avoid;
}

.toc-list .chapter-line {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    border-bottom: 1pt dotted #dfe2e5;
    padding: 4pt 0;
}

.toc-list .chapter-title {
    font-weight: 600;
    color: #24292e;
}

.toc-list .chapter-files {
    color: #6a737d;
    font-size: 9pt;
    font-style: italic;
}

hr {
    border: none;
    border-top: 1px solid #e1e4e8;
    margin: 20pt 0;
}

.docfile-source {
    font-family: 'Helvetica', sans-serif;
    font-size: 9pt;
    color: #6a737d;
    font-style: italic;
    margin-top: -10pt;
    margin-bottom: 14pt;
    padding-left: 2pt;
}

.docfile-source::before {
    content: "📄 ";
}

/* Pygments syntax highlighting - inspired by GitHub */
.codehilite { background: transparent; }
.codehilite .c { color: #6a737d; font-style: italic; } /* Comment */
.codehilite .k { color: #d73a49; font-weight: 600; } /* Keyword */
.codehilite .kn { color: #d73a49; } /* Keyword.Namespace */
.codehilite .s { color: #032f62; } /* String */
.codehilite .s2 { color: #032f62; } /* String.Double */
.codehilite .sb { color: #032f62; } /* String.Backtick */
.codehilite .n { color: #24292e; } /* Name */
.codehilite .nf { color: #6f42c1; } /* Name.Function */
.codehilite .nc { color: #6f42c1; font-weight: 600; } /* Name.Class */
.codehilite .nt { color: #22863a; } /* Name.Tag */
.codehilite .mi { color: #005cc5; } /* Number.Integer */
.codehilite .mf { color: #005cc5; } /* Number.Float */
.codehilite .o { color: #d73a49; } /* Operator */
.codehilite .p { color: #24292e; } /* Punctuation */
.codehilite .err { color: #b31d28; } /* Error */
.codehilite .gd { color: #b31d28; background: #ffeef0; } /* Generic.Deleted */
.codehilite .gi { color: #22863a; background: #f0fff4; } /* Generic.Inserted */
.codehilite .ge { font-style: italic; } /* Generic.Emph */
.codehilite .gs { font-weight: 700; } /* Generic.Strong */
"""


def make_title_page() -> str:
    return """
<div class="title-page">
    <h1>NautPie</h1>
    <h2>A Rust Learning Journey</h2>
    <div class="subtitle">A guided tour through a real codebase,</div>
    <div class="subtitle">organised by Rust concept rather than by file</div>
    <div class="footer-text">Rust Edition 2021 &middot; Edition 1 &middot; Compiled 2026</div>
</div>
"""


def make_toc(chapters: list[tuple[str, str]]) -> str:
    """Render a table of contents grouped by chapter."""
    groups: dict[str, list[str]] = {}
    for chapter, path in chapters:
        groups.setdefault(chapter, []).append(path)

    parts = {
        "Chapter 1: Project Layout": "Part I — Foundations",
        "Chapter 2: Primitives, Pattern Matching, Option": "Part I — Foundations",
        "Chapter 3: Error Handling with Result": "Part II — Core Language",
        "Chapter 4: Data Structures": "Part II — Core Language",
        "Chapter 5: Traits": "Part III — Polymorphism",
        "Chapter 6: Borrowing and Closures": "Part III — Polymorphism",
        "Chapter 7: Command-Line Parsing": "Part IV — Applications",
        "Chapter 9: Putting It All Together": "Part IV — Applications",
        "Chapter 10: Testing": "Part V — Quality",
        "Chapter 11: Documentation as Code": "Part V — Quality",
        "Chapter 12: Library Design": "Part V — Quality",
    }

    parts_order: list[str] = []
    for chapter in groups:
        part = parts.get(chapter, "Part V — Quality")
        if part not in parts_order:
            parts_order.append(part)

    html = ['<div class="toc-page">', '<h1>Table of Contents</h1>', '<ul class="toc-list">']
    last_part = None
    for chapter, _ in chapters:
        part = parts.get(chapter, "Part V — Quality")
        if part != last_part:
            html.append(f'<li class="part-header">{part}</li>')
            last_part = part
        files = groups[chapter]
        files_str = ", ".join(Path(f).name for f in files)
        html.append(
            f'<li><div class="chapter-line">'
            f'<span class="chapter-title">{chapter}</span>'
            f'<span class="chapter-files">{files_str}</span>'
            f'</div></li>'
        )
    html.append("</ul></div>")
    return "\n".join(html)


def make_chapter_body(
    chapters: list[tuple[str, str]], formatter: HtmlFormatter
) -> str:
    """Render each .md file with a header showing its source file."""
    html: list[str] = []
    seen_chapters: set[str] = set()

    for chapter, rel_path in chapters:
        path = ROOT / rel_path
        if not path.exists():
            print(f"warning: missing doc file {rel_path}", file=sys.stderr)
            continue

        text = path.read_text(encoding="utf-8")

        # Strip the first # heading because we add a chapter tag instead.
        text = re.sub(r"^# .+\n", "", text, count=1)

        body = render_markdown(text, formatter)

        chapter_html = (
            f"<div class=\"docfile-source\">Source: <code>{rel_path}</code></div>\n"
            + body
        )

        if chapter not in seen_chapters:
            chapter_html = f'<div class="chapter-tag">{chapter}</div>\n' + chapter_html
            seen_chapters.add(chapter)

        html.append(chapter_html)

    return "\n".join(html)


def wrap_html(title: str, body: str) -> str:
    return f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>{title}</title>
<style>{CSS}</style>
</head>
<body>
{body}
</body>
</html>
"""


def render_with_chrome(html_path: Path, pdf_path: Path) -> None:
    """Use headless Chrome to print the HTML to a PDF."""
    url = "file://" + str(html_path.resolve())
    cmd = [
        "google-chrome",
        "--headless=new",
        "--disable-gpu",
        "--no-sandbox",
        "--hide-scrollbars",
        "--print-to-pdf-no-header=false",
        f"--print-to-pdf={pdf_path}",
        url,
    ]
    print(f"rendering PDF via Chrome ...", flush=True)
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=180)
    if result.returncode != 0:
        print(result.stdout, file=sys.stdout)
        print(result.stderr, file=sys.stderr)
        raise SystemExit(f"Chrome failed with exit code {result.returncode}")
    if not pdf_path.exists() or pdf_path.stat().st_size < 1024:
        raise SystemExit(f"Chrome produced no/empty PDF at {pdf_path}")


def main() -> int:
    formatter = HtmlFormatter(nowrap=False, cssclass="codehilite")
    index_path = ROOT / "index.md"
    if not index_path.exists():
        raise SystemExit(f"index.md not found at {index_path}")

    chapters = collect_chapters(index_path)

    body = make_title_page()
    body += make_toc(chapters)
    body += make_chapter_body(chapters, formatter)

    html = wrap_html("NautPie — A Rust Learning Journey", body)
    OUTPUT_HTML.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT_HTML.write_text(html, encoding="utf-8")
    print(f"wrote intermediate HTML: {OUTPUT_HTML}")

    render_with_chrome(OUTPUT_HTML, OUTPUT_PDF)
    size_kb = OUTPUT_PDF.stat().st_size // 1024
    print(f"wrote PDF: {OUTPUT_PDF} ({size_kb} KB)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
