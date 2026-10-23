# Pitch

Every engineer here works with five tools open and an agent next to them that can't safely touch any of them. So we either babysit the agent or do the work ourselves.

We built one terminal for developers and their agents.

Every tool is an extension: your app's pods and volumes, a quick HTTP call, image builds, and triage that tells you in seconds whether it's a product bug or a deployment issue. Agents work in the same shell, with the same tools, under the same rules. Each extension declares what it can touch. Every agent run gets its own sandbox, with no real keys and no network it didn't ask for. Every change leaves a receipt.

Watch it live. We ask the agent to fix a broken service. It triages and finds a deployment issue. It tries to restart a pod it was never granted, and the shell stops it. It tries to call a host it never declared, and the shell blocks that too. It tests its fix in the sandbox, a person approves, the check passes, and the proof is on screen. That took 60 seconds, and nothing touched production on trust alone.

It's sandboxed by the shell, not by the prompt, and it works with any model provider.

One terminal for developers and their agents: the same tools, the same rules, and proof on every change. Any team can write an extension on Monday.

## Award fit

Award names are category labels only.

| Award | Fit |
|---|---|
| Magic Wand (time-saver, manual task automation) | Triage plus a gated fix. kubectl, docker, an HTTP client, a log viewer, and an image builder in one shell. |
| Elixir of Life (morale, well-being, culture) | Engineers stop babysitting agents and stop dreading the "is it us or the deploy?" page. Teams can try new agents safely. |
| Sorcerer's Apprentice (cross-functional collaboration) | Any team (platform, QA, security, support) ships an extension. Their tools and their rules travel together. |
| Banishment (tool or process removed) | Removes tab sprawl, copy-pasted kubectl commands, and "trust me, the agent said it's fixed." |

## Notes

- All demo services, clusters, logs, and images are mock. Event rule: "No Beast Code data or code - use a mock data set if you need to."
- We have no time-saved measurement. Do not claim one until we measure it.
