
---

## Skill/effect parity release (2026-11-24)

The RO-Next payload was updated with the skill-animation and effect work
(commit `d2b3c1f6` and the preceding effect/action commits):

| item | value |
| --- | --- |
| payload version | `0.2.29` (`ro-next-files/ro-next.version`, config `clients.ro_next.version`) |
| content release | `20261124-201226` (`/home/phantom/patcher-releases/ro_next/`) |
| files in manifest | 87 (exe + overlay data + new damage-font sprites) |
| classic client | untouched |

Updated files under `archive/data/client/` (skill_effects, skill_actions,
skill_sounds, status_effects, status_visual_effects, weapons, jobs, keybinds,
paper_doll_slots, robes, shields), `archive/data/languages/*.ron` and the new
`archive/data/sprite/이팩트/` sprites; `RO-Next.exe` rebuilt from `d2b3c1f6`
with `cargo xwin --target x86_64-pc-windows-msvc`.

Publish procedure used:

```bash
rsync -a ro-next/archive/data/client/ rathena:.../ro-next-files/archive/data/client/
scp target/x86_64-pc-windows-msvc/release/ro-next.exe rathena:.../ro-next-files/RO-Next.exe
ssh rathena 'echo 0.2.29 > .../ro-next-files/ro-next.version'
ssh rathena '/home/phantom/ragnarok-web/scripts/publish-content-release.sh --client ro_next --reason "..."'
```

The script snapshots the payload, regenerates the manifest/packages, switches
`current-ro_next` atomically and smoke-tests the live endpoints (manifest 200,
download ok).
