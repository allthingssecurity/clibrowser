# clibrowser

**Zero-dependency CLI browser for AI agents.** Single Rust binary, 23 commands, bash-native.

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
| **OAuth support** | Browser relay | Yes | Yes | No | Yes |
| **WebMCP support** | Yes | No | No | No | No |
| **Runs locally** | Yes (no cloud/API key) | Yes | Yes | No (cloud API) | Yes |

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
# macOS Apple Silicon (direct binary download)
curl -L https://github.tools.sap/I074560/clibrowser/releases/download/v0.1.0/clibrowser-darwin-arm64 -o clibrowser
chmod +x clibrowser
mv clibrowser ~/.local/bin/

# Build from source (any platform)
git clone https://github.tools.sap/I074560/clibrowser.git
cd clibrowser
cargo build --release
cp target/release/clibrowser ~/.local/bin/
```

## IMPORTANT: Commands are sequential, NOT piped

`get` fetches and caches the page. Then `text`/`markdown`/`select`/`links`/`tables` read from that cache.

```bash
# CORRECT — run sequentially:
clibrowser --stealth get "https://example.com"
clibrowser markdown --max-length 3000

# WRONG — do NOT pipe:
clibrowser get "https://url" --json | clibrowser markdown
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

## All 23 Commands

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

### Authentication (OAuth / Google Login)

| Command | Description | Example |
|---------|-------------|---------|
| `auth <url>` | Open browser for OAuth login, capture cookies | `clibrowser auth "https://site.com/login/"` |
| `import-cookies` | Import cookies from browser DevTools | `clibrowser import-cookies "sessionid=abc" --domain site.com` |

The `auth` command opens your real browser (Safari/Chrome) for the login flow. You complete Google OAuth or any login normally, then press Enter in the terminal. clibrowser captures the session cookies — no JS engine needed.

```bash
# Login to a site that requires Google OAuth
clibrowser --session mysite auth "https://mysite.com/login/"
# Browser opens → you login → press Enter → cookies captured

# Now browse as authenticated user
clibrowser --session mysite --stealth get "https://mysite.com/dashboard/"
clibrowser --session mysite tables --json

# Or import cookies directly from Chrome DevTools
# (F12 → Application → Cookies → copy sessionid value)
clibrowser --session mysite import-cookies "sessionid=abc123; csrftoken=xyz" --domain mysite.com
```

### WebMCP (Google Chrome Agentic Web Standard)

| Command | Description | Example |
|---------|-------------|---------|
| `webmcp [url]` | Discover WebMCP tools on a page | `clibrowser webmcp "https://site.com" --json` |
| `webmcp-call <tool> key=value...` | Call a WebMCP tool by name | `clibrowser webmcp-call search_flights origin=SFO destination=JFK` |

[WebMCP](https://developer.chrome.com/blog/webmcp-epp) is Google's new standard for making websites agent-ready. Sites annotate their forms with `toolname`, `tooldescription`, and `toolparamdescription` attributes so agents can discover and invoke structured actions.

```bash
# Discover what tools a website exposes
clibrowser --stealth webmcp "https://travel-site.com" --json
# Returns: tool names, descriptions, parameters, types, required/optional

# Call a tool by name with structured parameters
clibrowser webmcp-call search_flights origin=SFO destination=JFK date=2026-04-15 class=business

# Read the result
clibrowser markdown --max-length 5000
```

WebMCP is in early preview — few sites have adopted it yet. But clibrowser is ready when they do. The regular `forms` command works on every site in the meantime.

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

```
~/.clibrowser/sessions/
├── default/           ← used when no --session flag
│   ├── state.json     ← current URL, status code, headers
│   ├── cookies.json   ← all cookies (persists across requests)
│   ├── page.html      ← cached last-fetched page
│   └── fills.json     ← pending form field values
├── agent-A/           ← clibrowser --session agent-A ...
├── agent-B/           ← clibrowser --session agent-B ...
└── research/          ← clibrowser --session research ...
```

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

### OAuth Login + Authenticated Scraping

```bash
# Login via browser relay
clibrowser --session mysite auth "https://mysite.com/login/"
# → browser opens, you login with Google, press Enter

# Browse authenticated pages
clibrowser --session mysite --stealth get "https://mysite.com/dashboard/"
clibrowser --session mysite tables --json
clibrowser --session mysite select ".premium-content" --json
```

### WebMCP Tool Discovery

```bash
# Discover structured tools on agent-ready websites
clibrowser --stealth webmcp "https://travel-site.com" --json

# Call tools by name with typed parameters
clibrowser webmcp-call search_flights origin=SFO destination=JFK date=2026-04-15
clibrowser markdown --max-length 5000
```

## Architecture

```
src/
  main.rs              Entry point, arg parsing, session load/save, SIGPIPE handling
  cli.rs               Clap derive structs for all 23 commands
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
    webmcp.rs          WebMCP tool discovery and invocation
    auth.rs            OAuth browser relay + cookie import
```

### Key Design Decisions

- **Manual redirect following** — captures Set-Cookie from every hop (reqwest's auto-redirect misses intermediate cookies)
- **Session-per-directory** — `~/.clibrowser/sessions/<name>/` with cookies.json, state.json, page.html, fills.json
- **DOM abstraction** — all commands go through `dom.rs`, never call scraper directly
- **Exit codes** — agents branch on exit codes without parsing output
- **SIGPIPE handling** — graceful exit when piped to `head` or other truncating commands
- **No JS engine** — deliberate tradeoff: 6MB binary vs 400MB+ with Chromium. Handles 80%+ of the web (server-rendered pages, APIs, docs, blogs, news)
- **Browser relay for OAuth** — opens real browser for JS-heavy login flows, captures cookies back. Zero added dependencies.
- **WebMCP support** — ready for Google's agentic web standard before most sites have adopted it

## Crate Dependencies

| Crate | Purpose |
|-------|---------|
| clap 4 | CLI argument parsing (derive macros) |
| reqwest 0.12 | HTTP client (rustls-tls, gzip, brotli, deflate) |
| tokio 1 | Async runtime (current_thread, time) |
| scraper 0.22 | HTML parsing + CSS selectors |
| url 2 | URL resolution |
| serde / serde_json | Serialization |
| chrono | Timestamps |
| dirs | Home directory resolution |
| anyhow / thiserror | Error handling |
| rand | Jittered delays for stealth retry |
| libc | SIGPIPE signal handling |

## Limitations

- **No JavaScript execution** — JS-rendered SPAs (React, Angular, Vue) show empty shells. Use `auth` command to login via real browser, then browse server-rendered pages.
- **No screenshots** — text/data extraction only, no visual output
- **No CAPTCHA solving** — stealth mode bypasses basic bot detection but not interactive CAPTCHAs
- **macOS ARM64 binary only** — other platforms build from source with `cargo build --release`

## License

MIT
