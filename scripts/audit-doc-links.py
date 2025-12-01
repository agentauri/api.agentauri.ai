#!/usr/bin/env python3
"""
Documentation Link Audit Script
Checks all internal links in markdown files for broken references
"""

import os
import re
from pathlib import Path
from urllib.parse import urlparse

PROJECT_ROOT = Path("/Users/matteoscurati/work/api.8004.dev")

def is_external_link(link):
    """Check if link is external (http/https/mailto)"""
    return link.startswith(('http://', 'https://', 'mailto:'))

def is_anchor_only(link):
    """Check if link is anchor only (#section)"""
    return link.startswith('#')

def extract_links(content):
    """Extract all markdown links from content"""
    # Pattern: [text](link)
    pattern = r'\[([^\]]+)\]\(([^)]+)\)'
    return [(match.group(1), match.group(2)) for match in re.finditer(pattern, content)]

def resolve_link(source_file, link):
    """Resolve relative link to absolute path"""
    # Remove anchor
    link_path = link.split('#')[0]

    if not link_path:  # Only anchor
        return None

    source_dir = source_file.parent

    # Resolve relative path
    if link_path.startswith('/'):
        # Absolute from project root
        target = PROJECT_ROOT / link_path.lstrip('/')
    else:
        # Relative to current file
        target = (source_dir / link_path).resolve()

    return target

def audit_documentation():
    """Audit all markdown files for broken links"""
    print("🔍 Documentation Link Audit")
    print("=" * 60)
    print()

    total_files = 0
    total_links = 0
    broken_links = 0
    broken_details = []

    # Find all markdown files in docs/ and root
    patterns = [
        PROJECT_ROOT / "docs" / "**" / "*.md",
        PROJECT_ROOT / "*.md",
    ]

    md_files = []
    for pattern in patterns:
        md_files.extend(PROJECT_ROOT.glob(str(pattern.relative_to(PROJECT_ROOT))))

    md_files = sorted(set(md_files))

    for md_file in md_files:
        if not md_file.is_file():
            continue

        total_files += 1

        try:
            content = md_file.read_text(encoding='utf-8')
        except Exception as e:
            print(f"⚠️  Could not read {md_file.relative_to(PROJECT_ROOT)}: {e}")
            continue

        links = extract_links(content)

        for text, link in links:
            # Skip external links
            if is_external_link(link):
                continue

            # Skip anchor-only links
            if is_anchor_only(link):
                continue

            total_links += 1

            # Resolve link
            target = resolve_link(md_file, link)

            if target is None:
                continue

            # Check if target exists
            if not target.exists():
                broken_links += 1
                broken_details.append({
                    'source': md_file.relative_to(PROJECT_ROOT),
                    'link': link,
                    'target': target.relative_to(PROJECT_ROOT) if target.is_relative_to(PROJECT_ROOT) else target,
                    'text': text
                })

    # Print broken links
    if broken_details:
        print("❌ BROKEN LINKS FOUND:\n")
        for detail in broken_details:
            print(f"  File: {detail['source']}")
            print(f"  Link: [{detail['text']}]({detail['link']})")
            print(f"  Expected: {detail['target']}")
            print()

    # Print summary
    print("=" * 60)
    print("📊 Summary:")
    print(f"  Files scanned: {total_files}")
    print(f"  Links checked: {total_links}")
    print(f"  Broken links: {broken_links}")
    print()

    if broken_links == 0:
        print("✅ All links are valid!")
        return 0
    else:
        print(f"⚠️  Found {broken_links} broken link(s)")
        return 1

if __name__ == "__main__":
    exit(audit_documentation())
