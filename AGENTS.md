# Agent Instructions

## clibrowser — Web browsing via bash

`clibrowser` is available at `~/.local/bin/clibrowser`. Use it whenever you need to:
- Search the web
- Fetch and read web pages
- Extract structured data from HTML
- Fill and submit forms
- Monitor RSS feeds
- Crawl sites

### Essential commands

| Task | Command |
|------|---------|
| Search | `clibrowser --stealth search "query" -n 5 --json` |
| Fetch page | `clibrowser --stealth get "https://url" --json` |
| Read as markdown | `clibrowser markdown --max-length 3000` |
| Extract text | `clibrowser text --strip --json` |
| Get links | `clibrowser links --absolute --json` |
| Follow link | `clibrowser click "a.selector" --json` |
| CSS query | `clibrowser select "h1, .class" --json` |
| List forms | `clibrowser forms --json` |
| Fill form | `clibrowser fill "form" "name=value"` |
| Submit form | `clibrowser submit --json` |
| RSS feed | `clibrowser rss "https://feed" -n 10 --json` |
| Crawl site | `clibrowser crawl "https://url" --depth 2 --max-pages 10 --json` |
| Batch URLs | `echo "url1\nurl2" \| clibrowser pipe --title --json` |

### Tips
- Always use `--json` for parseable output
- Use `--stealth` for Cloudflare-protected sites
- Use `--session name` for parallel browsing sessions
- Exit codes: 0=success, 1=app error, 2=HTTP error, 3=network error
- Every JSON response has `"ok": true/false` at top level
