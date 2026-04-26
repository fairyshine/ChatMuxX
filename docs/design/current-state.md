# Current State

## Repository Snapshot

- Root project: `ChatMuxX`.
- Main README describes the product intent: connect mobile social applications such as Telegram and WeChat to local tmux sessions running Codex, Claude Code, and other CLI applications.
- `docs/design` and `docs/development` are currently empty before this draft.
- `references/ccgram` contains a mature Telegram-to-tmux bridge implementation.
- `references/openclaw-weixin` contains a WeChat channel implementation and protocol notes for the iLink Bot API. ChatMuxX uses it as protocol reference only, not as a runtime dependency.
- There is no application source tree yet for ChatMuxX itself. The intended order is to complete design docs first, then development docs, then source code.

## Reference: ccgram

Useful patterns to learn from conceptually:

- Tmux remains the runtime source of truth.
- One Telegram forum topic maps to one tmux window.
- Session state is persisted locally and survives bot restarts.
- Provider-specific behavior is isolated behind provider modules.
- Outbound agent output is read from transcripts, hook events, or terminal capture.
- Interactive prompts are converted into mobile-friendly inline controls.
- Screenshots, live view, file sending, voice transcription, and recovery are treated as normal product flows.

Important constraints for ChatMuxX:

- ccgram is only a partial reference. ChatMuxX is a complete new implementation and should not copy ccgram's structure wholesale.
- ccgram is Telegram-first. ChatMuxX needs channel abstraction from the start because Telegram, WeChat, and later chat apps are product targets.
- ccgram is Python. ChatMuxX has not chosen a language/runtime yet.
- ccgram's topic-per-window model maps naturally to Telegram forum topics, but WeChat may need a different conversation/session mapping.

## Reference: openclaw-weixin

Useful protocol/interface facts:

- WeChat access should be implemented through Tencent's iLink HTTP API directly.
- `@tencent-weixin/openclaw-weixin` is a protocol/interface reference, not a required ChatMuxX runtime dependency.
- Login is QR-code based and stores a `bot_token` for later API calls.
- Incoming messages are fetched through long polling with a `get_updates_buf` cursor.
- Replies must preserve the inbound `context_token` to stay attached to the right WeChat conversation.
- Message items include text, image, voice, file, and video.
- Media upload/download uses encrypted CDN references and AES-128-ECB handling in the reference implementation.
- Typing indicators require `typing_ticket` from `getconfig`.

Open questions for ChatMuxX:

- Whether WeChat media support is part of the first usable release.
- How WeChat conversations map to tmux windows when there is no Telegram-style forum topic.

## Current Design Baseline

The current baseline is a greenfield implementation guided by the two references:

- Use ccgram only as experience-level inspiration for mobile chat to tmux-agent control.
- Use openclaw-weixin only for the exposed WeChat/iLink interfaces and protocol details.
- Design ChatMuxX as a multi-channel, multi-provider bridge rather than a Telegram-only fork.
- Start with Telegram and WeChat as the first two channel targets, while keeping room for more chat applications later.
