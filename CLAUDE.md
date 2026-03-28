# clibrowser

`clibrowser` is installed at `~/.local/bin/clibrowser`. It is a CLI browser for AI agents — use it to browse the web, search, extract content, and interact with forms entirely via bash commands.

## Quick reference

```bash
clibrowser --stealth search "query" --json        # web search
clibrowser --stealth search "query" --lucky        # search & navigate to first result
clibrowser --stealth get "https://url" --json      # fetch a page
clibrowser text --strip                            # extract text
clibrowser markdown --max-length 3000              # convert page to markdown
clibrowser links --absolute --json                 # extract links
clibrowser select "h1, h2" --json                  # CSS selector query
clibrowser click "a.link" --json                   # follow a link
clibrowser forms --json                            # list forms
clibrowser fill "form" "field=value"               # fill form
clibrowser submit --json                           # submit form
clibrowser cookies --json                          # show cookies
clibrowser rss "https://feed.url" -n 10 --json     # parse RSS feed
clibrowser sitemap "https://site.com" --json       # discover pages
clibrowser crawl "https://url" --depth 2 --json    # crawl link tree
echo "url1\nurl2" | clibrowser pipe --title --json # batch process URLs
```

Always use `--json` for structured output you can parse. Use `--stealth` for sites with bot detection.
