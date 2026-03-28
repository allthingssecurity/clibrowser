# clibrowser

**Zero-dependency CLI browser for AI agents.** Single Rust binary, 19 commands, bash-native.

AI agents interact with tools via bash — but browsing the web has always required heavy dependencies (Playwright, Selenium, headless Chrome, Python/Node runtimes). `clibrowser` changes that: one 6MB binary, zero runtime dependencies, every browser action exposed as a simple CLI command with JSON output.

## Why clibrowser?

| | clibrowser | Playwright MCP | Browser Use | Firecrawl | agent-browser |
|---|---|---|---|---|---|
| **Binary size** | 6MB | ~400MB | ~500MB | Cloud | ~50MB |
| **Dependencies** | None | Node + Chromium | Python + Chromium | API key | Node + Chromium |
| **Setup time** | 0 seconds | Minutes | Minutes | Sign up | Minutes |
| **How agents call it** | `bash` | MCP/JSON-RPC | Python SDK | REST API | bash (+ daemon) |
| **Session persistence** | Yes | No | Limited | No | No |
| **Form interaction** | Yes | Yes | Yes | No | Limited |
| **Works offline** | Yes | Yes | Yes | No | Yes |

**clibrowser is the only tool where an agent can do this with zero setup:**

```bash
clibrowser search "NVIDIA earnings 2026" --lucky
clibrowser select "table.financials" --json
clibrowser click "a.next-quarter"
clibrowser forms --json
clibrowser fill "form" "email=agent@test.com"
clibrowser submit --json
```

## Install

```bash
# Build from source
cargo build --release
cp target/release/clibrowser ~/.local/bin/

# Or install directly
cargo install --path .
```

## Quick Start

```bash
# Fetch a page
clibrowser get "https://example.com"

# Extract text
clibrowser text --strip

# Get links
clibrowser links --absolute --json

# Search the web
clibrowser search "Anthropic Claude MCP" -n 5

# Search and go to first result
clibrowser --stealth search "MCP rug pull attack" --lucky

# Convert page to markdown
clibrowser markdown --max-length 2000

# Monitor RSS feeds
clibrowser rss "https://simonwillison.net/atom/everything/" -n 5 --filter "claude"
```

## All Commands

### Browse & Navigate

| Command | Description | Example |
|---------|-------------|---------|
| `get <url>` | Fetch any URL | `clibrowser get "https://example.com" --json` |
| `click <selector>` | Follow a link by CSS selector | `clibrowser click "a.next" --index 2` |
| `status` | Show current session state | `clibrowser status --json` |

### Extract Content

| Command | Description | Example |
|---------|-------------|---------|
| `text` | Extract text content | `clibrowser text --strip --max-length 500` |
| `select <css>` | CSS selector queries | `clibrowser select "h1, h2" --json` |
| `links` | Extract all links | `clibrowser links --absolute --filter "docs"` |
| `tables` | Extract table data | `clibrowser tables --index 0 --json` |
| `headers` | Show HTTP response headers | `clibrowser headers --name content-type` |
| `markdown` | Convert page to clean markdown | `clibrowser markdown --max-length 3000` |

### Interact with Forms

| Command | Description | Example |
|---------|-------------|---------|
| `forms` | List all forms and fields | `clibrowser forms --json` |
| `fill <selector> field=value...` | Fill form fields | `clibrowser fill "form" "user=agent" "pass=x"` |
| `submit` | Submit a form | `clibrowser submit --json` |
| `cookies` | List/set/clear cookies | `clibrowser cookies --json` |

### Discover Content

| Command | Description | Example |
|---------|-------------|---------|
| `search <query>` | Web search (DuckDuckGo/Google) | `clibrowser search "AI agents" -n 10` |
| `rss <url>` | Parse RSS/Atom feeds | `clibrowser rss "https://feed.url" -n 5 --since 7d` |
| `sitemap <url>` | Discover pages via sitemap.xml | `clibrowser sitemap "https://site.com" --filter "api"` |
| `crawl [url]` | Crawl links to specified depth | `clibrowser crawl "https://docs.site.com" --depth 2` |

### Batch & Session

| Command | Description | Example |
|---------|-------------|---------|
| `pipe` | Process URLs from stdin | `cat urls.txt \| clibrowser pipe --title --json` |
| `session list\|delete\|clear` | Manage named sessions | `clibrowser session list` |

## Global Flags

| Flag | Description |
|------|-------------|
| `--json` | Output structured JSON (every command supports this) |
| `--stealth` | Chrome-like headers + TLS for Cloudflare/bot bypass |
| `--session <name>` | Named session for parallel agent workflows |
| `--quiet` | Suppress non-essential output |

## How It Works for Agents

### Session Persistence

Every command reads and writes to a session directory (`~/.clibrowser/sessions/<name>/`). Between invocations, the session stores:

- **Cookies** — persist across requests, handle redirects
- **Current URL** — relative URLs resolve automatically
- **Page HTML** — cached for selector queries without re-fetching
- **Form fills** — `fill` stores data, `submit` uses it

This means an agent can do multi-step workflows across separate bash calls:

```bash
clibrowser get "https://site.com/login"         # fetch login page
clibrowser forms --json                          # discover form fields
clibrowser fill "form" "user=bot" "pass=secret"  # fill credentials
clibrowser submit --json                         # submit, follows redirect
clibrowser select ".dashboard-data" --json       # extract from logged-in page
```

### Named Sessions

Parallel agents can each use their own session:

```bash
# Agent 1 researches topic A
clibrowser --session research-a get "https://arxiv.org/..."
clibrowser --session research-a text --json

# Agent 2 researches topic B simultaneously
clibrowser --session research-b get "https://papers.org/..."
clibrowser --session research-b text --json
```

### JSON Output

Every command supports `--json` for structured, parseable output:

```json
$ clibrowser links --json
{
  "ok": true,
  "count": 3,
  "links": [
    {"index": 0, "href": "https://example.com/page1", "text": "Page 1"},
    {"index": 1, "href": "https://example.com/page2", "text": "Page 2"}
  ]
}
```

Every JSON response includes `"ok": true/false` at the top level — agents check one field.

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Application error (bad selector, no session) |
| 2 | HTTP error (4xx/5xx) |
| 3 | Network error (timeout, DNS) |

### Stealth Mode

`--stealth` adds Chrome-like headers to bypass bot detection:

- Full Chrome 131 User-Agent
- `sec-ch-ua`, `sec-fetch-*`, `accept-language` headers
- `rustls` TLS (better fingerprint than native-tls)
- Cloudflare challenge detection with auto-retry
- Referer header on follow-up requests

```bash
# Without stealth: minimal headers, often blocked
clibrowser get "https://protected-site.com"

# With stealth: looks like a real Chrome browser
clibrowser --stealth get "https://protected-site.com"
```

## Real-World Agent Workflows

### Newsletter Link Validation

```bash
# Extract URLs from a markdown newsletter and validate them
grep -oE 'https?://[^)>"]+' newsletter.md | \
  clibrowser --stealth pipe --title --strip --max-text 200 --continue-on-error --json
```

### Research Pipeline

```bash
# Search → navigate → extract → follow sub-links
clibrowser --stealth search "transformer architecture explained" --lucky
clibrowser markdown --max-length 5000 > research.md
clibrowser links --absolute --filter "paper" --json
```

### RSS Monitoring

```bash
# Check multiple feeds for recent AI news
clibrowser rss "https://www.latent.space/feed" -n 5 --since 7d
clibrowser rss "https://simonwillison.net/atom/everything/" -n 5 --filter "claude"
clibrowser rss "https://hnrss.org/frontpage" -n 10
```

### Site Mapping

```bash
# Discover all pages, then crawl interesting ones
clibrowser --stealth sitemap "https://docs.anthropic.com" --filter "api" --json
clibrowser --stealth crawl "https://docs.anthropic.com" --depth 2 --max-pages 20 --extract-text
```

### Deep Link Tree Exploration

```bash
# Crawl 2 levels deep with content extraction
clibrowser --stealth crawl "https://www.theregister.com/2026/03/23/google_dark_web_ai/" \
  --depth 2 --max-pages 8 --extract-text --text-max-length 200

# Or manually navigate the tree
clibrowser --stealth get "https://en.wikipedia.org/wiki/Artificial_intelligence"
clibrowser click "#mw-content-text p a[href]" --index 0   # follow first link
clibrowser click "#mw-content-text p a[href]" --index 0   # go deeper
clibrowser click "#mw-content-text p a[href]" --index 0   # and deeper
```

## Architecture

```
src/
  main.rs              Entry point, arg parsing, session load/save
  cli.rs               Clap derive structs for all 19 commands
  session.rs           Session state persistence (cookies, URL, page cache)
  http.rs              HTTP client with manual redirect following, stealth mode
  dom.rs               HTML parsing abstraction over scraper crate
  output.rs            JSON + human-readable formatting
  error.rs             Error types with exit codes
  commands/
    navigate.rs        get
    select.rs          select (CSS queries)
    text.rs            text extraction
    links.rs           link extraction
    tables.rs          table extraction
    click.rs           click (follow links)
    forms.rs           forms, fill, submit
    headers.rs         headers
    cookies.rs         cookies
    status.rs          status
    session_cmd.rs     session management
    crawl.rs           crawl (automated link tree traversal)
    search.rs          search (DuckDuckGo/Google)
    rss.rs             RSS/Atom feed parsing
    sitemap.rs         sitemap.xml discovery
    markdown.rs        HTML to markdown conversion
    pipe.rs            batch URL processing from stdin
```

### Key Design Decisions

- **Manual redirect following** — captures Set-Cookie from every hop (reqwest's auto-redirect misses intermediate cookies)
- **Session-per-directory** — `~/.clibrowser/sessions/<name>/` with cookies.json, state.json, page.html, fills.json
- **DOM abstraction** — all commands go through `dom.rs`, never call scraper directly
- **Exit codes** — agents branch on exit codes without parsing output
- **No JS engine** — deliberate tradeoff: 6MB binary vs 400MB+ with Chromium. Handles 80%+ of the web (server-rendered pages, APIs, docs, blogs, news)

## Crate Dependencies

| Crate | Purpose |
|-------|---------|
| clap 4 | CLI argument parsing (derive macros) |
| reqwest 0.12 | HTTP client (rustls-tls, gzip, brotli, deflate) |
| tokio 1 | Async runtime (current_thread) |
| scraper 0.22 | HTML parsing + CSS selectors |
| url 2 | URL resolution |
| serde / serde_json | Serialization |
| chrono | Timestamps |
| dirs | Home directory resolution |
| anyhow / thiserror | Error handling |
| rand | Jittered delays for stealth retry |

## License

MIT
