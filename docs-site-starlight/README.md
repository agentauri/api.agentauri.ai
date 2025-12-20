# AgentAuri Public Documentation Site

Public-facing documentation for the AgentAuri API, built with [Starlight](https://starlight.astro.build/).

**Live URL**: https://docs.agentauri.ai

## Content Structure

```
src/content/docs/
├── index.mdx                    # Homepage
├── getting-started/
│   ├── quickstart.md            # 5-minute quickstart guide
│   ├── authentication.md        # JWT and API key auth
│   └── api-keys.md              # API key management
├── concepts/
│   ├── triggers.md              # Trigger system overview
│   ├── actions.md               # Available action types
│   └── events.md                # ERC-8004 events
└── guides/
    ├── webhook-integration.md   # REST webhook setup
    └── telegram-notifications.md # Telegram bot setup
```

## Local Development

```bash
cd docs-site-starlight
pnpm install
pnpm dev          # http://localhost:4321
```

## Deployment

The site is deployed via GitHub Actions to AWS S3 + CloudFront on push to main.

See `.github/workflows/docs.yml` for deployment configuration.

## Adding New Content

1. Create a `.md` or `.mdx` file in `src/content/docs/`
2. Add frontmatter with `title` and `description`
3. Optionally set `sidebar.order` for navigation ordering

Example:
```markdown
---
title: My New Guide
description: A guide to something useful
sidebar:
  order: 5
---

Content goes here...
```
