#!/usr/bin/env python3
"""Check local report links, source citations, IDs, and tab wiring without a browser."""

import argparse
from html.parser import HTMLParser
import json
from pathlib import Path
from urllib.parse import unquote, urlparse


class ReportParser(HTMLParser):
    def __init__(self):
        super().__init__()
        self.ids = []
        self.links = []
        self.tabs = []
        self.panels = []
        self.remote_assets = []

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if "id" in attrs:
            self.ids.append(attrs["id"])
        if tag == "a" and "href" in attrs:
            self.links.append(attrs["href"])
        if "data-source" in attrs:
            path, line = attrs["data-source"].rsplit(":", 1)
            self.links.append("https://github.com/btrkeks/lifestats/blob/06e4e70888b2fca062e3fdf2fd6fed60fd39d389/" + path + "#L" + line)
        if attrs.get("role") == "tab":
            self.tabs.append(attrs)
        if attrs.get("role") == "tabpanel":
            self.panels.append(attrs)
        if tag in ("script", "img", "link") and any(urlparse(attrs.get(key, "")).scheme in ("http", "https") for key in ("src", "href")):
            self.remote_assets.append(attrs)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", type=Path, default=Path(__file__).with_name("report.html"))
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    report = args.report.resolve()
    root = Path(__file__).resolve().parents[3]
    parsed = ReportParser()
    parsed.feed(report.read_text())
    errors = []
    if len(parsed.ids) != len(set(parsed.ids)):
        errors.append("Duplicate element IDs")
    if parsed.remote_assets:
        errors.append("Report depends on remote assets")
    panels = {p["id"]: p for p in parsed.panels}
    for tab in parsed.tabs:
        panel = panels.get(tab.get("aria-controls"))
        if panel is None or panel.get("aria-labelledby") != tab.get("id"):
            errors.append("Broken tab/panel relationship: " + tab.get("id", "missing"))
    citations = 0
    local_links = 0
    for link in parsed.links:
        url = urlparse(link)
        if link.startswith("#"):
            if link[1:] not in parsed.ids:
                errors.append("Missing anchor: " + link)
        elif url.netloc == "github.com" and "/lifestats/blob/" in url.path:
            parts = unquote(url.path).split("/blob/", 1)[1].split("/", 1)
            if len(parts) != 2:
                errors.append("Malformed source citation: " + link)
                continue
            path = root / parts[1]
            if not path.is_file():
                errors.append("Missing cited source: " + parts[1])
            elif url.fragment.startswith("L"):
                line = int(url.fragment[1:].split("-")[0])
                if not 1 <= line <= len(path.read_text().splitlines()):
                    errors.append("Invalid source line: " + link)
            citations += 1
        elif not url.scheme and url.path:
            if not (report.parent / unquote(url.path)).exists():
                errors.append("Missing local link: " + link)
            local_links += 1
    if not parsed.tabs:
        errors.append("No report navigation tabs")
    if not citations:
        errors.append("No source citations")
    result = {"source_citations_checked": citations, "local_links_checked": local_links,
              "tabs_checked": len(parsed.tabs), "errors": errors,
              "scope": "Static artifact checks. Browser layout and interaction are verified separately with html_preview."}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result))
    if errors:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
