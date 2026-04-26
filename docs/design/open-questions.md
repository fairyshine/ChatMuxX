# Open Questions

Please answer these before the design docs are considered stable.

## Product Direction

1. First release channel: Telegram first, WeChat first, or both from day one?
2. Is ChatMuxX for one trusted owner only, or should it support multiple authorized users from the beginning?
3. Should ChatMuxX be a daemon plus CLI only, or should it also have a local web UI/dashboard?
4. Is the main goal to build a new implementation inspired by ccgram, or to fork/adapt ccgram heavily?

## Runtime and Language

1. Preferred implementation language/runtime: Python, TypeScript/Node.js, or something else?
2. Should WeChat support depend on OpenClaw being installed, or should ChatMuxX call the iLink HTTP API directly?
3. Should the daemon run as a normal foreground process first, or include launchd/systemd service setup in the first release?

## Session Model

1. Should one mobile conversation always map to exactly one tmux window?
2. Should multiple mobile channels be allowed to bind to the same tmux window?
3. Should ChatMuxX create new tmux windows itself, or only bind to existing windows in the first release?
4. Should session identity be based on tmux window ID, pane ID, or a ChatMuxX-generated session ID with tmux metadata?

## Agent Providers

1. Which providers are mandatory for v0.1: Codex, Claude Code, shell, Gemini, Pi?
2. For Claude Code, should ChatMuxX install/use hooks, or start with transcript/terminal polling only?
3. For Codex, should ChatMuxX parse JSONL session files, terminal output, or both?
4. Should shell provider support natural-language-to-command generation in v0.1?

## Mobile UX

1. What are must-have phone actions for v0.1: screenshot, live view, Esc, Ctrl-C, Enter, file send, voice input, status, recovery?
2. Should prompts and approval dialogs be rendered as chat buttons whenever the channel supports them?
3. Should ChatMuxX send every agent output message, or only summaries/status unless explicitly requested?
4. Should long output be split verbatim, summarized, or attached as a file?

## Security

1. How should allowed users be configured: env vars, config file, first-login pairing, or all of these?
2. Should dangerous actions such as shell commands, Ctrl-C, file reads, and file sends require extra confirmation?
3. What local file paths should be blocked from mobile delivery by default?
4. Should state files be plaintext, or should tokens/account credentials be stored through OS keychain where possible?

## WeChat Details

1. Is WeChat personal account support a hard requirement for v0.1?
2. Should WeChat support text only first, or include image/file/voice from the start?
3. How should WeChat group chats map to sessions: per group, per sender within group, or explicit bind command?
4. Is using the Tencent/OpenClaw package acceptable as a dependency, or do we need a standalone implementation?

## Documentation Structure

1. Should design docs stay high-level, or include detailed module APIs before coding starts?
2. Should we write an ADR series for each confirmed decision?
3. Should `docs/development` become the task breakdown after design stabilizes?

