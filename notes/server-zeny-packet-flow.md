# How the server sends zeny (and why the client cannot know it at login)

Investigated 2026-11-24 on the live server (`ssh rathena`, tree `/home/phantom/rathena`,
rAthena fork, `PACKETVER 20190605`).

## Wire format

| packet | layout | var id | value |
| --- | --- | --- | --- |
| `ZC_LONGPAR_CHANGE` **0x00B1** | `<var>.W <value>.L` (8 bytes) | `SP_ZENY = 20` | `sd.status.zeny` (int32) |

* server enum: `src/map/map.hpp:536` (`SP_ZENY` = 20, `_sp` starts at `SP_SPEED` 0)
* client enum: `rodecomp/Source/Network/Packets/NoticeEnums.h:24` (`SP_ZENY = 20`)
* sender: `clif_updatestatus(sd, SP_ZENY)` → `src/map/clif.cpp:3717-3719`
  (`clif_longpar_change`)
* 64-bit alternative `ZC_LONGLONGPAR_CHANGE` **0x0ACB** exists, but zeny never uses
  it (`clif_longlongpar_change` is only used for EXP under PACKETVER >= 20170830).

## When it is sent

`clif_updatestatus(SP_ZENY)` is only called from zeny-changing paths:

* `pc_getzeny` — `src/map/pc.cpp:6447`
* `pc_payzeny` — `src/map/pc.cpp:6412`
* trade — `src/map/trade.cpp:550` / `:571`
* mail attachment claim — `src/map/mail.cpp:146`

Additionally the bank acks carry the carried zeny as a field:
`ZC_ACK_BANKING_DEPOSIT` 0x09A8 / `ZC_ACK_BANKING_WITHDRAW` 0x09AA
(`<reason>.W <money>.Q <zeny>.L`, `src/map/packets.hpp:337-351`).

## At login it is NOT sent

`clif_parse_LoadEndAck` (`src/map/clif.cpp` around the `sd->state.connect_new`
block) sends

```
SP_BASEEXP, SP_NEXTBASEEXP, SP_JOBEXP, SP_NEXTJOBEXP, SP_SKILLPOINT, clif_initialstatus()
```

and `clif_initialstatus` (`src/map/clif.cpp:4092`) sends `ZC_STATUS` plus the
attribute/HP/SP/time stats — **neither sends SP_ZENY**. Verified by grepping
every `SP_ZENY` occurrence in the server tree: only the `clif_updatestatus`
case and the pay/get paths.

## Consequence for the client

The client's zeny is unknown until the first `ZC_LONGPAR_CHANGE`, which is
exactly what the official client does too: it initialises `g_session.m_zeny`
to 0 (`rodecomp/Source/Client/Mode/GameMode.cpp:151`) and updates it in
`ReceiveLongParChange` (`GameModeNotices.cpp:3667`).

Client parity (2026-11-24): `interface/components/zeny.rs` now displays
`0 z` while the value is unknown instead of the old `— z` placeholder, so the
row matches the official client until the server pushes the first update
(any zeny gain/loss, trade, bank or mail claim).

If the real value is wanted immediately after login, the server has to push it
(e.g. one `clif_updatestatus(*sd, SP_ZENY);` next to `SP_SKILLPOINT` in
`clif_parse_LoadEndAck`). That change was prepared and then reverted on
request; it is not applied to the live server.
