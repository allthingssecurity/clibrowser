# Agent Instructions

## clibrowser — Web browsing via bash

`clibrowser` is available at `~/.local/bin/clibrowser`. Use it whenever you need to:
- Search the web
- Fetch and read web pages
- Extract structured data from HTML
- Fill and submit forms
- Monitor RSS feeds
- Crawl sites

### IMPORTANT: Do NOT pipe commands together

Commands are **sequential, not piped**. Each command reads from the session state (cached page), not stdin.

```bash
# WRONG — do not pipe:
clibrowser get "https://example.com" --json | clibrowser markdown

# CORRECT — run sequentially:
clibrowser --stealth get "https://example.com"
clibrowser markdown --max-length 3000
```

The `get` command fetches the page and caches it in the session. Then `markdown`, `text`, `select`, `links`, `tables`, and other extraction commands read from that cache.

### Essential commands

| Task | Command |
|------|---------|
| Search | `clibrowser --stealth search "query" -n 5 --json` |
| Search & go to first result | `clibrowser --stealth search "query" --lucky` |
| Fetch page | `clibrowser --stealth get "https://url"` |
| Read as markdown | `clibrowser markdown --max-length 3000` |
| Extract text | `clibrowser text --strip` |
| Get links | `clibrowser links --absolute --json` |
| Follow link | `clibrowser click "a.selector" --json` |
| CSS query | `clibrowser select "h1, .class" --json` |
| List forms | `clibrowser forms --json` |
| Fill form | `clibrowser fill "form" "name=value"` |
| Submit form | `clibrowser submit --json` |
| RSS feed | `clibrowser rss "https://feed" -n 10 --json` |
| Crawl site | `clibrowser crawl "https://url" --depth 2 --max-pages 10 --json` |
| Sitemap | `clibrowser sitemap "https://site.com" --filter "docs" --json` |
| Batch URLs | `echo "url1\nurl2" \| clibrowser pipe --title --json` |

### Typical workflow

```bash
# Step 1: Fetch the page
clibrowser --stealth get "https://example.com/article"

# Step 2: Read it (pick ONE of these)
clibrowser markdown --max-length 5000          # as markdown
clibrowser text --strip --max-length 3000      # as plain text
clibrowser select "h1, h2, h3" --json          # headings only
clibrowser links --absolute --json             # all links
clibrowser tables --json                       # all tables

# Step 3: Navigate deeper
clibrowser click "a.next-page"                 # follow a link
clibrowser markdown --max-length 5000          # read the new page
```

### Tips
- Always use `--json` when you need to parse the output programmatically
- Use `--stealth` for sites with bot detection (Cloudflare, etc.)
- Use `--session name` for parallel browsing sessions
- Exit codes: 0=success, 1=app error, 2=HTTP error, 3=network error
- Every JSON response has `"ok": true/false` at top level
- `get` saves the page; `text`/`markdown`/`select`/`links`/`tables` read from it
