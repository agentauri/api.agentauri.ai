#!/bin/bash
# Documentation Link Audit Script
# Checks all internal links in markdown files for broken references

set -euo pipefail

PROJECT_ROOT="/Users/matteoscurati/work/api.agentauri.ai"
cd "$PROJECT_ROOT"

echo "🔍 Documentation Link Audit"
echo "================================"
echo ""

TOTAL_FILES=0
TOTAL_LINKS=0
BROKEN_LINKS=0

# Find all markdown files
while IFS= read -r file; do
    TOTAL_FILES=$((TOTAL_FILES + 1))

    # Extract markdown links: [text](path)
    # Look for relative links (starting with ./ or ../ or direct path)
    links=$(grep -oE '\[([^\]]+)\]\(([^)]+)\)' "$file" | grep -oE '\(([^)]+)\)' | tr -d '()' || true)

    if [ -z "$links" ]; then
        continue
    fi

    while IFS= read -r link; do
        # Skip external links (http://, https://, mailto:)
        if [[ "$link" =~ ^https?:// ]] || [[ "$link" =~ ^mailto: ]]; then
            continue
        fi

        # Skip anchors (e.g., #section)
        if [[ "$link" =~ ^# ]]; then
            continue
        fi

        TOTAL_LINKS=$((TOTAL_LINKS + 1))

        # Get directory of current file
        file_dir=$(dirname "$file")

        # Remove anchor from link (e.g., file.md#section -> file.md)
        link_path="${link%%#*}"

        # Resolve relative path
        if [[ "$link_path" == /* ]]; then
            # Absolute path
            target_path="$link_path"
        else
            # Relative path
            target_path=$(cd "$file_dir" && realpath --relative-to="$PROJECT_ROOT" "$link_path" 2>/dev/null || echo "$link_path")
        fi

        # Check if target exists
        if [ ! -e "$PROJECT_ROOT/$target_path" ] && [ ! -e "$file_dir/$link_path" ]; then
            BROKEN_LINKS=$((BROKEN_LINKS + 1))
            echo "❌ BROKEN: $file"
            echo "   Link: $link"
            echo "   Expected: $target_path"
            echo ""
        fi
    done <<< "$links"

done < <(find . -name "*.md" -type f | grep -E "^\./(docs|README|CLAUDE|CHANGELOG|CONTRIBUTING|SECURITY|DEPLOYMENT)" | sort)

echo "================================"
echo "📊 Summary:"
echo "  Files scanned: $TOTAL_FILES"
echo "  Links checked: $TOTAL_LINKS"
echo "  Broken links: $BROKEN_LINKS"
echo ""

if [ "$BROKEN_LINKS" -eq 0 ]; then
    echo "✅ All links are valid!"
    exit 0
else
    echo "⚠️  Found $BROKEN_LINKS broken link(s)"
    exit 1
fi
