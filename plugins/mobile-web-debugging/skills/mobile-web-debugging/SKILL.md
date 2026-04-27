---
name: mobile-web-debugging
description: Use when helping a user make a phone access a web page running on their computer through VS Code Port Forwarding / devtunnels.ms, including starting a local server, using the VS Code Ports panel, handling GitHub/Microsoft authorization, copying the forwarded URL, and managing Private/Public visibility.
metadata:
  short-description: Use VS Code dev tunnels for mobile web testing
---

# Mobile Web Debugging

Use this skill only for the **VS Code Port Forwarding / devtunnels.ms** workflow: expose a local web server on the user's computer as an HTTPS URL that a phone can open without being on the same LAN.

## When To Use

- User wants an iPhone/Android phone to open a web page running on the computer.
- User does not want Cloudflare Tunnel.
- User is using VS Code or allows using VS Code.
- User wants a generated `https://...devtunnels.ms/` link.

## Core Workflow

1. Ensure the local web service works on the computer first, usually:

```text
http://localhost:3000
```

2. Start a simple static test server if needed:

```bash
python3 -m http.server 3000 --bind 127.0.0.1
```

3. Open VS Code `Ports` view:
   - Command Palette: `Ports: Focus on Ports View`
   - Or bottom panel tab: `Ports`

4. Click `Forward a Port`.

5. Enter the local port, for example:

```text
3000
```

6. If VS Code asks to sign in or authorize GitHub/Microsoft/local tunnel port forwarding, stop and ask the user for explicit confirmation before clicking `Allow`.

7. Copy the generated forwarded address, usually like:

```text
https://xxxx-3000.region.devtunnels.ms/
```

8. Give that URL to the user for mobile Safari/Chrome.

## Visibility Rules

VS Code dev tunnels can be `Private` or `Public`.

- **Private**: safer default. Phone may need to sign in with the same GitHub/Microsoft account.
- **Public**: easier for phone access. Anyone with the URL can reach the forwarded local service while it is active.

Before changing visibility to `Public`, ask for explicit confirmation because this exposes the local service through a public URL.

Recommended confirmation wording:

```text
当前端口是 Private。改成 Public 会让任何拿到链接的人都能访问这个本地服务。你确认要改成 Public 吗？
```

## Computer Use Notes

When using the Computer Use plugin to operate VS Code:

- Call `get_app_state` before interacting with VS Code each assistant turn.
- After the user changes the app or a modal appears/disappears, re-query app state before the next click/type.
- If a VS Code authorization dialog appears, do not click `Allow` until the user explicitly confirms.
- Prefer Command Palette command `Ports: Focus on Ports View` to reach the Ports panel.
- Use the Ports table to read the final `https://...devtunnels.ms/` URL and visibility.

## Troubleshooting

- If port forwarding succeeds but phone cannot open the URL, check whether visibility is `Private` and whether the phone is signed into the required account.
- If VS Code shows no running process, the tunnel may still work if the local server is actually listening on that port; verify computer can open `http://localhost:PORT`.
- If local server fails to bind, the sandbox or another process may block the port; try a different port or request permission to run the server.
- If the phone needs camera/geolocation/service worker/OAuth callback behavior, the `devtunnels.ms` HTTPS URL is usually preferable to LAN HTTP.
- Keep the local server running while the phone uses the forwarded URL.
